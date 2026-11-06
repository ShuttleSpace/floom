//! Linux half of the camera plugin — camera capability, via the C-ABI bridge.
//!
//! Drives the Linux camera through the provider ABI. The native half
//! (`linux/floom_camera.c`, a GStreamer pipeline composited into the host's
//! X11/GTK window) builds the `FloomNativeProviderV1` vtable and registers it
//! from a constructor; this crate calls it through `extern "C"`.
//!
//! Nothing is looked up by name at runtime: the symbols resolve when the crate is
//! linked.

use std::ffi::{CStr, c_char, c_int, c_void};
use std::sync::OnceLock;

use floom_native_ffi::abi::{FloomNativeProviderV1, FloomNativeSlot};
use floom_native_ffi::floom_native::NativeViewProvider;
use floom_native_ffi::{CProvider, CProviderError};

use crate::common::PLUGIN_NAME;

unsafe extern "C" {
    fn floom_camera_enable() -> c_int;
    fn floom_camera_disable();
    fn floom_camera_refused_count() -> c_int;
    fn floom_camera_live_count() -> c_int;
    fn floom_camera_slot_is_attached(slot: FloomNativeSlot) -> c_int;
}

/// The provider vtable, once the native half has handed it over.
static PROVIDER: OnceLock<usize> = OnceLock::new();

/// Receives the provider vtable from the native half.
///
/// Called from the native half's constructor, so it runs before `main` and
/// before any app asks for the provider.
#[unsafe(no_mangle)]
pub extern "C" fn floom_camera_register_provider(provider: *const FloomNativeProviderV1) {
    if provider.is_null() {
        return;
    }
    let _ = PROVIDER.set(provider as usize);
}

/// Claims the plugin's platform state.
///
/// Called once by the host, through [`floom_platform::register_plugin`].
pub fn enable() -> Result<(), String> {
    // SAFETY: called on the host thread during startup, before any frame.
    let rc = unsafe { floom_camera_enable() };
    if rc != 0 {
        return Err(format!(
            "the Linux camera native half refused to enable with code {rc}"
        ));
    }
    Ok(())
}

/// Releases whatever [`enable`] claimed.
pub fn disable() {
    // SAFETY: the native half tolerates a disable without a matching enable.
    unsafe { floom_camera_disable() };
}

/// The provider for this plugin, if the native half has handed one over.
///
/// `host_window` is the platform's opaque window handle: an X11 `Window` / GTK
/// widget pointer on Linux.
///
/// # Safety
///
/// `host_window` must be a live window handle of the kind the native half
/// expects and must stay valid for as long as the returned provider is driven.
pub unsafe fn provider(
    host_window: *const c_void,
) -> Result<Box<dyn NativeViewProvider>, CProviderError> {
    let Some(raw) = PROVIDER.get().copied() else {
        return Err(CProviderError::MissingEntry(
            "the Linux camera native half registered no provider",
        ));
    };
    let vtable = raw as *const FloomNativeProviderV1;
    let provider = unsafe { CProvider::from_raw(vtable) };
    // SAFETY: the handle outlives the provider by the caller's contract.
    unsafe { provider.with_host(host_window) }.map(|p| Box::new(p) as Box<dyn NativeViewProvider>)
}

/// Registers this plugin with the process-wide table.
pub fn register() {
    floom_platform::register_plugin(PLUGIN_NAME, enable);
}

/// The diagnostics the native half publishes — plain numbers, so a host can
/// check what the native side did without knowing anything about GStreamer.
pub mod diagnostics {
    use super::*;
    use floom_native_ffi::floom_native::NativeSlot;

    /// How many `create` calls were refused for a kind this plugin cannot build.
    pub fn refused() -> i32 {
        // SAFETY: the native half returns a plain integer.
        unsafe { floom_camera_refused_count() }
    }

    /// How many participants are alive right now.
    pub fn live() -> i32 {
        // SAFETY: the native half returns a plain integer.
        unsafe { floom_camera_live_count() }
    }

    /// Whether the participant in `slot` is really in the host window.
    pub fn slot_is_attached(slot: NativeSlot) -> bool {
        let s = FloomNativeSlot {
            index: slot.id.index(),
            generation: slot.generation,
        };
        // SAFETY: the native half returns a plain integer and the slot crosses
        // the boundary in the ABI's own layout.
        unsafe { floom_camera_slot_is_attached(s) != 0 }
    }
}
