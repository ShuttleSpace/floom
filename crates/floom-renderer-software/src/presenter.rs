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

use floom_renderer::{
    AttachedSurface, FrameInfo, Presenter, RenderTarget, Renderer, RendererCapabilities,
    SurfaceHandles,
};
use floom_scene::{ResourceRegistry, Scene};
use floom_types::Size;

use crate::SoftwareRenderer;

/// [`Presenter`] that renders into an RGBA8 buffer.
pub struct SoftwarePresenter {
    renderer: SoftwareRenderer,
    width: u32,
    height: u32,
    /// Logical size of the attached surface, as opposed to the device-pixel
    /// extent in `width`/`height`. Both are handed to the renderer: the scene
    /// it receives is authored in these units.
    logical: Size,
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
            logical: Size::new(1.0, 1.0),
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
        self.logical = size;
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
        // The buffer is in device pixels, the scene in logical ones: the frame
        // has to carry both, or the renderer has no way to know the scene is
        // not already in the units it is painting into.
        let info = FrameInfo {
            target: RenderTarget::Software {
                width: self.width,
                height: self.height,
            },
            logical_size: self.logical,
            scale_factor: self.scale,
            frame_number: self.frames,
        };
        self.renderer.begin_frame(&info);
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

    /// The software renderer's answer, because the software renderer is what
    /// does the drawing.
    fn capabilities(&self) -> RendererCapabilities {
        self.renderer.capabilities()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use floom_scene::{Point, Rect, SceneNode};
    use floom_types::{Color, SurfaceId, WindowId};

    fn attached(size: Size) -> AttachedSurface {
        AttachedSurface::new(WindowId::new(0, 1), SurfaceId::new(0, 1), size, 1.0, None)
    }

    fn single_rect_scene() -> Scene {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(4.0, 4.0)),
            fill: floom_types::Brush::from(Color::rgb(200, 100, 50)),
            radius: 0.0,

            shadow: None,
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

    /// A scene is authored in *logical* units and the buffer is in *device*
    /// pixels, so the scale factor is the only thing that crosses the gap. A
    /// 4×4 logical rect on a 2× display is 8×8 device pixels: it reaches
    /// device (7,7) and stops there.
    ///
    /// Painting it 1:1 leaves the app drawn at half size in the corner of its
    /// own surface — and, because hit-testing converts the pointer *into*
    /// logical units, leaves every click landing at 1/scale of where the
    /// widget is drawn. The far pixel is what tells the two apart; asserting
    /// only the near one passes at either scale.
    #[test]
    fn a_logical_scene_is_painted_at_the_scale_factor() {
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

        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(4.0, 4.0)),
            fill: floom_types::Brush::from(Color::rgb(200, 100, 50)),
            radius: 0.0,

            shadow: None,
        });
        scene.finish_update();
        presenter.present(&scene, &ResourceRegistry::new());

        assert_eq!(
            presenter.pixel_rgba(7, 7),
            Some([200, 100, 50, 255]),
            "4 logical px at 2x reach the 8th device pixel"
        );
        assert_eq!(
            presenter.pixel_rgba(9, 9),
            Some([0, 0, 0, 0]),
            "and no further: the scene is scaled, not stretched to the buffer"
        );
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
