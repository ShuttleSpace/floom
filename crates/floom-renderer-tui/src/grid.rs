//! The character grid: the scene surface a terminal actually has.
//!
//! Logical coordinates map to cells by `size / cols` and `size / rows`, so a
//! scene laid out for a 200×100 logical window over an 80×25 grid gives cells
//! 2.5 logical units wide. Nothing here rounds up to "the nearest thing it
//! could have meant" — a glyph either lands inside the grid or it is dropped,
//! and [`PaintStats`] says how many went.

use floom_scene::{ResourceKind, ResourceRegistry, Scene, SceneNode};
use floom_types::{ClipRegion, Color, Rect, Size};

/// One character cell.
///
/// `fg`/`bg` are `None` for "leave the terminal's own colours alone", which is
/// what an untouched cell wants: emitting explicit colours everywhere would
/// force every frame to repaint the whole screen inescapably.
/// `Default` is spelled out rather than derived: `char`'s own default is
/// `'\0'`, which would put a NUL in every untouched cell — invisible in a
/// screenshot and quietly corrupt for anything that reads the bytes back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: Option<Color>,
    pub bg: Option<Color>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: None,
            bg: None,
        }
    }
}

/// What a paint pass did, so a caller can tell "nothing to draw" apart from
/// "everything was skipped".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PaintStats {
    pub rects: usize,
    pub text_runs: usize,
    /// Nodes this backend cannot express — paths, images, filters, shaders,
    /// group transforms, non-solid fills, and the rounded corners and drop
    /// shadows of a rect. See the crate docs for why these are dropped rather
    /// than approximated.
    ///
    /// A rect that has both a radius and a shadow counts **twice**: each is a
    /// separate thing that was asked for and cannot be had, and the number is
    /// meant to be read as "how many requests were dropped", not "how many
    /// nodes were dropped".
    pub skipped: usize,
    /// Glyphs whose cell fell outside the grid.
    pub clipped_glyphs: usize,
}

impl PaintStats {
    /// The frame's degradations, in the vocabulary every backend shares.
    ///
    /// The honest answer here is that the terminal cannot attribute its one
    /// counter: a character grid drops a rounded corner, a shadow, a path, a
    /// filter and a shader, and it counts them all in the same `skipped`.
    /// Splitting the number into four fields would be a guess dressed as a
    /// measurement, so the report carries the total as `shadows_skipped` and
    /// says so — a caller comparing backends sees "the terminal dropped
    /// things" rather than a precise-looking number nobody counted.
    ///
    /// That is the one place the unified view is less informative than the
    /// backend's own stats, and it is the honest direction: the report
    /// normalises what was measured, it does not invent what was not.
    pub fn degradation(&self) -> floom_renderer::DegradationReport {
        floom_renderer::DegradationReport {
            shadows_skipped: self.skipped as u64,
            filters_skipped: 0,
            backdrop_filters_skipped: 0,
            shaders_skipped: 0,
        }
    }

    fn skipped_one(&mut self) {
        self.skipped += 1;
    }
}

/// A `cols × rows` grid of [`Cell`]s covering a logical extent.
#[derive(Debug, Clone)]
pub struct TuiGrid {
    cols: usize,
    rows: usize,
    /// Logical size of one cell.
    cell_size: Size,
    cells: Vec<Cell>,
    stats: PaintStats,
}

