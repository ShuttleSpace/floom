//! Floem 2.0 demo app — one crate, one gallery, one command per platform.
//!
//! The tree is described once ([`demos`]) and mounted by three backends that
//! differ only in how frames are driven:
//!
//! ```text
//!                       demos::gallery_root()
//!                              │
//!        ┌─────────────────────┼─────────────────────┐
//!        ▼                     ▼                     ▼
//!   headless             desktop (macos)        android
//!   WidgetHost::frame()  winit + vello GPU      native-activity + vello GPU
//!   (CI, no window)      (a real OS window)     (a real APK)
//! ```
//!
//! The platform features on this crate are the only thing that varies, so
//! `scripts/build.sh floem2-demo <platform>` has a single crate to point at:
//!
//! ```sh
//! cargo run -p floem2-demo                          # headless (default)
//! just build floem2-demo macos                      # a real window
//! just build floem2-demo android                    # an installable APK
//! ```
//!
//! `ios`, `ohos` and `web` turn their backend crate on so the demo can be
//! `cargo check`ed for that target; the v2 backends themselves are still
//! in-memory skeletons (see `docs/2.0/EXAMPLES.md`), so no artifact is
//! produced for them yet.

pub mod demos;

/// Pump a [`floem_app::WidgetHost`] by hand and assert every demo mounted and
/// painted. The default feature, and the mode `scripts/run-golden-examples.sh`
/// runs.
#[cfg(feature = "headless")]
pub mod headless;

/// A real OS window: winit + vello over wgpu (`floem-platform-desktop`).
#[cfg(all(feature = "macos", not(target_os = "android")))]
pub mod desktop;

/// The Android entry point (`android_main`), loaded from the `cdylib` by
/// `native-activity`.
#[cfg(all(feature = "android", target_os = "android"))]
pub mod android;
