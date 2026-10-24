//! OS-neutral platform contracts for Floom 2.0.
//!
//! Concrete backends (desktop/mobile/web/embedded) live in sibling crates and
//! must not leak into `floom-types` / `floom-scene` / widgets.
//!
//! # Plugins
//!
//! [`register_plugin`] and [`plugin`] are the plugin seam. A plugin is a
//! link-time dependency whose native half is compiled into the binary by its own
//! `build.rs`; the registry holds its enable function, not its code, and
//! nothing crosses a serialization boundary. See [`plugins`] for why this is not
//! a Flutter-style method-channel registry.

mod capabilities;
mod event;
mod fake;
mod keymap;
mod plugins;
mod surface;
mod window;

pub use capabilities::{Capability, PlatformCapabilities};
pub use event::{PlatformEvent, PointerButton};
pub use fake::{FakePlatform, HostPlatform};
pub use floom_types::{DisplayInfo, Modifiers, Point, ScaleFactor, Size, SurfaceId, WindowId};
pub use keymap::KeyCode;
pub use plugins::{
    Plugin, PluginError, plugin, register_plugin, registered_plugins, unregister_plugin,
};
pub use surface::{Surface, SurfaceConfig, SurfaceTarget};
pub use window::{Window, WindowConfig, WindowState};

use floom_types::PlatformId;

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

    /// Access the IME bridge for `window`, if the platform exposes one.
    ///
    /// Returns `None` for backends without input-method support, in which
    /// case the runner skips the caret-area handshake. The platform itself
    /// implements [`Ime`] (it owns the native text-input surface); this method
    /// is the door the runner uses to reach it without naming a concrete
    /// backend type.
    fn ime(&mut self, _window: WindowId) -> Option<&mut dyn Ime> {
        None
    }
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
