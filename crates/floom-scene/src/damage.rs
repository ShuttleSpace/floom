//! Damage region tracking for partial redraws.

use floem_types::Rect;
use smallvec::SmallVec;

/// Accumulated damage for a scene update transaction.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DamageRegion {
    /// When true, the whole surface must be redrawn.
    full_frame: bool,
    /// Bounded list of dirty rectangles (logical pixels).
    rects: SmallVec<[Rect; 8]>,
}

impl DamageRegion {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        !self.full_frame && self.rects.is_empty()
    }

    pub fn is_full_frame(&self) -> bool {
        self.full_frame
    }

    pub fn rects(&self) -> &[Rect] {
        if self.full_frame { &[] } else { &self.rects }
    }

    pub fn clear(&mut self) {
        self.full_frame = false;
        self.rects.clear();
    }

    pub fn mark_full_frame(&mut self) {
        self.full_frame = true;
        self.rects.clear();
    }

    pub fn union_rect(&mut self, rect: Rect) {
        if self.full_frame {
            return;
        }
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return;
        }
        if self.rects.len() >= 8 {
            // Spill to full-frame rather than unbounded growth.
            self.mark_full_frame();
            return;
        }
        self.rects.push(rect);
    }

    pub fn union(&mut self, other: &DamageRegion) {
        if other.full_frame {
            self.mark_full_frame();
            return;
        }
        for rect in &other.rects {
            self.union_rect(*rect);
        }
    }
}
