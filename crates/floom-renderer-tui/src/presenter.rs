//! Presentation: scenes onto a character grid, and bytes onto a terminal.
//!
//! [`Presenter`] is the seam floom's app runner uses to hand a finished scene
//! to "wherever pixels go". Here they go into [`TuiGrid`] cells and then out as
//! ANSI text — to whichever writer the presenter was built with.
//!
//! The writer is generic rather than hard-coded to stdout for the same reason
//! [`SoftwarePresenter`] keeps its pixels addressable: the interesting failures
//! hide in output nobody can read back. With a `Vec<u8>` sink a test can assert
//! on the exact bytes a frame produced, including the escapes.
//!
//! Framing is a property of the *caller*: nothing here clears the screen or
//! homes the cursor, because only the caller knows whether it owns the whole
//! screen or one pane of it.
//!
//! [`SoftwarePresenter`]: floom_renderer_software::SoftwarePresenter

use std::io::Write;

use floom_renderer::{AttachedSurface, FrameInfo, Presenter, RendererCapabilities};
use floom_scene::{ResourceRegistry, Scene};
use floom_types::Size;

use crate::{DiffStats, PaintStats, TuiGrid, diff_encode, encode_with_stats, paint_scene};

/// How many characters the terminal has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub cols: usize,
    pub rows: usize,
}

impl TerminalSize {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self { cols, rows }
    }
}

/// A [`Presenter`] that writes scenes into a terminal as ANSI text.
pub struct TuiPresenter<W: Write> {
    sink: W,
    size: TerminalSize,
    /// Logical extent of the surface; the scene is authored in these units and
    /// the grid maps them onto `size` cells.
    logical: Size,
    attached: bool,
    frames: u64,
    stats: PaintStats,
    last: Option<TuiGrid>,
}

impl<W: Write> TuiPresenter<W> {
    pub fn new(size: TerminalSize, sink: W) -> Self {
        Self {
            sink,
            size,
            logical: Size::new(size.cols as f64, size.rows as f64),
            attached: false,
            frames: 0,
            stats: PaintStats::default(),
            last: None,
        }
    }

    /// The grid the most recent [`present`](Presenter::present) produced.
    ///
    /// `None` before the first frame. Kept for the same reason pixel buffers
    /// are: "nothing appeared" needs to be distinguishable from "the grid is
    /// correct and the writer dropped it".
    pub fn last_grid(&self) -> Option<&TuiGrid> {
        self.last.as_ref()
    }

    /// [`PaintStats`] for the most recent frame.
    pub fn stats(&self) -> PaintStats {
        self.stats
    }

    /// The writer this presenter writes to.
    pub fn sink(&mut self) -> &mut W {
        &mut self.sink
    }
}

impl<W: Write> Presenter for TuiPresenter<W> {
    fn attach(&mut self, surface: AttachedSurface) -> bool {
        // No OS handles are required: a character grid needs nothing from the
        // platform, so unlike a GPU presenter there is nothing here that can be
        // refused.
        if surface.size.width > 0.0 && surface.size.height > 0.0 {
            self.logical = surface.size;
        }
        self.attached = true;
        true
    }

    fn resize(&mut self, size: Size, _scale: f64) {
        if size.width > 0.0 && size.height > 0.0 {
            self.logical = size;
            let cols = size.width as usize;
            let rows = size.height as usize;
            if cols != self.size.cols || rows != self.size.rows {
                // The grid is allocated to the terminal's cell count; a resize
                // changes that count, so the stored grid — which is also the
                // diff baseline — must follow, or the next patch indexes a
                // differently shaped buffer. Forcing a full repaint is the
                // honest answer to a reshaped screen: there is no cheap
                // incremental one.
                self.size = TerminalSize::new(cols, rows);
                self.last = None;
            }
        }
    }

    fn present(&mut self, scene: &Scene, resources: &ResourceRegistry) {
        // The runner calls `present`; route it through the incremental path so a
        // live terminal repaints in patches, not in full every frame.
        self.present_internal(scene, resources);
    }

    fn detach(&mut self) {
        self.attached = false;
    }

    fn frames_presented(&self) -> u64 {
        self.frames
    }

    /// A character grid has no soft edges and no shader runtime.
    ///
    /// A rounded corner, a drop shadow, a filter scope, a backdrop filter and
    /// a custom shader are all requests a terminal cannot honour. The grid
    /// counts each of them (`PaintStats::skipped`), and this says so before
    /// the frame is drawn — a host can then decide not to ask for a shadow it
    /// was never going to get.
    fn capabilities(&self) -> RendererCapabilities {
        RendererCapabilities::NONE
    }
}

