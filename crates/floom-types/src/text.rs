//! Text appearance: weight.
//!
//! Renderer-neutral by construction, like everything else in `floom-types`: the
//! nine CSS weights are named here and the lowering layer maps them onto whatever
//! the shaper wants (`fontique::FontWeight::new(500.0)` at the bottom of the
//! stack). Nothing in this module knows a font exists.

/// How heavy a run of text is drawn.
///
/// The nine CSS weights, named as CSS names them. floem-ui already writes
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
