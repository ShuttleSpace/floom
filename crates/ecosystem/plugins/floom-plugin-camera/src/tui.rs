//! Terminal half of the camera plugin.
//!
//! A terminal UI has no camera, so the capability is unsupported: the module
//! exists so the `tui` feature's surface is complete and so the host sees a
//! clear refusal rather than a missing symbol.

use std::ffi::c_void;

use floom_native_ffi::floom_native::NativeViewProvider;
use floom_native_ffi::CProviderError;

const MESSAGE: &str = "the camera plugin is not available on a terminal backend";

/// Claims the platform state — and refuses, because there is no camera half.
pub fn enable() -> Result<(), String> {
    Err(MESSAGE.to_string())
}

/// Releases whatever [`enable`] claimed — nothing to release.
pub fn disable() {}

/// Builds a provider — and refuses, because there is no camera half.
///
/// # Safety
///
/// `host_window` is unused; the contract is the same as the platform halves.
pub unsafe fn provider(
    _host_window: *const c_void,
) -> Result<Box<dyn NativeViewProvider>, CProviderError> {
    Err(CProviderError::MissingEntry(MESSAGE))
}

/// Does not register: a terminal backend should not carry a camera plugin at
/// all, so the host reports it unavailable.
pub fn register() {}
