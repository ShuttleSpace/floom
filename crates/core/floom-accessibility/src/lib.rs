// SPDX-License-Identifier: MIT
//! Semantics tree and accessibility adapter ABI for Floom 2.0.
//!
//! Widgets describe their accessibility semantics through a retained
//! [`SemanticsTree`] keyed by [`ElementId`](floom_types::ElementId). The runtime's `SEMANTICS` dirty
//! flag routes into [`SemanticsTree::mark_semantics_dirty`]; the frame loop
//! drains [`SemanticsTree::collect_updates`] into a [`SemanticsDiff`] that
//! platform adapters ([`AccessibilityAdapter`]) translate to OS a11y APIs.
//!
//! This crate is deliberately OS-free: it depends only on `floom-types`, so
//! platform backends can depend on it without creating a dependency cycle.

mod adapter;
mod semantics;

pub use adapter::{
    AccessibilityAdapter, AccessibilityCapabilities, AccessibilityFrame,
    CapabilityLimitedAccessibilityAdapter,
};
pub use semantics::{
    SemanticsAction, SemanticsContext, SemanticsDiff, SemanticsNode, SemanticsRole, SemanticsState,
    SemanticsTree,
};
