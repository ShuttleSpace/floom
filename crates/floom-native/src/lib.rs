//! Native composition for Floem 2.0.
//!
//! A *participant* is a view the framework does not paint — an `NSView`, an
//! `HWND`, an Android `SurfaceView`, a KMS plane. This crate holds everything
//! about them that does not need an OS: the slot pool that keeps their count
//! bounded in virtualized lists, the provider ABI a platform backend
//! implements, and the controller that enforces their lifecycle.
//!
//! It is a **leaf crate**: it depends on `floem-types` and `floem-virtual` and
//! nothing else, so a platform backend can use the ABI without dragging in the
//! framework. Host-side composition lives in `floem-app`; OS bridges live in
//! the platform crates (decision D5 in the Phase 16 plan).
//!
//! Nothing here opens a window or creates a view. The phase's last mile is
//! device work — see `docs/superpowers/plans/2026-09-15-phase-16-native-composition-plan.md`.

#![forbid(unsafe_code)]

pub mod abi;
pub mod controller;
pub mod fake;
pub mod pool;

pub use abi::{
    ClipSupport, DeviceRect, InputMode, InputModes, NativeCapabilities, NativeCompositionMode,
    NativeError, NativeSnapshot, NativeState, NativeViewKind, NativeViewProvider, to_device,
};
// Re-exported because they are part of the ABI's vocabulary: a provider
// implementing `NativeViewProvider` needs them and should not have to guess
// which crate they came from.
pub use floem_types::{NativeViewId, Rect, ResourceId};

pub use controller::NativeViewController;
pub use fake::{FakeNativeProvider, NativeCommand, NativeCommandLog};
pub use pool::{NativeSlot, NativeViewPool};
