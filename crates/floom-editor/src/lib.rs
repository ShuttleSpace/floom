//! Shared document / selection / undo primitives for text fields and editors.

mod document;
mod selection;
mod undo;

pub use document::{EditOp, TextDocument, Transaction};
pub use selection::{Affinity, Selection};
pub use undo::UndoManager;
