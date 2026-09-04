//! Cursor / selection model (byte offsets into UTF-8).

/// Caret affinity when landing on a bidi boundary (forward-compatible).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Affinity {
    #[default]
    Downstream,
    Upstream,
}

/// Inclusive-exclusive selection; caret when `anchor == head`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
    pub affinity: Affinity,
}

impl Default for Selection {
    fn default() -> Self {
        Self::caret(0)
    }
}

impl Selection {
    pub fn caret(offset: usize) -> Self {
        Self {
            anchor: offset,
            head: offset,
            affinity: Affinity::Downstream,
        }
    }

    pub fn range(anchor: usize, head: usize) -> Self {
        Self {
            anchor,
            head,
            affinity: Affinity::Downstream,
        }
    }

    pub fn is_caret(self) -> bool {
        self.anchor == self.head
    }

    pub fn start(self) -> usize {
        self.anchor.min(self.head)
    }

    pub fn end(self) -> usize {
        self.anchor.max(self.head)
    }

    pub fn len(self) -> usize {
        self.end() - self.start()
    }

    pub fn clamp(self, len: usize) -> Self {
        Self {
            anchor: self.anchor.min(len),
            head: self.head.min(len),
            affinity: self.affinity,
        }
    }
}
