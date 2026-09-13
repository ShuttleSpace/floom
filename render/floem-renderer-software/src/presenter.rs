//! Software presentation: draw scenes into the CPU buffer and hand it back.
//!
//! The headless counterpart to a GPU presenter. It accepts *any* surface,
//! including the ones with no native handles, which is what makes it useful in
//! two places:
//!
//! * tests and CI, where there is no window and no GPU, and
//! * a genuine fallback, when the GPU device is unavailable or refuses the
//!   surface — the app still paints, just on the CPU.
//!
//! Because there is no OS surface to blit into, [`present`](Presenter::present)
//! leaves the pixels in [`SoftwarePresenter::pixels`] for the caller to
//! inspect or copy. That is the whole point: an offscreen frame the harness can
//! assert on.

use floem_renderer::{AttachedSurface, FrameInfo, Presenter, Renderer, SurfaceHandles};
use floem_scene::{ResourceRegistry, Scene};
use floem_types::Size;

use crate::SoftwareRenderer;

/// [`Presenter`] that renders into an RGBA8 buffer.
pub struct SoftwarePresenter {
    renderer: SoftwareRenderer,
    width: u32,
    height: u32,
    scale: f64,
    attached: bool,
    frames: u64,
}

impl Default for SoftwarePresenter {
    fn default() -> Self {
        Self::new()
    }
}

impl SoftwarePresenter {
    /// A presenter with no surface yet; `attach` sizes the buffer.
    pub fn new() -> Self {
        Self {
            renderer: SoftwareRenderer::new(1, 1),
            width: 1,
            height: 1,
            scale: 1.0,
            attached: false,
            frames: 0,
        }
    }

    /// Buffer width in physical pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Buffer height in physical pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pixels(&self) -> &[u8] {
        self.renderer.pixels()
    }

    pub fn pixel_rgba(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        self.renderer.pixel_rgba(x, y)
    }

    /// Whether a surface is currently attached.
    pub fn is_attached(&self) -> bool {
        self.attached
    }

    pub fn renderer(&self) -> &SoftwareRenderer {
        &self.renderer
    }

    fn set_extent(&mut self, size: Size, scale: f64) {
        self.scale = if scale > 0.0 { scale } else { 1.0 };
        let width = (size.width.max(1.0) * self.scale).ceil() as u32;
        let height = (size.height.max(1.0) * self.scale).ceil() as u32;
        if width != self.width || height != self.height {
            self.width = width;
            self.height = height;
            self.renderer = SoftwareRenderer::new(width, height);
        }
    }
}

impl Presenter for SoftwarePresenter {
    /// Always succeeds: a CPU buffer needs no native handles. The `handles`
    /// field is deliberately ignored rather than rejected, so the same
    /// presenter serves both a real window and a headless harness.
    fn attach(&mut self, surface: AttachedSurface) -> bool {
        let _: Option<SurfaceHandles> = surface.handles;
        self.set_extent(surface.size, surface.scale);
        self.attached = true;
        true
    }

    fn resize(&mut self, size: Size, scale: f64) {
        if self.attached {
            self.set_extent(size, scale);
        }
    }

    fn present(&mut self, scene: &Scene, resources: &ResourceRegistry) {
        if !self.attached {
            return;
        }
        self.frames += 1;
        let info = FrameInfo::software(self.width, self.height, self.frames);
        self.renderer.begin_frame(&info);
        // The registry drives text (glyph-run rasterisation) and image blits;
        // the renderer itself is damage-aware, so an unchanged frame skips.
        self.renderer
            .render_scene_with_resources(scene, scene.damage(), resources);
        self.renderer.end_frame();
    }

    fn detach(&mut self) {
        self.attached = false;
    }

    fn frames_presented(&self) -> u64 {
        self.frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use floem_scene::{Point, Rect, SceneNode};
    use floem_types::{Color, SurfaceId, WindowId};

    fn attached(size: Size) -> AttachedSurface {
        AttachedSurface::new(WindowId::new(0, 1), SurfaceId::new(0, 1), size, 1.0, None)
    }

    fn single_rect_scene() -> Scene {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(4.0, 4.0)),
            fill: Color::rgb(200, 100, 50),
            radius: 0.0,
        });
        scene.finish_update();
        scene
    }

    #[test]
    fn attach_sizes_the_buffer_to_physical_pixels() {
        let mut presenter = SoftwarePresenter::new();
        assert!(presenter.attach(attached(Size::new(8.0, 6.0))));
        assert_eq!((presenter.width(), presenter.height()), (8, 6));
        assert!(presenter.is_attached());
    }

    #[test]
    fn scale_factor_multiplies_the_buffer() {
        let mut presenter = SoftwarePresenter::new();
        let surface = AttachedSurface::new(
            WindowId::new(0, 1),
            SurfaceId::new(0, 1),
            Size::new(8.0, 6.0),
            2.0,
            None,
        );
        presenter.attach(surface);
        assert_eq!((presenter.width(), presenter.height()), (16, 12));
    }

    #[test]
    fn present_draws_into_the_buffer() {
        let mut presenter = SoftwarePresenter::new();
        presenter.attach(attached(Size::new(8.0, 8.0)));
        presenter.present(&single_rect_scene(), &ResourceRegistry::new());
        assert_eq!(presenter.frames_presented(), 1);
        assert_eq!(presenter.pixel_rgba(1, 1), Some([200, 100, 50, 255]));
        assert_eq!(presenter.pixel_rgba(6, 6), Some([0, 0, 0, 0]));
    }

    #[test]
    fn present_before_attach_is_a_noop() {
        let mut presenter = SoftwarePresenter::new();
        presenter.present(&single_rect_scene(), &ResourceRegistry::new());
        assert_eq!(presenter.frames_presented(), 0);
    }

    #[test]
    fn detach_stops_presenting_and_resize_is_ignored() {
        let mut presenter = SoftwarePresenter::new();
        presenter.attach(attached(Size::new(8.0, 8.0)));
        presenter.detach();
        assert!(!presenter.is_attached());
        presenter.present(&single_rect_scene(), &ResourceRegistry::new());
        assert_eq!(presenter.frames_presented(), 0);

        presenter.resize(Size::new(100.0, 100.0), 1.0);
        assert_eq!((presenter.width(), presenter.height()), (8, 8));
    }

    #[test]
    fn resize_while_attached_rebuilds_the_buffer() {
        let mut presenter = SoftwarePresenter::new();
        presenter.attach(attached(Size::new(8.0, 8.0)));
        presenter.resize(Size::new(20.0, 10.0), 1.0);
        assert_eq!((presenter.width(), presenter.height()), (20, 10));
        presenter.present(&single_rect_scene(), &ResourceRegistry::new());
        assert_eq!(presenter.pixel_rgba(1, 1), Some([200, 100, 50, 255]));
    }
}
