//! Web/WASM canvas presenter for Floom 2.0.
//!
//! The scene is rasterized on the CPU by [`floom_renderer_software`] into an
//! RGBA8 buffer (which handles rects, paths, gradients, shadows, filters,
//! images and swash-rasterized text), then blitted to a real `<canvas>` 2D
//! context on `present`. This keeps the web backend's pixel output identical to
//! the headless software presenter while giving it a live on-screen surface.

use floom_renderer::{AttachedSurface, FrameInfo, Presenter, RenderTarget, Renderer};
use floom_renderer_software::SoftwareRenderer;
use floom_scene::{ResourceRegistry, Scene};
use floom_types::Size;

/// [`Presenter`] that draws into a CPU buffer and blits it to a web canvas.
pub struct WebPresenter {
    renderer: SoftwareRenderer,
    width: u32,
    height: u32,
    /// Logical size of the attached surface.
    logical: Size,
    scale: f64,
    attached: bool,
    frames: u64,
}

impl Default for WebPresenter {
    fn default() -> Self {
        Self::new()
    }
}

impl WebPresenter {
    /// A presenter with no surface yet; `attach` sizes the buffer + canvas.
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

    /// The rasterized RGBA8 buffer (straight alpha), for inspection / testing.
    pub fn pixels(&self) -> &[u8] {
        self.renderer.pixels()
    }

    /// Whether a surface is currently attached.
    pub fn is_attached(&self) -> bool {
        self.attached
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
        // Resize the on-screen canvas to match the device-pixel buffer.
        canvas::resize_canvas(width, height, size);
    }
}

impl Presenter for WebPresenter {
    /// Always succeeds: the CPU buffer needs no native handles. The `handles`
    /// field is deliberately ignored (the canvas is located by element id at
    /// blit time), so the same presenter serves a real window and a headless
    /// harness.
    fn attach(&mut self, surface: AttachedSurface) -> bool {
        let _: Option<floom_renderer::SurfaceHandles> = surface.handles;
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
        // The registry drives text (glyph-run rasterisation) and image blits;
        // the renderer itself is damage-aware, so an unchanged frame skips.
        self.renderer
            .render_scene_with_resources(scene, scene.damage(), resources);
        self.renderer.end_frame();

        // Push the freshly rasterized pixels onto the live canvas.
        canvas::blit(self.renderer.pixels(), self.width, self.height);
    }

    fn detach(&mut self) {
        self.attached = false;
    }

    fn frames_presented(&self) -> u64 {
        self.frames
    }
}

/// Canvas bridge: on wasm this talks to the real DOM; elsewhere it is a no-op so
/// the crate still compiles and runs headless (the buffer is the source of
/// truth for tests).
mod canvas {
    #[cfg(target_arch = "wasm32")]
    pub(super) fn canvas_element() -> Option<web_sys::HtmlCanvasElement> {
        use wasm_bindgen::JsCast;
        let window = web_sys::window()?;
        let doc = window.document()?;
        if let Some(el) = doc.get_element_by_id("floom-canvas") {
            return el.dyn_into::<web_sys::HtmlCanvasElement>().ok();
        }
        doc.query_selector("canvas")
            .ok()
            .flatten()?
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .ok()
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn resize_canvas(width: u32, height: u32, logical: floom_types::Size) {
        let Some(canvas) = canvas_element() else {
            return;
        };
        canvas.set_width(width);
        canvas.set_height(height);
        let _ = canvas
            .style()
            .set_property("width", &format!("{}px", logical.width.max(1.0)));
        let _ = canvas
            .style()
            .set_property("height", &format!("{}px", logical.height.max(1.0)));
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn blit(pixels: &[u8], width: u32, height: u32) {
        use wasm_bindgen::Clamped;
        use wasm_bindgen::JsCast;
        let Some(canvas) = canvas_element() else {
            return;
        };
        let Some(ctx) = canvas
            .get_context("2d")
            .ok()
            .flatten()
            .and_then(|v| v.dyn_into::<web_sys::CanvasRenderingContext2d>().ok())
        else {
            return;
        };
        let Ok(image_data) =
            web_sys::ImageData::new_with_u8_clamped_array_and_sh(Clamped(pixels), height, width)
        else {
            return;
        };
        let _ = ctx.put_image_data(&image_data, 0.0, 0.0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn resize_canvas(_width: u32, _height: u32, _logical: floom_types::Size) {}

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn blit(_pixels: &[u8], _width: u32, _height: u32) {}
}
