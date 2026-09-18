//! The runtime half of hot reload (Phase 22.3): apply a UI patch to a *live*
//! retained tree without throwing away per-element state.
//!
//! The contract, in one sentence: **nodes are matched by their stable
//! [`NodeKey`], so a node the edit did not touch keeps its [`ElementId`] and
//! everything keyed by it** — styles, layout requests, event wiring, and any
//! state a layer stashed on the element (scroll offset, focus, …).
//!
//! The pipeline is the one the plan asks for: the editor saves a new IR
//! document, [`diff`] turns old → new into a [`UiPatch`] of named operations,
//! and [`LiveTree::apply`] runs them against a checkpoint. Any failing
//! operation rolls the whole tree back — a half-applied reload is worse than
//! none, because the user's screen would disagree with the source.
//!
//! Boundary note: the retained core stores id/kind/parent, so this crate's
//! patch surface covers structure, style sets, layout requests and event
//! wiring. Widget properties (a button's label, a slider's value) live in the
//! widget instances above this layer; a real host forwards those separately.

#![forbid(unsafe_code)]

mod diff;
mod live;
mod patch;

pub use diff::diff;
pub use live::{LiveTree, Snapshot};
pub use patch::{ApplyReport, PatchOp, RolledBack, UiPatch};

/// Re-exported so a caller can talk about keys without depending on
/// `floem-ui-ir` directly.
pub use floem_ui_ir::NodeKey;