impl<W: Write> TuiPresenter<W> {
    /// The shared frame path for [`present`](Presenter::present) and
    /// [`present_diff`]: build the grid, and — on the first frame — clear the
    /// screen and home the cursor, then on every later frame emit only the
    /// cells that changed. A live terminal cannot repaint in full at animation
    /// speed, and the runner calls `present`, so the incremental behaviour
    /// lives here and `present_diff` is just the same method with its stats
    /// surfaced.
    fn present_internal(&mut self, scene: &Scene, resources: &ResourceRegistry) -> DiffStats {
        let mut grid = TuiGrid::new(self.size.cols, self.size.rows, self.logical);
        paint_scene(&mut grid, scene, resources);
        self.stats = grid.stats();
        let full = self.last.is_none();
        let (bytes, changed, escapes) = if full {
            let (body, stats) = encode_with_stats(&grid);
            // Clear + home, then the full screen. Patching needs a baseline.
            let mut s = String::from("\u{1b}[2J\u{1b}[H");
            s.push_str(&body);
            (s, grid.cells_with_content(), stats.escapes + 2)
        } else {
            let old = self.last.as_ref().unwrap();
            let (patch, changed, escapes) = diff_encode(&grid, old);
            (patch, changed, escapes)
        };
        self.frames += 1;
        self.last = Some(grid);
        // A writer that refuses bytes must not take the app down with it: the
        // frame still counted, and `frames_presented` stays honest about it by
        // counting the attempt. This is the same "never fatal" rule the other
        // presenters follow for a refused surface.
        let _ = self.sink.write_all(bytes.as_bytes());
        let _ = self.sink.flush();
        DiffStats {
            changed_cells: changed,
            escapes,
            full_repaint: full,
        }
    }

    /// Like [`present`](Presenter::present) but incremental, surfacing
    /// [`DiffStats`] about the patch. The runner calls `present`, which already
    /// goes through this path, so this is the explicit spelling for callers
    /// that want to assert on how much of the screen a frame touched.
    pub fn present_diff(&mut self, scene: &Scene, resources: &ResourceRegistry) -> DiffStats {
        self.present_internal(scene, resources)
    }
}

