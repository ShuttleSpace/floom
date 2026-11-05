// SPDX-License-Identifier: MIT
//! Undo / redo stacks of inverted transactions.

use crate::document::{TextDocument, Transaction};

#[derive(Debug, Default)]
pub struct UndoManager {
    pub(crate) undo: Vec<Transaction>,
    pub(crate) redo: Vec<Transaction>,
}

impl UndoManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, inverse: Transaction) {
        self.undo.push(inverse);
        self.redo.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self, doc: &mut TextDocument) -> bool {
        let Some(tx) = self.undo.pop() else {
            return false;
        };
        let inverse = doc.apply(&tx);
        self.redo.push(inverse);
        true
    }

    pub fn redo(&mut self, doc: &mut TextDocument) -> bool {
        let Some(tx) = self.redo.pop() else {
            return false;
        };
        let inverse = doc.apply(&tx);
        self.undo.push(inverse);
        true
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}
