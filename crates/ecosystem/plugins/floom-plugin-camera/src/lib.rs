//! A **native camera**, contributed by a plugin.
//!
//! The capability is a view the framework does not paint: a camera preview the
//! platform's own camera API renders live frames into, driven from the framework
//! side through the provider ABI ([`floom_native_ffi::floom_native`]).
//!
//! # Layout
//!
//! The plugin is split by platform. Each platform owns a directory at the crate
//! root — `android/ ios/ windows/ macos/ linux/ ohos/ web/ tui/` — holding its
//! native half, and `src/<platform>.rs` holds that half's Rust FFI bridge. The
//! shared C-ABI contract lives in `common/floom_plugin_camera.h`. This file
//! dispatches the public surface to whichever platform is being compiled.
//!
//! - `android/` — Java half (`FloomCameraView`), driven through JNI; the Rust
//!   half owns the `NativeViewProvider`.
//! - `macos/` `ios/` `windows/` `linux/` `ohos/` — a C/Objective-C/C++ native
//!   half that registers a `FloomNativeProviderV1` vtable; the Rust half calls
//!   it through `extern "C"`.
//! - `web/` — pure Rust (`web-sys`): `getUserMedia` into a `<video>` element.
//! - `tui/` — unsupported: a terminal has no camera.

pub mod common;

#[cfg(all(target_os = "android", not(feature = "tui")))]
mod android;
#[cfg(all(target_os = "ios", not(feature = "tui")))]
mod ios;
#[cfg(all(target_os = "linux", not(feature = "tui")))]
mod linux;
#[cfg(all(target_os = "macos", not(feature = "tui")))]
mod macos;
#[cfg(all(target_os = "ohos", not(feature = "tui")))]
mod ohos;
#[cfg(feature = "tui")]
mod tui;
#[cfg(all(target_arch = "wasm32", not(feature = "tui")))]
mod web;
#[cfg(all(target_os = "windows", not(feature = "tui")))]
mod windows;

#[cfg(not(any(
    target_os = "android",
    target_os = "ios",
    target_os = "macos",
    target_os = "windows",
    target_os = "linux",
    target_os = "ohos",
    target_arch = "wasm32",
    feature = "tui",
)))]
mod unsupported;

/// The plugin's version.
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// The plugin's capability id.
pub fn capability() -> i32 {
    common::CAPABILITY
}

// A terminal backend (`feature = "tui"`) has no camera, so it overrides any
// platform half: the `tui` module is the only one re-exported when the feature
// is on, and the platform halves are not even compiled.
#[cfg(all(target_os = "android", not(feature = "tui")))]
pub use android::{disable, enable, provider, register};
#[cfg(all(target_os = "ios", not(feature = "tui")))]
pub use ios::{disable, enable, provider, register};
#[cfg(all(target_os = "linux", not(feature = "tui")))]
pub use linux::{disable, enable, provider, register};
#[cfg(all(target_os = "macos", not(feature = "tui")))]
pub use macos::{disable, enable, provider, register};
#[cfg(all(target_os = "ohos", not(feature = "tui")))]
pub use ohos::{disable, enable, provider, register};
#[cfg(feature = "tui")]
pub use tui::{disable, enable, provider, register};
#[cfg(all(target_arch = "wasm32", not(feature = "tui")))]
pub use web::{disable, enable, provider, register};
#[cfg(all(target_os = "windows", not(feature = "tui")))]
pub use windows::{disable, enable, provider, register};

#[cfg(not(any(
    target_os = "android",
    target_os = "ios",
    target_os = "macos",
    target_os = "windows",
    target_os = "linux",
    target_os = "ohos",
    target_arch = "wasm32",
    feature = "tui",
)))]
pub use unsupported::{disable, enable, provider, register};
