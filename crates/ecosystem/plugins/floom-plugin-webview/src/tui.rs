//! The terminal (TUI) host binding: honest refusal.
//!
//! A terminal has no view layer to contribute — there is no compositor in a
//! pty. The capability honestly reports that it cannot be provided here rather
//! than faking a virtual surface.

use std::ffi::c_void;
use std::sync::atomic::{AtomicI32, Ordering};

use floom_native_ffi::CProviderError;
use floom_native_ffi::floom_native::NativeViewProvider;

static REFUSED: AtomicI32 = AtomicI32::new(0);

pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

pub fn enable() -> Result<(), String> {
    Ok(())
}

pub fn disable() {}

pub fn capability() -> i32 {
    0
}

/// # Safety
///
/// The signature exists so the terminal binding has the same shape as the
/// real ones. It never dereferences `_host_window`: the call refuses before
/// touching it, so any value — including a dangling one — is sound.
pub unsafe fn provider(
    _host_window: *const c_void,
) -> Result<Box<dyn NativeViewProvider>, CProviderError> {
    Err(CProviderError::MissingEntry(
        "the terminal host has no view layer for a native view to live in",
    ))
}

/// # Safety
///
/// Same contract as [`provider`]: the handle is never dereferenced, and the
/// refusal is returned before it could be.
pub unsafe fn boxed_provider(
    _host_window: *const c_void,
) -> Result<Box<dyn NativeViewProvider>, CProviderError> {
    unsafe { provider(_host_window) }
}

pub fn load_url(
    _slot: floom_native_ffi::floom_native::NativeSlot,
    url: &str,
) -> Result<(), String> {
    Err(format!("no view to load into on the terminal host: {url}"))
}

pub fn post_message(
    _slot: floom_native_ffi::floom_native::NativeSlot,
    json: &str,
) -> Result<(), String> {
    Err(format!(
        "no view to deliver a message to on the terminal host: {json}"
    ))
}

pub fn set_message_callback(
    _cb: Option<
        unsafe extern "C" fn(floom_native_ffi::abi::FloomNativeSlot, *const std::ffi::c_char),
    >,
) {
}

pub mod diagnostics {
    use super::*;

    pub fn created() -> i32 {
        0
    }

    pub fn destroyed() -> i32 {
        0
    }

    pub fn refused() -> i32 {
        REFUSED.load(Ordering::Relaxed)
    }

    pub fn live() -> i32 {
        0
    }

    pub fn slot_is_web(_slot: floom_native_ffi::floom_native::NativeSlot) -> bool {
        false
    }

    pub fn slot_is_attached(_slot: floom_native_ffi::floom_native::NativeSlot) -> bool {
        false
    }

    pub fn last_frame() -> (f64, f64, f64, f64) {
        (0.0, 0.0, 0.0, 0.0)
    }

    pub fn focused() -> i32 {
        0
    }

    pub fn applied() -> i32 {
        0
    }
}
