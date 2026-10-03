//! Rope-backed document with transactional edits and IME composition.

use lapce_xi_rope::Rope;

use crate::selection::Selection;
use crate::undo::UndoManager;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditOp {
    /// Replace `range` (byte offsets) with `text`.
    Replace {
        start: usize,
        end: usize,
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Transaction {
    pub ops: Vec<EditOp>,
}

impl Transaction {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn replace(start: usize, end: usize, text: impl Into<String>) -> Self {
        Self {
            ops: vec![EditOp::Replace {
                start,
                end,
                text: text.into(),
            }],
        }
    }

    pub fn insert(offset: usize, text: impl Into<String>) -> Self {
        Self::replace(offset, offset, text)
    }

    pub fn delete(start: usize, end: usize) -> Self {
        Self::replace(start, end, "")
    }
}

#[derive(Debug)]
pub struct TextDocument {
    rope: Rope,
    selection: Selection,
    /// Active IME preedit overlay (not yet committed into the rope).
    composition: Option<String>,
    undo: UndoManager,
}

impl Default for TextDocument {
    fn default() -> Self {
        Self::new()
    }
}

impl TextDocument {
    pub fn new() -> Self {
        Self {
            rope: Rope::default(),
            selection: Selection::caret(0),
            composition: None,
            undo: UndoManager::new(),
        }
    }

    pub fn from_str(s: &str) -> Self {
        let mut doc = Self::new();
        doc.rope = Rope::from(s);
        doc.selection = Selection::caret(s.len());
        doc
    }

    pub fn len(&self) -> usize {
        self.rope.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rope.is_empty()
    }

    pub fn text(&self) -> String {
        String::from(&self.rope)
    }

    pub fn selection(&self) -> Selection {
        self.selection
    }

    pub fn set_selection(&mut self, sel: Selection) {
        self.selection = sel.clamp(self.len());
    }

    pub fn composition(&self) -> Option<&str> {
        self.composition.as_deref()
    }

    /// Apply `tx` and return the inverse transaction (does not touch undo).
    pub fn apply(&mut self, tx: &Transaction) -> Transaction {
        let mut inverse = Transaction::new();
        for op in &tx.ops {
            match op {
                EditOp::Replace { start, end, text } => {
                    let start = (*start).min(self.len());
                    let end = (*end).min(self.len()).max(start);
                    let old = self.rope.slice_to_cow(start..end).into_owned();
                    let new_end = start + text.len();
                    self.rope.edit(start..end, text.as_str());
                    inverse.ops.push(EditOp::Replace {
                        start,
                        end: new_end,
                        text: old,
                    });
                    // Keep selection at end of inserted text for simple edits.
                    self.selection = Selection::caret(new_end);
                }
            }
        }
        // Inverse ops are recorded in apply order; undo should apply reverse order.
        inverse.ops.reverse();
        inverse
    }

    /// Apply and push inverse onto the undo stack.
    pub fn commit(&mut self, tx: Transaction) {
        let inverse = self.apply(&tx);
        self.undo.push(inverse);
        self.composition = None;
    }

    pub fn insert_text(&mut self, text: &str) {
        let sel = self.selection;
        self.commit(Transaction::replace(sel.start(), sel.end(), text));
    }

    pub fn delete_selection_or_backward(&mut self) {
        let sel = self.selection;
        if !sel.is_caret() {
            self.commit(Transaction::delete(sel.start(), sel.end()));
            return;
        }
        if sel.head == 0 {
            return;
        }
        let start = prev_char_boundary(&self.text(), sel.head);
        self.commit(Transaction::delete(start, sel.head));
    }

    pub fn undo(&mut self) -> bool {
        self.composition = None;
        let Some(tx) = self.undo.undo.pop() else {
            return false;
        };
        let inverse = self.apply(&tx);
        self.undo.redo.push(inverse);
        true
    }

    pub fn redo(&mut self) -> bool {
        self.composition = None;
        let Some(tx) = self.undo.redo.pop() else {
            return false;
        };
        let inverse = self.apply(&tx);
        self.undo.undo.push(inverse);
        true
    }

    /// IME preedit: store composition without committing.
    pub fn set_ime_preedit(&mut self, text: &str) {
        self.composition = Some(text.to_string());
    }

    /// IME commit: insert composition (or provided text) at selection.
    pub fn ime_commit(&mut self, text: Option<&str>) {
        let insert = text
            .map(str::to_string)
            .or_else(|| self.composition.clone())
            .unwrap_or_default();
        self.composition = None;
        if !insert.is_empty() {
            self.insert_text(&insert);
        }
    }

    pub fn clear_ime(&mut self) {
        self.composition = None;
    }
}

fn prev_char_boundary(s: &str, offset: usize) -> usize {
    if offset == 0 {
        return 0;
    }
    let mut i = offset - 1;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}
