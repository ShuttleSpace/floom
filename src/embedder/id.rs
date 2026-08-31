//! Platform-neutral identifiers used by the embedder protocol.
//!
//! These are plain values so non-winit backends (Android, iOS, HarmonyOS,
//! TUI, headless) can mint their own identifiers without depending on any
//! windowing library.

/// Opaque, backend-assigned window identifier.
///
/// Backends translate their native window id into this value. The winit
/// backend uses `winit::window::WindowId::into_raw()`.
pub type EmbedderWindowId = u64;

/// Opaque, backend-assigned pointer identifier.
///
/// Most pointer events carry their own stable id inside the
/// [`ui_events`](crate::ui_events) pointer payload; this type is provided for
/// backends that need a standalone, protocol-level pointer handle.
pub type EmbedderPointerId = u64;
