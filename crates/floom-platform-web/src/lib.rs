//! Web / WASM platform adapter.
//!
//! This crate provides [`WebPlatform`] — a `Platform` backend that collects DOM
//! pointer/keyboard/wheel events and drives an `AppRunner` through
//! `requestAnimationFrame` — plus the [`run`](crate::run) entry that wires a
//! `WebApp` to a `<canvas>`. The pixels are produced by
//! `floom-renderer-web`'s `WebPresenter`, which rasterises the scene on the CPU
//! (reusing `floom-renderer-software`) and blits the buffer to the canvas.

pub mod accessibility;
pub mod runloop;

use floom_platform::{
    HostPlatform, Ime, Platform, PlatformCapabilities, PlatformEvent, Window, WindowConfig,
};
use floom_types::{PlatformId, Size, WindowId};

pub use floom_platform::{Capability, Clipboard, Platform as _};
pub use runloop::{WebApp, run};

/// Detect WebGPU availability.
///
/// Native/CI builds report `false` (software fallback). WASM builds can flip this
/// when a real `navigator.gpu` probe is wired.
pub fn detect_webgpu() -> bool {
    cfg!(all(target_arch = "wasm32", feature = "webgpu-probe"))
}

pub fn web_platform() -> HostPlatform {
    HostPlatform::new()
        .with_id(PlatformId::new(20, 1))
        .with_capabilities(PlatformCapabilities::web_default(detect_webgpu()))
}

/// Hook for DOM / semantic bridge registration (stub).
#[derive(Debug, Default, Clone)]
pub struct DomBridge {
    pub root_selector: Option<String>,
}

impl DomBridge {
    pub fn attach(&mut self, selector: &str) {
        self.root_selector = Some(selector.to_string());
    }
}

/// [`Platform`] backend that turns DOM input + `requestAnimationFrame` into a
/// live Floom window.
///
/// It delegates the in-memory window/surface bookkeeping to [`HostPlatform`]
/// (exactly the pattern the embedded and TUI backends use) and only adds the
/// web-specific glue: a `main_window` id for event tagging, and an event-driven
/// frame clock implemented via [`Platform::request_frame`] + `pending_frame`,
/// which the [`run`](crate::run) loop reads to decide whether to keep animating.
///
/// [`Platform::request_frame`]: floom_platform::Platform::request_frame
#[derive(Debug)]
pub struct WebPlatform {
    host: HostPlatform,
    /// The window DOM listeners tag their events with.
    main_window: Option<WindowId>,
    /// Set by `request_frame`; consumed by the run loop to decide whether to
    /// schedule another animation frame (event-driven, self-settling).
    #[allow(dead_code)]
    pending_frame: bool,
    /// Whether an animation-frame callback is already pending (dedup).
    #[allow(dead_code)]
    raf_pending: bool,
}

impl WebPlatform {
    /// Build a platform. `canvas_id` is the DOM id of the `<canvas>` the
    /// presenter blits into (resolved at blit time, so it is only consulted by
    /// the event loop, which uses it as the input target).
    pub fn new(_canvas_id: &str) -> Self {
        Self {
            host: HostPlatform::new()
                .with_id(PlatformId::new(20, 1))
                .with_capabilities(PlatformCapabilities::web_default(detect_webgpu())),
            main_window: None,
            pending_frame: false,
            raf_pending: false,
        }
    }

    /// Seed the device-pixel ratio from `window.devicePixelRatio()` before any
    /// window is created.
    pub fn set_scale(&mut self, scale: f64) {
        self.host.set_scale(scale);
    }

    /// The window DOM listeners tag their events with.
    pub fn set_main_window(&mut self, window: WindowId) {
        self.main_window = Some(window);
    }

    pub fn main_window(&self) -> Option<WindowId> {
        self.main_window
    }

    /// Queue a platform event (DOM listeners call this).
    pub fn push_event(&mut self, event: PlatformEvent) {
        self.host.push_event(event);
    }

