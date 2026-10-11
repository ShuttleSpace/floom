//! The `floom-demo` binary.
//!
//! Exactly one backend is compiled into `main`, chosen by the crate's features:
//!
//! | features              | `main` runs            |
//! | --------------------- | ---------------------- |
//! | `headless` (default)  | [`headless::run`]      |
//! | `macos`               | [`desktop::launch`]    |
//! | `android` (target)    | nothing — the `cdylib` exports `android_main` instead |
//!
//! The Android bin is still *built* (the default `headless` feature satisfies
//! its `required-features`), it just never runs: `native-activity` calls the
//! shared library's `android_main`, not this executable.

#[cfg(all(feature = "macos", not(target_os = "android")))]
fn main() {
    floom_demo::desktop::launch();
}

#[cfg(all(not(feature = "macos"), feature = "headless"))]
fn main() {
    floom_demo::headless::run();
}

#[cfg(all(not(feature = "macos"), not(feature = "headless")))]
fn main() {
    eprintln!(
        "floom-demo: no runnable backend in this feature set — \
         enable `headless` (CI), `macos` (desktop window) or build the crate's \
         `cdylib` for Android/iOS/OHOS"
    );
    std::process::exit(1);
}
