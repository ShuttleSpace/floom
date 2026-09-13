//! OS-neutral platform contracts for Floem 2.0.
//!
//! Concrete backends (desktop/mobile/web/embedded) live in sibling crates and
//! must not leak into `floem-types` / `floem-scene` / widgets.

mod capabilities;
mod event;
mod fake;
mod keymap;
mod surface;
mod window;

pub use capabilities::{Capability, PlatformCapabilities};
pub use event::{PlatformEvent, PointerButton};
pub use fake::{FakePlatform, HostPlatform};
pub use floem_types::{DisplayInfo, Modifiers, Point, ScaleFactor, Size, SurfaceId, WindowId};
pub use keymap::KeyCode;
pub use surface::{Surface, SurfaceConfig, SurfaceTarget};
pub use window::{Window, WindowConfig, WindowState};

use floem_types::PlatformId;

/// Top-level platform backend.
pub trait Platform {
    fn id(&self) -> PlatformId;
    fn capabilities(&self) -> PlatformCapabilities;
    fn create_window(&mut self, config: WindowConfig) -> WindowId;
    fn window(&self, id: WindowId) -> Option<&dyn Window>;
    fn window_mut(&mut self, id: WindowId) -> Option<&mut dyn Window>;
    fn poll_events(&mut self, out: &mut Vec<PlatformEvent>);
    fn request_frame(&mut self, window: WindowId);
    /// Optional frame clock reading (seconds since start).
    fn now_secs(&self) -> f64;
}

/// Clipboard service (capability-gated).
pub trait Clipboard {
    fn get_text(&mut self) -> Option<String>;
    fn set_text(&mut self, text: &str) -> bool;
}

/// IME / text input bridge (capability-gated).
pub trait Ime {
    fn set_enabled(&mut self, window: WindowId, enabled: bool);
    fn set_cursor_area(&mut self, window: WindowId, origin: Point, size: Size);
}
