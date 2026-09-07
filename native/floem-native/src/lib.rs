//! Native composition participant pool for Floem 2.0.
//!
//! This crate is introduced as a **Phase 12.2 precursor**. The full Native ABI
//! (`NativeViewProvider` / `NativeViewController` / `NativeCapabilities`) lands
//! in Phase 16, but virtualization already needs a recycling primitive for
//! native views. `NativeViewPool` keyed by a stable data key keeps the live
//! native view count bounded regardless of list length.
//!
//! Depends only on `floem-types` (for `NativeViewId`) and `floem-virtual`.

#![forbid(unsafe_code)]

pub mod pool;

pub use pool::{NativeSlot, NativeViewPool};
