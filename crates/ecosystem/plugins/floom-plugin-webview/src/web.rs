//! The web host binding: honest refusal.
//!
//! A browser host *is* the web view — the document the page renders into is
//! the view. There is no separate native-view layer for a plugin to contribute
//! above the page, so the capability honestly reports that it cannot be provided
//! here rather than pretending a virtual layer exists.
//!
//! The unified names are kept stable: [`provider`] refuses, [`load_url`] refuses,
//! [`post_message`] refuses, and the diagnostics report a table that never grows.

use std::ffi::c_void;
use std::sync::atomic::{AtomicI32, Ordering};

use floom_native_ffi::CProviderError;
use floom_native_ffi::floom_native::NativeViewProvider;

static REFUSED: AtomicI32 = AtomicI32::new(0);

/// The plugin's version, from the crate. The web host has no native half, so
/// the crate version is the truthful one.
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// The web host has nothing to claim.
pub fn enable() -> Result<(), String> {
    Ok(())
}

/// The web host has nothing to release.
pub fn disable() {}

/// The plugin's capability id. No native view is contributed on this host.
pub fn capability() -> i32 {
    0
}

/// No native view is contributed by a web host, and the refusal says so.
pub unsafe fn provider(
    _host_window: *const c_void,
) -> Result<Box<dyn NativeViewProvider>, CProviderError> {
    Err(CProviderError::MissingEntry(
        "the web host is itself the web view; no separate native-view capability applies",
    ))
}

/// No native view is contributed by a web host, and the refusal says so.
pub unsafe fn boxed_provider(
    _host_window: *const c_void,
) -> Result<Box<dyn NativeViewProvider>, CProviderError> {
    unsafe { provider(_host_window) }
}

/// No view exists to load into.
pub fn load_url(slot: floom_native_ffi::floom_native::NativeSlot, url: &str) -> Result<(), String> {
    Err(format!("no web view is bound on the web host: {url}"))
}

/// No view exists to deliver a message to.
pub fn post_message(
    slot: floom_native_ffi::floom_native::NativeSlot,
    json: &str,
) -> Result<(), String> {
    Err(format!("no web view is bound on the web host: {json}"))
}

/// No view exists to deliver a message to, so there is no callback to set.
pub fn set_message_callback(
    _cb: Option<
        unsafe extern "C" fn(floom_native_ffi::abi::FloomNativeSlot, *const std::ffi::c_char),
    >,
) {
}

/// The native half never ran on this host, so nothing was ever created; the
/// table is all zeros.
pub mod diagnostics {
    use super::*;

    /// How many participants were created. Always zero on the web host.
    pub fn created() -> i32 {
        0
    }

    /// How many participants were torn down. Always zero on the web host.
    pub fn destroyed() -> i32 {
        0
    }

    /// How many create calls were refused. Counts the refusals this host is
    /// asked for.
    pub fn refused() -> i32 {
        REFUSED.load(Ordering::Relaxed)
    }

    /// How many participants are alive right now. Always zero on the web host.
    pub fn live() -> i32 {
        0
    }

    /// Whether the participant in `slot` holds a real web view. Never true.
    pub fn slot_is_web(_slot: floom_native_ffi::floom_native::NativeSlot) -> bool {
        false
    }

    /// Whether the participant in `slot` is really in a window. Never true.
    pub fn slot_is_attached(_slot: floom_native_ffi::floom_native::NativeSlot) -> bool {
        false
    }

    /// The last frame placed. None has ever been placed.
    pub fn last_frame() -> (f64, f64, f64, f64) {
        (0.0, 0.0, 0.0, 0.0)
    }

    /// How many times focus was taken. Zero.
    pub fn focused() -> i32 {
        0
    }

    /// How many placements were applied. Zero.
    pub fn applied() -> i32 {
        0
    }
}