impl TuiGrid {
    pub fn new(cols: usize, rows: usize, logical_size: Size) -> Self {
        // A zero-width/height cell would make every column computation divide
        // by zero; a one-unit cell at least keeps the mapping total.
        let cell_size = Size::new(
            if cols > 0 {
                (logical_size.width / cols as f64).max(f64::EPSILON)
            } else {
                1.0
            },
            if rows > 0 {
                (logical_size.height / rows as f64).max(f64::EPSILON)
            } else {
                1.0
            },
        );
        Self {
            cols,
            rows,
            cell_size,
            cells: vec![Cell::default(); cols * rows],
            stats: PaintStats::default(),
        }
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn stats(&self) -> PaintStats {
        self.stats
    }

    pub fn cell(&self, col: usize, row: usize) -> Option<&Cell> {
        self.cells.get(row * self.cols + col)
    }

    pub fn cell_mut(&mut self, col: usize, row: usize) -> Option<&mut Cell> {
        let cols = self.cols;
        self.cells.get_mut(row * cols + col)
    }

    /// Count cells that carry any visible content — a non-space character or an
    /// explicit colour. Used by the diff presenter to report how much of the
    /// screen a full repaint actually touched.
    pub fn cells_with_content(&self) -> usize {
        self.cells
            .iter()
            .filter(|c| c.ch != ' ' || c.fg.is_some() || c.bg.is_some())
            .count()
    }

    /// The column a logical x falls in, if any.
    pub fn col_of(&self, x: f64) -> Option<usize> {
        if x < 0.0 {
            return None;
        }
        let col = (x / self.cell_size.width) as usize;
        (col < self.cols).then_some(col)
    }

    /// The row a logical y falls in, if any.
    pub fn row_of(&self, y: f64) -> Option<usize> {
        if y < 0.0 {
            return None;
        }
        let row = (y / self.cell_size.height) as usize;
        (row < self.rows).then_some(row)
    }

    /// Fill `rect`'s cells with `color`, keeping whatever characters are
    /// already there.
    ///
    /// This is background paint, not a draw: a label laid out on top of a panel
    /// must keep its glyphs, so only `bg` changes. A partially transparent fill
    /// composites over what was underneath rather than replacing it — the
    /// alternative (rounding alpha up to opaque) would make every translucent
    /// panel look solid.
    pub fn fill_background(&mut self, rect: Rect, color: Color) {
        if color.a == 0 {
            return;
        }
        let x0 = (rect.x0.max(0.0) / self.cell_size.width) as usize;
        let y0 = (rect.y0.max(0.0) / self.cell_size.height) as usize;
        let x1 = ((rect.x1 / self.cell_size.width).ceil() as isize).max(0) as usize;
        let y1 = ((rect.y1 / self.cell_size.height).ceil() as isize).max(0) as usize;
        for row in y0..y1.min(self.rows) {
            for col in x0..x1.min(self.cols) {
                let cell = &mut self.cells[row * self.cols + col];
                let base = cell.bg.unwrap_or(Color::TRANSPARENT);
                cell.bg = Some(composite(color, base));
            }
        }
    }

    /// Write one character with a foreground colour, leaving the background.
    pub fn put(&mut self, col: usize, row: usize, ch: char, fg: Color) -> bool {
        let Some(cell) = self.cell_mut(col, row) else {
            return false;
        };
        cell.ch = ch;
        cell.fg = Some(composite(fg, cell.fg.unwrap_or(Color::TRANSPARENT)));
        true
    }
}

/// Straight-alpha `over`, matching how the scene composes elsewhere.
fn composite(src: Color, dst: Color) -> Color {
    if src.a == u8::MAX {
        return src;
    }
    let a = f64::from(src.a) / 255.0;
    let inv = 1.0 - a;
    let d = f64::from(dst.a) / 255.0;
    let out_a = a + d * inv;
    if out_a <= 0.0 {
        return Color::TRANSPARENT;
    }
    let blend = |s: u8, over: u8| -> u8 {
        let src_part = f64::from(s) * a;
        let dst_part = f64::from(over) * d * inv;
        ((src_part + dst_part) / out_a).round() as u8
    };
    Color::rgba(
        blend(src.r, dst.r),
        blend(src.g, dst.g),
        blend(src.b, dst.b),
        (out_a * 255.0).round() as u8,
    )
}

fn solid_color(fill: &floom_types::Brush) -> Option<Color> {
    match fill {
        floom_types::Brush::Solid(color) => Some(*color),
        _ => None,
    }
}

/// Paint `scene` into `grid`.
///
/// Nodes arrive as one ordered stream, exactly as the host painted them, so
/// later nodes overwrite earlier ones — the same painter's-algorithm contract
/// every other floom backend honours. Clips are tracked as a stack of rects
/// because a pushed region applies to everything drawn after the marker.
pub fn paint_scene(grid: &mut TuiGrid, scene: &Scene, resources: &ResourceRegistry) {
    let mut clips: Vec<Rect> = Vec::new();
    for (_id, node) in scene.iter() {
        match node {
            SceneNode::Rect {
                bounds,
                fill,
                radius,
                shadow,
            } => {
                // A character grid has no soft edges and no blur, so a
                // rounded corner and a drop shadow are both things it cannot
                // express. They are counted, because "cannot express" is not
                // the same as "was not asked for": a card with a shadow that
                // silently loses it looks like a card with no shadow, and
                // nobody can tell the difference by looking.
                if *radius > 0.0 {
                    grid.stats.skipped_one();
                }
                if shadow.is_some() {
                    grid.stats.skipped_one();
                }
                let Some(color) = solid_color(fill) else {
                    grid.stats.skipped_one();
                    continue;
                };
                let bounds = apply_clips(*bounds, &clips);
                grid.fill_background(bounds, color);
                grid.stats.rects += 1;
            }
            SceneNode::Text {
                origin,
                glyphs,
                color,
                ..
            } => {
                let Some(ResourceKind::GlyphRun { runs, text }) =
                    resources.get(*glyphs).map(|r| &r.kind)
                else {
                    grid.stats.skipped_one();
                    continue;
                };
                // Glyph runs split the string at font/script boundaries, and a
                // `GlyphRun` carries no byte range, so the run's share is taken
                // to be the next `glyphs.len()` characters. For the common
                // single-run case that is exact; for mixed-script text it can
                // slide — a known limitation of putting text on a grid built
                // from glyph positions.
                let mut cursor = 0usize;
                for run in runs {
                    let run_len = run.glyphs.len();
                    for (glyph, ch) in run
                        .glyphs
                        .iter()
                        .zip(text.chars().skip(cursor).take(run_len))
                    {
                        let x = origin.x + run.origin.x + f64::from(glyph.x);
                        let y = origin.y + run.origin.y + f64::from(glyph.y);
                        if !in_clips(x, y, &clips) {
                            grid.stats.clipped_glyphs += 1;
                            continue;
                        }
                        match (grid.col_of(x), grid.row_of(y)) {
                            (Some(col), Some(row)) => {
                                grid.put(col, row, ch, *color);
                            }
                            _ => grid.stats.clipped_glyphs += 1,
                        }
                    }
                    cursor += run_len;
                }
                grid.stats.text_runs += 1;
            }
            SceneNode::ClipPush { region } => {
                let bounds = match region {
                    ClipRegion::Rect(rect) => *rect,
                    ClipRegion::RoundedRect { bounds, .. } => *bounds,
                    // A path clip has no cheap bound here; take its declared
                    // extent and clip conservatively (a glyph inside the bounds
                    // but outside the path will still be drawn). Skipping would
                    // drop the whole subtree instead.
                    ClipRegion::Path { bounds, .. } => *bounds,
                };
                clips.push(bounds);
            }
            SceneNode::ClipPop => {
                clips.pop();
            }
            _ => grid.stats.skipped_one(),
        }
    }
}

/// Rect from two corners — `floom_types::Rect` itself only offers origin+size,
/// and the four literal forms here are easier to read as edges.
pub(crate) fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
    Rect { x0, y0, x1, y1 }
}

