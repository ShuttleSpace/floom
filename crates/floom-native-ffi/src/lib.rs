//! The C ABI for native composition providers.
//!
//! This crate is the **boundary** between the framework and a native
//! implementation of a native view — an `NSView`, an Android `SurfaceView`,
//! a `WKWebView`, a camera preview. It exists because the provider contract
//! in [`floom_native`] is a Rust trait, and a provider written in Swift, Java,
//! C++, C# or ArkTS cannot implement a Rust trait.
//!
//! # The shape of the contract
//!
//! One header, `include/floom_native_provider.h`, is the source of truth. It
//! declares a `FloomNativeProviderV1` — a versioned struct of function
//! pointers — which the **native side implements** and the **Rust side
//! calls**. [`CProvider`] is the thin adapter that presents such a provider
//! as an ordinary Rust [`NativeViewProvider`], so the controller's lifecycle
//! invariants apply to it exactly as they apply to a Rust provider.
//!
//! # Why a vtable and not named symbols
//!
//! A provider is an **instance** — one per window — so the contract has to
//! carry a `self` pointer, and a struct of function pointers is what carries
//! one without a global registry. It is also versioned in place: the ABI
//! version lives in the struct, so the Rust side can ask a provider it did
//! not compile what it was built against, and refuse it.
//!
//! # Why this is a separate crate
//!
//! [`floom_native`] is `#![forbid(unsafe_code)]`: it is the crate a
//! platform backend implements the ABI against, and it should not have to be
//! trusted with a raw pointer. The unsafe lives here instead — in one crate,
//! at one boundary, where it can be read and checked.
//!
//! # What is not in v1
//!
//! `snapshot` and `take_captures`. Freezing a participant into a texture
//! needs the host's own capture path, and a provider whose host cannot
//! capture is the common and honest case; the controller already gates the
//! call on `capabilities().snapshot`, so a C provider that declares
//! `snapshot: false` is simply never asked.

#![deny(unsafe_op_in_unsafe_fn)]

pub mod abi;
pub mod provider;

pub use abi::{
    ABI_VERSION, FloomNativeCapabilities, FloomNativeDeviceRect, FloomNativeProviderV1,
    FloomNativeSlot, FloomNativeState,
};
pub use provider::{CProvider, CProviderError, check};

/// The header this crate's types mirror.
///
/// A plugin's build copies it next to its own sources so the native half can
/// `#include` it.
pub const HEADER: &str = include_str!("../include/floom_native_provider.h");