/// Utility: the `FrameInfo` describing a logical-space frame, for callers that
/// want one without going through a platform.
pub fn frame_info(logical: Size, frame_number: u64) -> FrameInfo {
    FrameInfo {
        target: floom_renderer::RenderTarget::Software {
            width: logical.width.ceil() as u32,
            height: logical.height.ceil() as u32,
        },
        logical_size: logical,
        scale_factor: 1.0,
        frame_number,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::rect;
    use floom_scene::{Resource, ResourceKind, SceneNode};
    use floom_types::{Color, Point, Rect};

    fn attached(size: Size) -> AttachedSurface {
        AttachedSurface::new(
            floom_types::WindowId::new(0, 1),
            floom_types::SurfaceId::new(0, 1),
            size,
            1.0,
            None,
        )
    }

    fn scene_with_label(bounds: Rect, color: Color) -> (Scene, ResourceRegistry) {
        let mut scene = Scene::new();
        let registry = ResourceRegistry::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds,
            fill: floom_types::Brush::from(color),
            radius: 0.0,
            shadow: None,
        });
        scene.finish_update();
        (scene, registry)
    }

    #[test]
    fn a_frame_reaches_the_writer() {
        let (scene, registry) = scene_with_label(rect(0.0, 0.0, 80.0, 24.0), Color::rgb(1, 2, 3));
        let mut presenter = TuiPresenter::new(TerminalSize::new(8, 2), Vec::new());
        assert!(presenter.attach(attached(Size::new(80.0, 24.0))));
        presenter.present(&scene, &registry);

        let out = String::from_utf8(presenter.sink().clone()).unwrap();
        assert_eq!(presenter.frames_presented(), 1);
        assert!(
            out.contains("\u{1b}[48;2;1;2;3m"),
            "the panel colour appears: {out:?}"
        );
        assert_eq!(
            presenter.last_grid().map(|g| (g.cols(), g.rows())),
            Some((8, 2))
        );
    }

    #[test]
    fn presenting_twice_counts_two_frames() {
        let (scene, registry) = scene_with_label(rect(0.0, 0.0, 4.0, 2.0), Color::BLACK);
        let mut presenter = TuiPresenter::new(TerminalSize::new(4, 2), Vec::new());
        presenter.attach(attached(Size::new(4.0, 2.0)));
        presenter.present(&scene, &registry);
        presenter.present(&scene, &registry);
        assert_eq!(presenter.frames_presented(), 2);
    }

    #[test]
    fn detach_stops_nothing_but_is_recorded() {
        let mut presenter = TuiPresenter::new(TerminalSize::new(2, 2), Vec::new());
        presenter.attach(attached(Size::new(2.0, 2.0)));
        presenter.detach();
        // Presenting after detach still writes: a detached terminal-surface is
        // not a dangling pointer the way a detached swap-chain is.
        let (scene, registry) = scene_with_label(rect(0.0, 0.0, 2.0, 2.0), Color::BLACK);
        presenter.present(&scene, &registry);
        assert_eq!(presenter.frames_presented(), 1);
    }

    #[test]
    fn a_resize_rebuilds_the_grid_at_the_new_terminal_size() {
        let mut presenter = TuiPresenter::new(TerminalSize::new(8, 2), Vec::new());
        presenter.attach(attached(Size::new(8.0, 2.0)));
        // A real terminal resize changes the cell count, not just the logical
        // mapping: the grid must follow the new width (here doubled) so a rect
        // spanning the right half lands in the right half of a 16-column grid,
        // not squashed into 8 columns.
        presenter.resize(Size::new(16.0, 2.0), 1.0);

        let mut scene = Scene::new();
        let registry = ResourceRegistry::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(8.0, 0.0), Size::new(8.0, 2.0)),
            fill: floom_types::Brush::from(Color::rgb(9, 9, 9)),
            radius: 0.0,
            shadow: None,
        });
        scene.finish_update();
        presenter.present(&scene, &registry);

        let grid = presenter.last_grid().unwrap();
        // The grid now tracks the new terminal width.
        assert_eq!(grid.cols(), 16, "the grid tracks the new terminal width");
        // Columns 0..8 stay unpainted: the rect starts at the midpoint of a
        // 16-column grid.
        assert_eq!(grid.cell(0, 0).unwrap().bg, None);
        assert_eq!(grid.cell(7, 0).unwrap().bg, None);
        assert_eq!(grid.cell(8, 0).unwrap().bg, Some(Color::rgb(9, 9, 9)));
    }

    #[test]
    fn a_text_resource_without_a_string_is_not_fatal() {
        // A caller that shaped runs but never handed over the string (every
        // pre-existing fixture) gets a skipped node, not a panic.
        let mut registry = ResourceRegistry::new();
        let glyphs = registry.insert(Resource {
            kind: ResourceKind::glyph_runs(Vec::new()),
        });
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Text {
            origin: Point::new(0.0, 0.0),
            size: Size::new(1.0, 1.0),
            glyphs,
            color: Color::WHITE,
        });
        scene.finish_update();

        let mut presenter = TuiPresenter::new(TerminalSize::new(4, 2), Vec::new());
        presenter.attach(attached(Size::new(4.0, 2.0)));
        presenter.present(&scene, &registry);
        assert_eq!(presenter.stats().text_runs, 1);
        assert_eq!(presenter.frames_presented(), 1);
    }

    #[test]
    fn first_diff_frame_is_a_full_repaint() {
        let (scene, registry) = scene_with_label(rect(0.0, 0.0, 80.0, 24.0), Color::rgb(1, 2, 3));
        let mut presenter = TuiPresenter::new(TerminalSize::new(8, 2), Vec::new());
        presenter.attach(attached(Size::new(80.0, 24.0)));
        let stats = presenter.present_diff(&scene, &registry);
        assert!(stats.full_repaint);
        let out = String::from_utf8(presenter.sink().clone()).unwrap();
        assert!(
            out.starts_with("\u{1b}[2J\u{1b}[H"),
            "the first diff frame clears and homes: {out:?}"
        );
        assert_eq!(presenter.frames_presented(), 1);
    }

    #[test]
    fn an_unchanged_second_frame_patches_nothing() {
        let (scene, registry) = scene_with_label(rect(0.0, 0.0, 8.0, 2.0), Color::BLACK);
        let mut presenter = TuiPresenter::new(TerminalSize::new(8, 2), Vec::new());
        presenter.attach(attached(Size::new(8.0, 2.0)));
        presenter.present_diff(&scene, &registry); // full
        let before = presenter.sink().clone().len();
        let stats = presenter.present_diff(&scene, &registry); // identical
        assert!(!stats.full_repaint);
        assert_eq!(
            stats.changed_cells, 0,
            "nothing moved between identical frames"
        );
        let patch_len = presenter.sink().clone().len() - before;
        assert!(
            patch_len < before,
            "a no-op patch is a handful of bytes, not a screen: {patch_len} vs {before}"
        );
    }
}