fn apply_clips(mut bounds: Rect, clips: &[Rect]) -> Rect {
    for clip in clips {
        bounds = intersect(bounds, *clip);
    }
    bounds
}

fn intersect(a: Rect, b: Rect) -> Rect {
    rect(
        a.x0.max(b.x0),
        a.y0.max(b.y0),
        a.x1.min(b.x1),
        a.y1.min(b.y1),
    )
}

fn in_clips(x: f64, y: f64, clips: &[Rect]) -> bool {
    clips
        .iter()
        .all(|c| x >= c.x0 && x < c.x1 && y >= c.y0 && y < c.y1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use floom_scene::{Resource, SceneNode};
    use floom_text::{FontWeight, TextAlign, TextLayoutOptions, TextShaper};
    use floom_types::Point;

    fn default_fill_cell() -> Cell {
        Cell::default()
    }

    fn grid(cols: usize, rows: usize, w: f64, h: f64) -> TuiGrid {
        TuiGrid::new(cols, rows, Size::new(w, h))
    }

    #[test]
    fn an_empty_grid_is_spaces_not_nuls() {
        let g = grid(4, 2, 40.0, 20.0);
        for row in 0..g.rows() {
            for col in 0..g.cols() {
                assert_eq!(g.cell(col, row), Some(&default_fill_cell()));
                assert_eq!(
                    g.cell(col, row).unwrap().ch,
                    ' ',
                    "an untouched cell is a space"
                );
            }
        }
    }

    /// A rounded corner and a drop shadow are things a character grid cannot
    /// express, and saying so is the point.
    ///
    /// The rule the whole renderer layer follows is that a degradation is
    /// recorded, never silent. A card with a shadow that quietly loses it
    /// looks exactly like a card with no shadow, and nobody can tell the
    /// difference by looking.
    #[test]
    fn a_rounded_corner_and_a_shadow_are_counted_as_dropped() {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: floom_types::Rect {
                x0: 0.0,
                y0: 0.0,
                x1: 20.0,
                y1: 10.0,
            },
            fill: floom_types::Brush::from(floom_types::Color::rgb(1, 2, 3)),
            radius: 4.0,
            shadow: Some(floom_types::Shadow {
                color: floom_types::Color::rgba(0, 0, 0, 128),
                offset_x: 0.0,
                offset_y: 2.0,
                blur: 4.0,
                spread: 0.0,
            }),
        });
        scene.finish_update();

        let mut g = grid(4, 2, 40.0, 20.0);
        paint_scene(&mut g, &scene, &ResourceRegistry::default());

        assert_eq!(
            g.stats().skipped,
            2,
            "the radius and the shadow are two separate requests that cannot \
             be had, and each is counted"
        );
        assert_eq!(
            g.stats().rects,
            1,
            "and the rect itself was still drawn: a degraded request is not a \
             dropped node"
        );
    }

    /// A plain rect is not a degradation.
    ///
    /// The other half of the rule: a counter that always reads zero is as
    /// useless as one that is never read.
    #[test]
    fn a_plain_rect_is_not_a_degradation() {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: floom_types::Rect {
                x0: 0.0,
                y0: 0.0,
                x1: 20.0,
                y1: 10.0,
            },
            fill: floom_types::Brush::from(floom_types::Color::rgb(1, 2, 3)),
            radius: 0.0,
            shadow: None,
        });
        scene.finish_update();

        let mut g = grid(4, 2, 40.0, 20.0);
        paint_scene(&mut g, &scene, &ResourceRegistry::default());

        assert_eq!(g.stats().skipped, 0);
        assert_eq!(g.stats().rects, 1);
    }

    #[test]
    fn columns_and_rows_follow_the_cell_size() {
        // 10 logical units per column, 10 per row.
        let g = grid(4, 4, 40.0, 40.0);
        assert_eq!(g.col_of(0.0), Some(0));
        assert_eq!(g.col_of(9.99), Some(0));
        assert_eq!(g.col_of(10.0), Some(1));
        assert_eq!(g.col_of(39.9), Some(3));
        assert_eq!(
            g.col_of(40.0),
            None,
            "past the logical width there is no cell"
        );
        assert_eq!(g.col_of(-1.0), None);
        assert_eq!(g.row_of(25.0), Some(2));
        assert_eq!(g.row_of(-0.5), None);
    }

    #[test]
    fn a_rect_fills_only_the_cells_it_covers() {
        let mut g = grid(4, 4, 40.0, 40.0);
        let red = Color::rgb(200, 0, 0);
        // x 10..30 and y 20..30 over 10-unit cells: columns 1..2, row 2 only.
        g.fill_background(
            Rect::from_origin_size(Point::new(10.0, 20.0), Size::new(20.0, 10.0)),
            red,
        );
        for col in 1..3 {
            assert_eq!(g.cell(col, 2).unwrap().bg, Some(red), "cell {col},2");
        }
        assert_eq!(g.cell(0, 2).unwrap().bg, None, "left of the rect");
        assert_eq!(g.cell(3, 2).unwrap().bg, None, "right of the rect");
        assert_eq!(g.cell(1, 1).unwrap().bg, None, "above the rect");
        assert_eq!(g.cell(1, 3).unwrap().bg, None, "below the rect");
    }

    #[test]
    fn background_paint_keeps_existing_characters() {
        let mut g = grid(2, 1, 20.0, 10.0);
        g.put(0, 0, 'A', Color::BLACK);
        g.fill_background(rect(0.0, 0.0, 20.0, 10.0), Color::rgb(0, 0, 200));
        assert_eq!(g.cell(0, 0).unwrap().ch, 'A', "fill must not erase glyphs");
        assert_eq!(g.cell(0, 0).unwrap().bg, Some(Color::rgb(0, 0, 200)));
    }

    #[test]
    fn a_transparent_rect_composites_over_what_is_underneath() {
        let mut g = grid(1, 1, 10.0, 10.0);
        g.fill_background(rect(0.0, 0.0, 10.0, 10.0), Color::rgb(0, 0, 255));
        // Half-opaque white over opaque blue.
        g.fill_background(rect(0.0, 0.0, 10.0, 10.0), Color::rgba(255, 255, 255, 128));
        let bg = g.cell(0, 0).unwrap().bg.unwrap();
        assert_eq!(bg.a, 255, "result stays opaque because the base was");
        assert!(
            bg.r > 110 && bg.r < 145,
            "r is half way to white, got {bg:?}"
        );
        assert!(bg.b > 110, "b keeps most of the blue, got {bg:?}");
    }

    #[test]
    fn text_lands_in_the_cell_its_glyph_occupies() {
        let mut scene = Scene::new();
        let mut registry = ResourceRegistry::new();
        let mut shaper = TextShaper::new();
        let options = TextLayoutOptions {
            max_width: f64::INFINITY,
            font_size: 10.0,
            align: TextAlign::Start,
            weight: FontWeight::Normal,
        };
        let laid = shaper.shape("AB", &options);
        assert_eq!(
            laid.runs.len(),
            1,
            "the fixture assumes one run for two ASCII letters"
        );
        let glyphs = registry.insert(Resource {
            kind: ResourceKind::glyph_runs_with_text(laid.runs.clone(), "AB"),
        });
        scene.begin_update();
        let _ = scene.insert(SceneNode::Text {
            origin: Point::new(0.0, 0.0),
            size: laid.size,
            glyphs,
            color: Color::WHITE,
        });
        scene.finish_update();

        // One logical unit per cell, and tall enough to hold the baseline: the
        // run origin's y is the *baseline* (8.0 for this 10px font), so a grid
        // shorter than baseline+1 row clips every glyph.
        let mut g = grid(40, 12, 40.0, 12.0);
        paint_scene(&mut g, &scene, &registry);
        assert_eq!(g.stats().text_runs, 1);
        assert_eq!(g.stats().clipped_glyphs, 0, "every glyph has a cell");

        let mut written: Vec<(usize, usize, char)> = Vec::new();
        for row in 0..g.rows() {
            for col in 0..g.cols() {
                if let Some(cell) = g.cell(col, row) {
                    if cell.ch != ' ' {
                        written.push((col, row, cell.ch));
                    }
                }
            }
        }
        assert_eq!(written.len(), 2, "both glyphs were painted: {written:?}");
        assert_eq!(written[0].2, 'A');
        assert_eq!(written[1].2, 'B');
        assert_eq!(written[0].0, 0, "'A' starts at the text origin");
        assert!(
            written[1].0 > written[0].0,
            "'B' is to the right of 'A': {written:?}"
        );
    }

    #[test]
    fn supported_and_unsupported_nodes_are_counted() {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: rect(0.0, 0.0, 10.0, 10.0),
            fill: floom_types::Brush::from(Color::BLACK),
            radius: 0.0,
            shadow: None,
        });
        let _ = scene.insert(SceneNode::GroupPop);
        scene.finish_update();

        let mut g = grid(4, 4, 40.0, 40.0);
        let registry = ResourceRegistry::new();
        paint_scene(&mut g, &scene, &registry);
        assert_eq!(g.stats().rects, 1);
        assert_eq!(g.stats().skipped, 1, "the group marker has no cell meaning");
    }
}
