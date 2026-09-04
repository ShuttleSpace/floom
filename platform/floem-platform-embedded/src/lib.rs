//! Embedded platform: headless framebuffer simulation (+ optional DRM feature).

use floem_platform::{
    Capability, HostPlatform, Platform, PlatformCapabilities, PlatformEvent, Window,
    WindowConfig,
};
use floem_types::{PlatformId, Size};

pub use floem_platform::{Clipboard, Ime};

/// RGBA8 framebuffer backing a software-rendered embedded surface.
#[derive(Debug, Clone)]
pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let len = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(4);
        Self {
            width,
            height,
            pixels: vec![0; len],
        }
    }

    pub fn fill_rgba(&mut self, r: u8, g: u8, b: u8, a: u8) {
        for chunk in self.pixels.chunks_exact_mut(4) {
            chunk[0] = r;
            chunk[1] = g;
            chunk[2] = b;
            chunk[3] = a;
        }
    }

    pub fn byte_len(&self) -> usize {
        self.pixels.len()
    }
}

#[derive(Debug)]
pub struct EmbeddedPlatform {
    host: HostPlatform,
    fb: Framebuffer,
}

impl EmbeddedPlatform {
    pub fn new(width: u32, height: u32) -> Self {
        let drm = cfg!(feature = "drm");
        Self {
            host: HostPlatform::new()
                .with_id(PlatformId::new(30, 1))
                .with_capabilities(PlatformCapabilities::embedded_default(drm)),
            fb: Framebuffer::new(width, height),
        }
    }

    pub fn framebuffer(&self) -> &Framebuffer {
        &self.fb
    }

    pub fn framebuffer_mut(&mut self) -> &mut Framebuffer {
        &mut self.fb
    }

    /// Present a solid color into the simulated framebuffer (software path).
    pub fn present_solid(&mut self, r: u8, g: u8, b: u8) {
        self.fb.fill_rgba(r, g, b, 255);
    }

    pub fn drm_enabled(&self) -> bool {
        self.host.capabilities().supports(Capability::DRM_KMS)
    }
}

impl Platform for EmbeddedPlatform {
    fn id(&self) -> PlatformId {
        self.host.id()
    }

    fn capabilities(&self) -> PlatformCapabilities {
        self.host.capabilities()
    }

    fn create_window(&mut self, config: WindowConfig) -> floem_types::WindowId {
        let size = Size::new(self.fb.width as f64, self.fb.height as f64);
        let mut cfg = config;
        cfg.size = size;
        self.host.create_window(cfg)
    }

    fn window(&self, id: floem_types::WindowId) -> Option<&dyn Window> {
        self.host.window(id)
    }

    fn window_mut(&mut self, id: floem_types::WindowId) -> Option<&mut dyn Window> {
        self.host.window_mut(id)
    }

    fn poll_events(&mut self, out: &mut Vec<PlatformEvent>) {
        self.host.poll_events(out)
    }

    fn request_frame(&mut self, window: floem_types::WindowId) {
        self.host.request_frame(window)
    }

    fn now_secs(&self) -> f64 {
        self.host.now_secs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use floem_platform::WindowConfig;

    #[test]
    fn framebuffer_software_path() {
        let mut p = EmbeddedPlatform::new(320, 240);
        assert!(!p.drm_enabled());
        assert!(p.capabilities().supports(Capability::SOFTWARE_RENDER));

        let win = p.create_window(WindowConfig::default());
        p.request_frame(win);
        p.present_solid(255, 0, 0);

        assert_eq!(p.framebuffer().byte_len(), 320 * 240 * 4);
        assert_eq!(&p.framebuffer().pixels[0..4], &[255, 0, 0, 255]);

        let mut events = Vec::new();
        p.poll_events(&mut events);
        assert!(!events.is_empty());
    }
}
