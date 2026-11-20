//! Android half of the `webview` plugin — not built yet.
//!
//! There is no `native/android/` for this plugin: no `FloomWebView.java`, no
//! Gradle module, so no Java `WebView` for the Rust side to drive. What used to
//! live here was 390 lines of JNI plumbing written against a `jni` API that no
//! longer exists (`Env::call_object_method`), for a native half that was never
//! written — it did not compile, and nothing caught it because the Android arm
//! is only checked on a cross target.
//!
//! Every entry point keeps the same shape as the other arms and answers with
//! the reason, the way `tray/ohos.rs` does. To make this a real capability:
//! port `floom-plugin-camera/android/` (Gradle + Java view + JNI calls), then
//! fill these bodies in — the trait to implement is `NativeViewProvider`.

use floom_native_ffi::floom_native::NativeSlot;

const WHY: &str = "the android web view has no native half yet — there is no \
                   `native/android/FloomWebView.java` in this plugin, so there is \
                   nothing for the Rust side to drive; the macOS, iOS, Linux and \
                   Windows arms are the working ones";

/// The plugin's version.
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Claims the plugin's platform state. Nothing to claim: see [`WHY`].
pub fn enable() -> Result<(), String> {
    Err(format!("webview: {WHY}"))
}

/// Releases whatever [`enable`] claimed.
pub fn disable() {}

/// The plugin's capability id. The web-view capability claims no platform id.
pub fn capability() -> i32 {
    0
}

/// Send a JSON message from Rust to the web view's JS.
pub fn post_message(_slot: NativeSlot, _json: &str) -> Result<(), String> {
    Err(format!("webview: {WHY}"))
}

/// Register the Rust callback invoked when JS calls
/// `window.floomNative.postMessage(json)`.
pub fn set_message_callback(
    _cb: Option<
        unsafe extern "C" fn(floom_native_ffi::abi::FloomNativeSlot, *const std::ffi::c_char),
    >,
) {
}

/// Loads `url` into the web participant bound to `slot`.
pub fn load_url(_slot: NativeSlot, _url: &str) -> Result<(), String> {
    Err(format!("webview: {WHY}"))
}
