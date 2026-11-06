// SPDX-License-Identifier: MIT
//! The mobile platform adapter: the Android JNI bridge plus the window-level
//! packages (clipboard, soft keyboard, safe-area insets, system chrome).
//!
//! Capabilities (battery, connectivity, sensors, …) are plugins, never here — a
//! capability in this crate would be linked into every mobile app that uses the
//! adapter, and the architecture gate enforces that direction. Layering and the
//! packages pattern: `docs/api/mobile-platform-layering.md`.
//!
//! Dispatch pattern, vibration FFI and the runtime platform enum are adapted from
//! `gpui-mobile` (Apache-2.0).

// The JNI bridge is only meaningful on Android; it pulls in the `jni` crate,
// which is a target-gated dependency.
#[cfg(target_os = "android")]
pub mod jni;
pub mod packages;
pub mod target_platform;
