//! Scrolling vocabulary.
//!
//! `floom-core` does not depend on a layout engine, so this module expresses
//! scrolling *intent* — how far a viewport has been scrolled, and how far it is
//! allowed to go — rather than computing a scrollbar or translating a subtree.
//! [`crate::Widget::children`] hands a [`ScrollOffset`] up to whichever layer owns
//! the tree, and that layer applies it after layout.
//!
//! ## Why the offset is applied after layout rather than during it
//!
//! The layout engine computes boxes; a scroll offset is not a box. Folding it
//! into a margin would make the scrolled content *occupy* space, so the parent
//! would grow by the scroll distance and the viewport would never stop moving.
//! Folding it into a relative position would make the offset a sibling concern
//! that a flex layout is free to recompute away.
//!
//! So it is a post-layout transform on absolute coordinates, accumulated down
//! the tree exactly the way a parent offset already is. A scroll container
//! contributes its offset to its descendants and to nothing else, which is what
//! makes two independent scroll containers compose instead of fighting.
//!
//! ## Why clamping lives here and not in the widget
//!
//! A widget knows its viewport but not its content's size until layout has run,
//! and the content's size changes for reasons the widget never sees (a sibling
//! grows, a font loads). Clamping therefore has to happen where both numbers are
//! known — after layout — and the rule belongs to the vocabulary so that the
//! engine and every scrolling widget agree on it.

/// How far a scroll viewport has been scrolled, in logical pixels.
///
/// Positive values scroll *towards* the end: `y > 0` means the content has moved
/// up, revealing what is below. This matches the sign convention of a mouse
/// wheel delta in every platform backend floem talks to, so a backend can add its
/// delta and be done.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScrollOffset {
    pub x: f64,
    pub y: f64,
}

impl ScrollOffset {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// This offset applied on top of `parent`.
    ///
    /// Offsets accumulate down the tree, so a scroll container inside another
    /// scroll container moves its content by both.
    pub fn then(self, parent: Self) -> Self {
        Self {
            x: self.x + parent.x,
            y: self.y + parent.y,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.x == 0.0 && self.y == 0.0
    }
}

impl From<(f64, f64)> for ScrollOffset {
    fn from((x, y): (f64, f64)) -> Self {
        Self { x, y }
    }
}

/// The largest offset a viewport of `viewport` may take over content of
/// `content`.
///
/// Returns [`ScrollOffset::ZERO`] when the content fits: there is nothing to
/// scroll, and a non-zero offset would leave a gap at the far end with no way to
/// scroll it back.
///
/// Both arguments are lengths, not offsets. Each axis is handled on its own: a
/// horizontal-only scroll container has `viewport.y >= content.y` and therefore
/// must not be prevented from scrolling vertically by that, and vice versa.
pub fn max_offset(viewport: (f64, f64), content: (f64, f64)) -> ScrollOffset {
    ScrollOffset {
        x: (content.0 - viewport.0).max(0.0),
        y: (content.1 - viewport.1).max(0.0),
    }
}

/// Confines an offset to what a viewport of `viewport` may show of `content`.
///
/// The clamp is applied to both ends. Clamping only the upper end is the usual
/// shortcut and it is wrong in a way that is easy to miss: shrink a scrolled
/// list and the offset stays where it was, so the content ends up scrolled
/// *past* its own start and the first rows become unreachable. The same applies
/// when content shrinks under a scrolled viewport for any other reason.
pub fn clamp_offset(
    offset: ScrollOffset,
    viewport: (f64, f64),
    content: (f64, f64),
) -> ScrollOffset {
    let limit = max_offset(viewport, content);
    ScrollOffset {
        x: offset.x.clamp(0.0, limit.x),
        y: offset.y.clamp(0.0, limit.y),
    }
}
