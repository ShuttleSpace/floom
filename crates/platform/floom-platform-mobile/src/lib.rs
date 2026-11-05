// SPDX-License-Identifier: MIT
//! The mobile platform layer for Floom 2.0.
//!
//! This crate is a **platform adapter**, and it is deliberately small. It
//! carries two things:
//!
//! * the **JNI bridge** — the JavaVM, the activity, and the helpers the
//!   platform's own packages use to reach Android. It is meaningful only on
//!   Android, and the `jni` crate is a target-gated dependency, so nothing
//!   pulls it on any other target.
//! * the **packages** that are the platform's own business rather than a
//!   capability: the clipboard and the soft keyboard (the two of
//!   `floom-platform`'s three traits that mobile implements) and the two
//!   facts about a window — the safe-area insets and the system chrome.
//!
//! # What is not here
//!
//! Every *capability* — the battery, the connectivity, the device's identity,
//! the biometrics, the location, the notifications, the sensors, the share
//! sheet, the haptics — is a **plugin**: one crate under
//! `crates/floom-plugin-<capability>/`, opt-in by dependency, reaching the OS
//! through a native host library.
//!
//! The split is not cosmetic, and it is the difference between an SDK that
//! stays small and one that does not. A capability here would be compiled and
//! linked into every mobile app that depends on the platform adapter, whether
//! it wanted the capability or not. A capability in its own crate is linked
//! only by the apps that ask for it.
//!
//! The rule is enforced rather than merely documented: the architecture gate
//! refuses a dependency from a `floom-platform-*` crate to a
//! `floom-plugin-*` crate, because the platform reaching *up* to a plugin is
//! the direction that would put the capability back in.
//!
//! # The packages pattern
//!
//! What is left here follows the *packages* pattern popularised by
//! [`gpui-mobile`](https://github.com/itsbalamurali/gpui-mobile): each
//! package is a module with three files —
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
//! and is a no-op on desktop and web.
//!
//! # Attribution
//!
//! The dispatch pattern, the vibration FFI and the runtime platform enum are
//! adapted from `gpui-mobile`, used under its Apache-2.0 option.

// The JNI bridge is only meaningful on Android; it pulls in the `jni` crate,
// which is a target-gated dependency.
#[cfg(target_os = "android")]
pub mod jni;
pub mod packages;
pub mod target_platform;
