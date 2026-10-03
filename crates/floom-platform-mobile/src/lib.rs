//! Mobile OS-capability packages for Floom 2.0.
//!
//! This crate follows the *packages* pattern popularised by
//! [`gpui-mobile`](https://github.com/itsbalamurali/gpui-mobile): each mobile
//! capability (vibration, camera, location, sensors, …) lives in its own
//! module under [`packages`] with three files:
//!
//! ```text
//! packages/<name>/
//!   mod.rs     — the public API + `#[cfg]` dispatch to the right backend
//!   ios.rs     — the iOS (objc2 / UIKit) implementation
//!   android.rs — the Android (JNI / NDK) implementation
//! ```
//!
//! The host (non-mobile) `#[cfg]` arm in `mod.rs` returns
//! `Err("… only available on iOS and Android")`, so the crate still compiles
//! and is a no-op on desktop/web. Capability modules are feature-gated in the
//! manifest so a shipping app only pulls the backends it actually uses.
//!
//! This is a *capability* layer, deliberately separate from the per-OS window
//! / event backends in `floom-platform-{android,ios,harmony,…}`. Those own
//! the winit glue; this crate owns the device APIs that an app calls once a
//! window exists.
//!
//! # Why a separate crate, and not a reorganisation of the whole tree?
//!
//! Floom's workspace is already a mature, layered graph (L0–L9; see
//! `scripts/check-architecture-deps.sh`). The packages pattern is a *local*
//! convention for mobile OS glue, not a replacement for that layering. We add
//! one focused crate that follows it rather than reshaping every directory.
//! Reorganising the entire project into `packages/*` would discard a better,
//! intentional structure and re-break the architecture gate.
//!
//! # Attribution
//!
//! The dispatch pattern, the vibration FFI and the runtime platform enum are
//! adapted from `gpui-mobile`, used under its Apache-2.0 option.

// The JNI bridge is only meaningful on Android; it pulls in the `jni` crate,
// which is a target-gated dependency. On iOS the vibration backend uses objc2,
// and on the host the capability APIs are inert no-ops, so neither needs it.
#[cfg(target_os = "android")]
pub mod jni;
pub mod packages;
pub mod target_platform;
