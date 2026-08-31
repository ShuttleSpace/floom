//! Rust adapter traits that make native Rust backends ergonomic.
//!
//! Backends may either implement these traits or feed [`EmbedderEvent`] values
//! directly and execute returned [`EmbedderCommand`] values.

use super::{BackendCapabilities, EmbedderWindowId, SurfaceDescriptor};
use winit::dpi::PhysicalInsets;

/// Owns the event loop, wakeups, timers, and frame pacing.
pub trait EmbedderRuntime {
    /// Wake the event loop so pending work is processed.
    fn wake(&mut self);
    /// Request a timer identified by `token` firing at `deadline_ms` (epoch millis).
    fn request_timer(&mut self, token: u64, deadline_ms: u64);
    /// Cancel a previously requested timer.
    fn cancel_timer(&mut self, token: u64);
}

/// A window owned by the backend, identified within the embedder protocol.
pub trait EmbedderWindow {
    /// The protocol-level window id.
    fn id(&self) -> EmbedderWindowId;
    /// Request a redraw of this window.
    fn request_redraw(&mut self);
    /// Set the window title.
    fn set_title(&mut self, title: &str);
    /// The safe-area insets in **physical (device) pixels** — e.g. the iOS
    /// notch / Home Indicator or Android system bars. Backends that cannot or
    /// do not report it return zero insets (the default). Layout can use this
    /// to keep content clear of OS chrome; the default implementation keeps the
    /// protocol backward-compatible for backends that predate this method.
    fn safe_area(&self) -> PhysicalInsets<u32> {
        PhysicalInsets {
            top: 0,
            left: 0,
            bottom: 0,
            right: 0,
        }
    }
}

/// A rendering surface owned by the backend.
pub trait SurfaceHandle {
    /// Describe the current surface.
    fn descriptor(&self) -> SurfaceDescriptor;
}

/// Platform services (clipboard, theme, IME, etc.) exposed to Floem.
pub trait PlatformServices {
    /// Report the capabilities of this backend.
    fn capabilities(&self) -> BackendCapabilities;
    /// Read the system clipboard.
    fn read_clipboard(&mut self) -> Result<String, String>;
    /// Write the system clipboard.
    fn write_clipboard(&mut self, text: String) -> Result<(), String>;
}
