//! Fallback for targets the camera plugin does not build for.
//!
//! Selected only when no platform half matches. Reports the capability as
//! unavailable rather than pretending, so a host that enumerates plugins sees a
//! clear refusal instead of a silent no-op.

use std::ffi::c_void;

use floom_native_ffi::CProviderError;
use floom_native_ffi::floom_native::NativeViewProvider;

const MESSAGE: &str = "the camera plugin is not available on this target";

/// Claims the platform state — and refuses, because there is no native half.
pub fn enable() -> Result<(), String> {
    Err(MESSAGE.to_string())
}

/// Nothing to release.
pub fn disable() {}

/// Builds a provider — and refuses, because there is no native half.
///
/// # Safety
///
/// `host_window` is unused; the contract is the same as the platform halves.
pub unsafe fn provider(
    _host_window: *const c_void,
) -> Result<Box<dyn NativeViewProvider>, CProviderError> {
    Err(CProviderError::MissingEntry(MESSAGE))
}

/// Does not register: a plugin that cannot enable should not appear in the
/// registry, so a host's `enable` call fails loudly rather than registering a
/// dangling entry.
pub fn register() {}
