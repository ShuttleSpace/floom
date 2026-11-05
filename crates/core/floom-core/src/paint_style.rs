// SPDX-License-Identifier: MIT
//! What a widget declares it paints, in renderer-neutral terms.
//!
//! An inspector has to answer "why does this box look like that", and the only
//! thing that knows is the widget itself. By the time anyone else could look,
//! the scene is already flattened into draw ops, and a draw op does not say
//! whether a rect was a background, a divider or a focus ring.
//!
//! [`PaintStyle`] is that answer, in the smallest vocabulary that covers what
//! the engine's primitives actually declare. Every field is `Option` so that
//! "not set" stays distinguishable from "set to zero" — the same rule
//! [`LayoutRequest`](crate::LayoutRequest) follows, and for the same reason: a
//! zero radius and an absent radius are different boxes.
//!
//! Two limits are deliberate, and a readout built on this must respect them:
//!
//! - **A subset.** A widget that paints something this does not name — a
//!   vector path, a gradient's stops, a glyph run — reports nothing about it
//!   rather than an approximation. The readout shows the properties that are
//!   present; silence about the rest is the honest answer, not a bug.
//! - **A declaration, not a result.** This is what the widget said it would
//!   draw, computed with no renderer in the loop. A filter that shifted the
//!   colour on its way to the framebuffer does not show up here.
//!
//! `elevation` is the one field whose name is a compromise. The engine's
//! [`Shadow`] carries a blur, an offset *and* a spread, while the style
//! vocabulary has a single word for "how far the shadow reaches". A widget
//! reports its blur as the elevation, and says so at the call site rather than
//! letting the reader assume the other two lengths survived.

use floom_types::Color;

/// The paint properties a widget knows about itself.
///
/// See the module docs for what this is and, more importantly, what it is not.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PaintStyle {
    /// The box's fill, when it is one flat colour.
    ///
    /// A gradient reports `None` rather than its first stop: one stop is not
    /// the fill, and picking it would turn "I cannot say" into a wrong answer.
    pub background: Option<Color>,
    /// The colour of the widget's own text or glyphs, when it has them.
    pub foreground: Option<Color>,
    /// Layer opacity, `0.0..=1.0`.
    pub opacity: Option<f32>,
    /// Font size in logical pixels, for a widget that draws text.
    pub font_size: Option<f32>,
    /// Inner spacing, in logical pixels, when it is uniform on all four sides.
    ///
    /// A box with different insets per side reports `None`: one number cannot
    /// describe four.
    pub padding: Option<f32>,
    /// Corner radius in logical pixels.
    pub radius: Option<f32>,
    /// Outline thickness in logical pixels.
    pub border_width: Option<f32>,
    /// Outline colour. Reported beside [`Self::border_width`] because a width
    /// with no colour is not a border.
    pub border_color: Option<Color>,
    /// How far the box's shadow reaches, in logical pixels — the blur radius.
    ///
    /// This is the same quantity [`floom_style::StyleProps::elevation`] stores;
    /// the shadow's offset and spread are *not* reported. See the module docs.
    ///
    /// [`floom_style::StyleProps::elevation`]: https://docs.rs/floom-style
    pub elevation: Option<f32>,
}

impl PaintStyle {
    /// Whether every property is absent — a widget that declares no paint.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}
