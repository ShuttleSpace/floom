//! The OpenHarmony host binding: honest refusal.
//!
//! OpenHarmony currently has no WebKit-style native-view surface wired into the
//! provider ABI, so the capability honestly reports that it is not available on
//! this host rather than claiming a view that cannot exist.

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

pub unsafe fn provider(
    _host_window: *const c_void,
) -> Result<Box<dyn NativeViewProvider>, CProviderError> {
    Err(CProviderError::MissingEntry(
        "the OpenHarmony binding has not implemented a native view yet",
    ))
}

pub unsafe fn boxed_provider(
    _host_window: *const c_void,
) -> Result<Box<dyn NativeViewProvider>, CProviderError> {
    unsafe { provider(_host_window) }
}

pub fn load_url(
    _slot: floom_native_ffi::floom_native::NativeSlot,
    url: &str,
) -> Result<(), String> {
    Err(format!(
        "no web view is bound on the OpenHarmony host: {url}"
    ))
}

pub fn post_message(
    _slot: floom_native_ffi::floom_native::NativeSlot,
    json: &str,
) -> Result<(), String> {
    Err(format!(
        "no web view is bound on the OpenHarmony host: {json}"
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
