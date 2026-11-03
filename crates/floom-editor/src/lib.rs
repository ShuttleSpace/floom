//! Shared document / selection / undo primitives for text fields and editors,
//! plus the [`EditorWidget`] that mounts a [`TextDocument`] into the widget tree.
//
mod document;
mod selection;
mod undo;
mod widget;

pub use document::{EditOp, TextDocument, Transaction};
pub use selection::{Affinity, Selection};
pub use undo::UndoManager;
pub use widget::EditorWidget;
