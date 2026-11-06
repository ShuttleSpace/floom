use floom_core::PaintContext;
use floom_types::{Color, Rect};
use floom_widgets::stroke_rounded_outline;

/// The ring drawn around a focused control.
///
/// `Option<FocusRing>` rather than a `width` of `0.0`, because "no ring" and "a
/// ring of no thickness" are different answers and only one of them means the
/// control answers focus some other way.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusRing {
    /// The ring's thickness. The three kits all wrote `2.0`.
    pub width: f32,
    /// The ring's ink. All three read the theme's `primary`.
    pub color: Color,
    /// The space between the control's edge and the ring's inner edge.
    ///
    /// `0.0` draws the ring flush against the control, which is what all three
    /// wrote -- Compose's `outline()` has no inset and the kits used it directly.
    pub gap: f32,
    /// The ring's corner radius.
    ///
    /// `None` follows the control's own radius plus the gap and the width, which
    /// is what a rounded control wants: a square ring around a rounded field
    /// reads as two shapes. `Some(r)` pins it, for a control whose ring should
    /// not follow -- a round button wants a circle, and its own radius is already
    /// the answer, so the flag exists for the cases where it is not.
    pub radius: Option<f32>,
}

impl FocusRing {
    /// Draws the ring around `bounds`, which is the control's own box.
    ///
    /// Drawn *outside* `bounds`, so it is visible rather than clipped by the
    /// layout box -- a control laid out to its own size has no room inside
    /// itself for a ring that means "this has keyboard focus".
    ///
    /// `corner_radius` is the control's own corner radius, which `radius: None`
    /// follows. Passing it in rather than reading it from a style is what lets
    /// one painter serve a rounded text field and a round radio.
    pub fn paint(&self, cx: &mut dyn PaintContext, bounds: Rect, corner_radius: f32) {
        if self.width <= 0.0 {
            return;
        }
        let width = f64::from(self.width);
        let gap = f64::from(self.gap);
        let outside = Rect {
            x0: bounds.x0 - gap - width,
            y0: bounds.y0 - gap - width,
            x1: bounds.x1 + gap + width,
            y1: bounds.y1 + gap + width,
        };
        let radius = self
            .radius
            .map_or_else(|| f64::from(corner_radius) + gap + width, |r| f64::from(r));
        stroke_rounded_outline(cx, outside, radius, width, self.color);
    }
}