    /// Announce a new logical window size (the resize listener calls this).
    pub fn resize_to(&mut self, size: Size) {
        if let Some(window) = self.main_window {
            self.host
                .push_event(PlatformEvent::WindowResized { window, size });
        }
    }

    /// Consumed by the run loop after a pump: whether another frame is needed.
    #[allow(dead_code)]
    pub(crate) fn take_pending_frame(&mut self) -> bool {
        std::mem::replace(&mut self.pending_frame, false)
    }

    #[allow(dead_code)]
    pub(crate) fn set_raf_pending(&mut self, value: bool) {
        self.raf_pending = value;
    }

    #[allow(dead_code)]
    pub(crate) fn raf_pending(&self) -> bool {
        self.raf_pending
    }
}

impl Platform for WebPlatform {
    fn id(&self) -> PlatformId {
        self.host.id()
    }

    fn capabilities(&self) -> PlatformCapabilities {
        self.host.capabilities()
    }

    fn create_window(&mut self, config: WindowConfig) -> WindowId {
        let id = self.host.create_window(config);
        self.main_window = Some(id);
        id
    }

    fn window(&self, id: WindowId) -> Option<&dyn Window> {
        self.host.window(id)
    }

    fn window_mut(&mut self, id: WindowId) -> Option<&mut dyn Window> {
        self.host.window_mut(id)
    }

    fn poll_events(&mut self, out: &mut Vec<PlatformEvent>) {
        self.host.poll_events(out);
    }

    fn request_frame(&mut self, window: WindowId) {
        // Ask the runner to redraw and flag the loop to keep animating until the
        // scene settles (the runner stops requesting frames when the tree is
        // static, so this naturally self-terminates).
        self.host
            .push_event(PlatformEvent::RedrawRequested { window });
        self.pending_frame = true;
    }

    fn now_secs(&self) -> f64 {
        self.host.now_secs()
    }

    fn ime(&mut self, window: WindowId) -> Option<&mut dyn Ime> {
        self.host.ime(window)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use floom_platform::{Platform, WindowConfig};
    use floom_types::Size;

    #[test]
    fn webgpu_fallback_and_frame_clock() {
        assert!(!detect_webgpu());
        let mut p = WebPlatform::new("floom-canvas");
        assert!(!p.capabilities().supports(Capability::WEBGPU));
        assert!(p.capabilities().supports(Capability::SOFTWARE_RENDER));
        // No native view on the web: the browser has no subwindow to put one
        // in, and the backend produces no provider. Claiming the capability
        // would make every `NativeSurface` mount fail at runtime instead of
        // being refused up front.
        assert!(!p.capabilities().supports(Capability::NATIVE_VIEW));

        let win = p.create_window(WindowConfig {
            title: "web".into(),
            size: Size::new(1280.0, 720.0),
            resizable: true,
        });
        assert_eq!(p.main_window(), Some(win));
        p.request_frame(win);
        let t0 = p.now_secs();
        assert!(t0 >= 0.0);

        let mut events = Vec::new();
        p.poll_events(&mut events);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, PlatformEvent::RedrawRequested { .. })),
            "request_frame must enqueue a RedrawRequested"
        );
        assert!(p.take_pending_frame(), "request_frame sets pending_frame");
        assert!(
            !p.take_pending_frame(),
            "pending_frame is cleared once read"
        );
    }

    #[test]
    fn resize_queues_window_resized() {
        let mut p = WebPlatform::new("floom-canvas");
        let win = p.create_window(WindowConfig::default());
        p.resize_to(Size::new(640.0, 480.0));
        let mut events = Vec::new();
        p.poll_events(&mut events);
        assert!(
            events.iter().any(
                |e| matches!(e, PlatformEvent::WindowResized { size, .. } if *size
                    == Size::new(640.0, 480.0))
            ),
            "resize_to must enqueue a WindowResized with the new size"
        );
    }

    #[test]
    fn dom_bridge_hook() {
        let mut bridge = DomBridge::default();
        bridge.attach("#floom-root");
        assert_eq!(bridge.root_selector.as_deref(), Some("#floom-root"));
    }
}
