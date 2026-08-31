//! Shared OpenHarmony N-API bridge registration.
//!
//! OpenHarmony exposes OS services only through ArkTS, so every OHOS platform
//! bridge (clipboard, external URL, file picker, receive-files) follows the
//! same pattern: at startup the ArkTS side calls [`register_platform_bridge`]
//! with one object that wraps the ArkTS capabilities, and the Rust side calls
//! *back* into that object through N-API via [`call_bridge`].
//!
//! This mirrors the Android JNI bridge (`crate::platform::android_jni`), which
//! also captures a platform handle once and dispatches from it.
//!
//! # Status
//! Compiled only for `target_env = "ohos"`. Depends on `napi` (napi-rs) and on
//! the OpenHarmony N-API lib that `cargo-ohos-app` links into `libentry.so`.
//! **UNVERIFIED** against a real OHOS SDK build (see `docs/ohos.md`).

#![cfg(target_env = "ohos")]

use std::sync::OnceLock;

use napi::{Env, JsObject, Ref, Result as NapiResult};
use napi_derive::napi;

/// A persistent handle to the ArkTS platform bridge object registered at startup.
struct PlatformBridge {
    /// The N-API environment captured at registration time. N-API calls must
    /// run on the thread that owns this `env` (the ArkTS / main thread).
    env: Env,
    /// A type-erased reference to the JS object so it is not GC'd.
    object: Ref<()>,
}

// SAFETY: `PlatformBridge` is only ever accessed from the single ArkTS / main
// thread that registered it (via `call_bridge` / `register_platform_bridge`).
// The `Env` raw pointer and `Ref<()>` are never dereferenced from another
// thread, so asserting `Send`/`Sync` for the `static` is sound for OHOS.
unsafe impl Send for PlatformBridge {}
unsafe impl Sync for PlatformBridge {}

static BRIDGE: OnceLock<PlatformBridge> = OnceLock::new();

/// Exported to ArkTS. Call **once** during ability / XComponent startup with a
/// single object that implements the union of the platform capabilities:
///
/// ```typescript
/// {
///   getText(): string | null;
///   setText(text: string): void;
///   openUrl(url: string): void;
///   openFile(opts): Promise<Array<string> | null>;   // files
///   saveFile(opts): Promise<string | null>;           // file
///   setAccessibilityTree(tree): void;                 // a11y
///   showKeyboard(): void;                             // IME (see ohos_ime.rs)
///   hideKeyboard(): void;                             // IME
/// }
/// ```
///
/// (`receive_files` is the reverse direction — ArkTS pushes shared files into
/// Rust via [`crate::platform::ohos_receive::on_shared_files`]; IME text flows
/// back through the `on_ime_*` exports in [`crate::platform::ohos_ime`].)
// TODO(OHOS): verify the `#[napi]` import path and that `cargo-ohos-app` links
// the N-API lib + registers this module alongside the winit C-ABI entry.
#[napi]
pub fn register_platform_bridge(env: Env, bridge: JsObject) -> NapiResult<()> {
    let object = env.create_reference(bridge)?;
    // Ignore a second registration; keep the first handle.
    let _ = BRIDGE.set(PlatformBridge { env, object });
    Ok(())
}

/// Run `f` with the registered bridge object, or return `None` if no bridge has
/// been registered yet (ArkTS startup not reached, or registration failed).
///
/// `f` returns `napi::Result<R>`; any error from the N-API call is converted to
/// `None` so callers can fall back to a capability-unavailable error.
pub(crate) fn call_bridge<R>(
    f: impl FnOnce(Env, JsObject) -> NapiResult<R>,
) -> Option<R> {
    let bridge = BRIDGE.get()?;
    let obj: JsObject = bridge.env.get_reference_value(&bridge.object).ok()?;
    f(bridge.env, obj).ok()
}
