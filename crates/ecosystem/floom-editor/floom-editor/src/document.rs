//! The document: the text being edited.
//!
//! v1's `Document` trait carried the editor's *view* state as well as its
//! text — the preedit, the phantom text, the screen-line computation, the
//! command dispatch — because the editor's `View` was the only thing that
//! could answer them. In 2.0 the widget is a plain object with no ambient
//! scope to reach into, so the document is only the *text* and the edits that
//! change it. Everything else the editor needs, it asks the engine for.

use std::sync::Mutex;

use floom_editor_core::buffer::Buffer;
use floom_editor_core::buffer::rope_text::{RopeText, RopeTextVal};
use floom_editor_core::editor::EditType;
use floom_editor_core::selection::Selection;
use floom_editor_core::xi_rope::Rope;

/// A document: text that can be edited.
///
/// The trait is `Send + Sync` because the editor's state is shared between the
/// widget and whoever holds a handle to it, and a shared handle has to be
/// shareable.
pub trait Document: Send + Sync {
    /// The whole text.
    fn text(&self) -> Rope;

    /// A counter that changes whenever the text does.
    ///
    /// The editor's layout cache is keyed on it: a line that was laid out at
    /// one revision and is asked for at another is a line whose glyphs do not
    /// match its text.
    fn revision(&self) -> u64;

    /// Replaces the selection's regions with `content`.
    fn edit(&self, selection: &Selection, content: &str);

    /// Replaces the whole text.
    fn set_text(&self, text: Rope);

    /// Undoes the last edit. Reports whether there was one.
    fn undo(&self) -> bool;

    /// Redoes the last undone edit. Reports whether there was one.
    fn redo(&self) -> bool;

    /// The text, with the helpers the editor needs.
    ///
    /// The default builds a value around [`Document::text`], which is what
    /// every document that owns a rope can do.
    fn rope_text(&self) -> RopeTextVal {
        RopeTextVal::new(self.text())
    }
}

/// A document backed by the editor core's rope buffer.
///
/// The buffer is the model the editor core already had: a rope, an undo
/// history, and the line-ending normalisation that goes with them. Wrapping it
/// rather than reimplementing it is what keeps the port's risk in the view
/// layer.
pub struct TextDocument {
    buffer: Mutex<Buffer>,
}

impl Default for TextDocument {
    fn default() -> Self {
        Self::new("")
    }
}

impl TextDocument {
    /// A document holding `text`.
    pub fn new(text: impl Into<Rope>) -> Self {
        Self {
            buffer: Mutex::new(Buffer::new(text)),
        }
    }

    /// The buffer, for a caller that wants the model itself.
    ///
    /// The lock is a poisoned-mutex-tolerant one: a caller that poisoned the
    /// mutex is as stuck as the editor is, and panicking over it would take
    /// down a paint pass.
    pub fn with_buffer<R>(&self, f: impl FnOnce(&Buffer) -> R) -> R {
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        f(&buffer)
    }

    /// The buffer, mutably.
    pub fn with_buffer_mut<R>(&self, f: impl FnOnce(&mut Buffer) -> R) -> R {
        let mut buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut buffer)
    }
}

impl Document for TextDocument {
    fn text(&self) -> Rope {
        self.with_buffer(|buffer| buffer.text().clone())
    }

    fn revision(&self) -> u64 {
        self.with_buffer(|buffer| buffer.rev())
    }

    fn edit(&self, selection: &Selection, content: &str) {
        self.with_buffer_mut(|buffer| {
            buffer.edit([(selection, content)], EditType::InsertChars);
        });
    }

    fn set_text(&self, text: Rope) {
        self.with_buffer_mut(|buffer| {
            buffer.reload(text, false);
        });
    }

    fn undo(&self) -> bool {
        self.with_buffer_mut(|buffer| buffer.do_undo().is_some())
    }

    fn redo(&self) -> bool {
        self.with_buffer_mut(|buffer| buffer.do_redo().is_some())
    }
}
