//! The `webview` plugin — a capability that is a **native view**.
//!
//! The capability is a view the framework does not paint: a `WKWebView` on
//! macOS, a `WebView` on Android or Linux, a WebView2 on Windows, and so on.
//! What the view *shows* is the web's own business; this crate owns the
//! *capability* (create / place / destroy a native view, drive a JS bridge)
//! and nothing about any one platform.
//!
//! # Layout
//!
//! `src/` is the **unified API**. It is the only surface an example or host
//! touches: [`enable`], [`provider`], [`load_url`], [`post_message`],
//! [`set_message_callback`], [`diagnostics`], [`register`]. None of it names a
//! platform.
//!
//! Each platform is a **peer project to `src/`**, implemented in whatever
//! native code it needs and reached through FFI. All compiled halves live
//! under `native/<platform>/`:
//!
//! | dir              | platform | native half                                   |
//! |------------------|----------|-----------------------------------------------|
//! | `native/macos/`  | macOS    | Objective-C (`WKWebView`), built by `build.rs` |
//! | `native/ios/`    | iOS      | Objective-C (`WKWebView`), built by `build.rs` |
//! | `native/linux/`  | Linux    | C11 + GTK/WebKitGTK, built by `build.rs`       |
//! | `native/windows/`| Windows  | C++ (WebView2), built by `build.rs`            |
//! | `android/`       | Android  | Gradle + Java (`WebView`)                      |
//!
//! `src/web.rs` and `src/tui.rs` are honest refusals: a browser host *is* the
//! web view and a terminal has no view layer, so there is no separate
//! native-view capability to contribute — the plugin says so rather than faking
//! one. `src/ohos.rs` is scaffolded as a refusal until a native XComponent half
//! lands.
//!
//! # Two contracts
//!
//! | | asks | shape |
//! |---|---|---|
//! | the plugin's own ABI (`native/common/floom_webview.h`) | is the capability available? | four functions; the plugin is a singleton |
//! | the provider ABI (`floom-native-ffi`) | how is the native view created, placed and torn down? | a versioned struct of function pointers; the provider is an instance per window |
//!
//! A plugin whose capability is a native view implements both.
//!
//! # Which side owns what
//!
//! The plugin owns the *capability*; the host owns the *transport*. The native
//! half reaches the host's window through the provider's `attach`, and the
//! framework drives the provider through the ordinary Rust [`NativeViewProvider`]
//! trait — so the controller's lifecycle invariants apply to this plugin exactly
//! as they apply to a Rust one.
//!
//! # How the two halves find each other
//!
//! On macOS, the native half (Objective-C) hands its vtable to this crate from a
//! constructor, and this crate hands the provider to the host on request. On
//! every other platform the binding is a Rust struct that drives a native
//! control through that platform's own bridge. Nothing is looked up by name at
//! runtime: the symbols resolve when the crate is linked.

#[cfg(all(target_os = "android", not(feature = "tui")))]
mod android;
#[cfg(all(target_os = "ios", not(feature = "tui")))]
mod ios;
#[cfg(all(target_os = "linux", not(target_env = "ohos"), not(feature = "tui")))]
mod linux;
#[cfg(all(target_os = "macos", not(feature = "tui")))]
mod macos;
#[cfg(all(target_env = "ohos", not(feature = "tui")))]
mod ohos;
#[cfg(all(target_os = "windows", not(feature = "tui")))]
mod windows;
// `web` and `tui` have no native-view capability to contribute; they are
// honest refusals that resolve the same unified names.
#[cfg(feature = "tui")]
mod tui;
#[cfg(all(
    any(target_arch = "wasm32", target_os = "unknown"),
    not(feature = "tui")
))]
mod web;

#[cfg(all(target_os = "android", not(feature = "tui")))]
pub use android::*;
#[cfg(all(target_os = "ios", not(feature = "tui")))]
pub use ios::*;
#[cfg(all(target_os = "linux", not(target_env = "ohos"), not(feature = "tui")))]
pub use linux::*;
#[cfg(all(target_os = "macos", not(feature = "tui")))]
pub use macos::*;
#[cfg(all(target_env = "ohos", not(feature = "tui")))]
pub use ohos::*;
#[cfg(feature = "tui")]
pub use tui::*;
#[cfg(all(
    any(target_arch = "wasm32", target_os = "unknown"),
    not(feature = "tui")
))]
pub use web::*;
#[cfg(all(target_os = "windows", not(feature = "tui")))]
pub use windows::*;

/// Registers this plugin with the process-wide table, using the active
/// platform's `enable`. The name is stable across platforms; what `enable` does
/// is the binding's business.
pub fn register() {
    #[cfg(all(target_os = "macos", not(feature = "tui")))]
    floom_platform::register_plugin("webview", macos::enable);
    #[cfg(all(target_os = "linux", not(target_env = "ohos"), not(feature = "tui")))]
    floom_platform::register_plugin("webview", linux::enable);
    #[cfg(all(target_os = "windows", not(feature = "tui")))]
    floom_platform::register_plugin("webview", windows::enable);
    #[cfg(all(target_os = "ios", not(feature = "tui")))]
    floom_platform::register_plugin("webview", ios::enable);
    #[cfg(all(target_os = "android", not(feature = "tui")))]
    floom_platform::register_plugin("webview", android::enable);
    #[cfg(all(target_env = "ohos", not(feature = "tui")))]
    floom_platform::register_plugin("webview", ohos::enable);
    #[cfg(all(
        any(target_arch = "wasm32", target_os = "unknown"),
        not(feature = "tui")
    ))]
    floom_platform::register_plugin("webview", web::enable);
    #[cfg(feature = "tui")]
    floom_platform::register_plugin("webview", tui::enable);
}
