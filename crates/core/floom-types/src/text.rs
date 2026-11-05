// SPDX-License-Identifier: MIT
//! Text appearance: weight, family, and the styled-text description.
//!
//! Renderer-neutral by construction, like everything else in `floom-types`: the
//! nine CSS weights are named here and the lowering layer maps them onto whatever
//! the shaper wants (`fontique::FontWeight::new(500.0)` at the bottom of the
//! stack). Nothing in this module knows a font exists.

use crate::color::Color;

/// How heavy a run of text is drawn.
///
/// The nine CSS weights, named as CSS names them. floom-ui already writes
/// `FontWeight::Medium` and `FontWeight::SemiBold` at 68 call sites, so the
/// variant names are chosen to match its call sites rather than to be tidy.
///
/// [`Normal`] is the default and is what every text path that does not ask for a
/// weight gets — which is why it is the `Default` rather than a value every
/// constructor has to be handed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FontWeight {
    /// 100.
    Thin,
    /// 200.
    ExtraLight,
    /// 300.
    Light,
    /// 400. The default.
    #[default]
    Normal,
    /// 500.
    Medium,
    /// 600.
    SemiBold,
    /// 700.
    Bold,
    /// 800.
    ExtraBold,
    /// 900.
    Black,
}

impl FontWeight {
    /// The CSS numeric weight, `100..=900`.
    ///
    /// The escape hatch for a design system that names a weight this enum does
    /// not — a variable font's `450`, say. It is deliberately *not* the only way
    /// in: a caller reaching for it on every text run is a caller who wants a
    /// variant added here instead.
    pub const fn to_value(self) -> u16 {
        match self {
            Self::Thin => 100,
            Self::ExtraLight => 200,
            Self::Light => 300,
            Self::Normal => 400,
            Self::Medium => 500,
            Self::SemiBold => 600,
            Self::Bold => 700,
            Self::ExtraBold => 800,
            Self::Black => 900,
        }
    }

    /// The nearest named weight to a CSS numeric one.
    ///
    /// Rounds to the nearest hundred and clamps, so `450` is `Medium` and `1000`
    /// is `Black`. Used when lowering an arbitrary number onto this vocabulary.
    pub fn from_value(value: u16) -> Self {
        match value.clamp(100, 900).div_ceil(100) {
            1 => Self::Thin,
            2 => Self::ExtraLight,
            3 => Self::Light,
            4 => Self::Normal,
            5 => Self::Medium,
            6 => Self::SemiBold,
            7 => Self::Bold,
            8 => Self::ExtraBold,
            _ => Self::Black,
        }
    }

    /// Whether this is the default weight — the question every cache key needs,
    /// because it is the difference between "shaped normally" and "shaped
    /// specially" and getting it wrong paints bold text with regular glyphs.
    pub const fn is_normal(self) -> bool {
        matches!(self, Self::Normal)
    }
}

/// A typeface request: one of the platform's generic families, or a family by
/// name.
///
/// The five generic families are the ones every platform resolves, so a caller
/// that does not care about the exact face can say "monospace" and get the
/// platform's idea of it. A named family is a request, not a guarantee: a
/// platform that does not have it falls back, and the fallback is the generic
/// family underneath it in the stack.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum FontFamily {
    /// The platform's default serif.
    Serif,
    /// The platform's default sans-serif.
    #[default]
    SansSerif,
    /// The platform's default monospace face.
    Monospace,
    /// The platform's default cursive face.
    Cursive,
    /// The platform's default fantasy face.
    Fantasy,
    /// A family by name, such as `"Fira Code"`.
    Named(String),
}

/// How a run of text is drawn.
///
/// This is the *appearance* of a run and nothing else: the size, the weight and
/// the ink. The typeface is a property of the whole layout rather than of a
/// range, because a line that mixed typefaces mid-word is a mistake, and
/// because the family is what the fallback stack is built around.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TextStyle {
    /// The size of the run, in logical pixels.
    pub size: f32,
    /// How heavy the run is drawn.
    pub weight: FontWeight,
    /// The ink the run is drawn in.
    pub color: Color,
}

/// A range of text that differs from the layout's base style.
///
/// Ranges are byte ranges into the *source* text, so a caller that knows the
/// document can describe a highlighted word without knowing anything about
/// layout.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextSpan {
    /// The byte range, into the source text.
    pub range: std::ops::Range<usize>,
    /// How this range is drawn.
    pub style: TextStyle,
}

/// Whether a layout breaks lines, and where.
///
/// The two cases are the ones a text layout actually has to distinguish, and
/// they are not the same choice: `Word` breaks at word boundaries *and* breaks
/// a word too long to fit, which is what a code editor wants, because a long
/// identifier must not run off the edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextWrap {
    /// One layout line per source line: no breaking at all.
    None,
    /// Break at word boundaries, and inside a word that does not fit.
    #[default]
    Word,
}

/// A styled text: the source text, the style it starts with, and the ranges
/// that differ from it.
///
/// This is the description a layout is built from, and it is deliberately
/// *data* rather than a shaped result: the same description can be laid out for
/// geometry and painted for ink, and the two agree because they are the same
/// description. A caller that laid out one way and painted another would put the
/// caret somewhere the pixels are not.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StyledText {
    /// The source text. Byte offsets in [`TextSpan::range`] index into this.
    pub text: String,
    /// The style every byte starts with.
    pub base: TextStyle,
    /// The ranges that differ from [`Self::base`].
    pub spans: Vec<TextSpan>,
    /// The typeface the whole layout is set in.
    pub family: FontFamily,
    /// Whether the layout breaks lines.
    pub wrap: TextWrap,
    /// How wide a tab is, in columns. `None` leaves tabs alone.
    pub tab_width: Option<usize>,
    /// The width the layout breaks at. `None` means "as wide as it needs to be".
    pub max_width: Option<f32>,
    /// The height of one line, in logical pixels. `None` uses the font's own.
    ///
    /// This is the *pitch* between lines, not the height of the ink: a layout
    /// that breaks its text into several lines puts each of them this far
    /// down. An editor needs the pitch to be a number it knows rather than one
    /// it has to measure, because the scroll position is a number of lines and
    /// the caret's rectangle is a box on one of them.
    pub line_height: Option<f32>,
}
