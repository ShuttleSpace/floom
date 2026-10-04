import io

# ---------------------------------------------------------------- floom-core
p = 'crates/floom-core/src/compose.rs'
s = io.open(p, encoding='utf-8').read()

# 1) The type.
old = """/// Whether a box participates in its parent's flow.
"""
new = """/// A length along one axis: a fixed number of logical pixels, or a fraction of
/// the containing block.
///
/// A percentage is not a convenience — it is a different thing to measure, and
/// the difference is the whole reason a full-width row can fill its parent while
/// a fixed 200px row cannot. v1 had `width(..)` and `width_pct(..)` as two
/// builders over one field, and 2.0 had only the first, so the 310
/// `width_pct` / `height_pct` calls across floem-ui had no spelling at all.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    /// A fixed extent in logical pixels.
    Px(f32),
    /// A percentage of the containing block, `0.0..=100.0`.
    Pct(f32),
}

impl Length {
    /// The same length as a fraction of the containing block.
    pub const fn pct(percent: f32) -> Self {
        Self::Pct(percent)
    }

    /// The fixed part, if this is one. `None` for a percentage, which has no
    /// pixel value until the containing block is known — so a caller that needs a
    /// number has to say what it will do with the answer rather than get a
    /// silently wrong one.
    pub const fn px(self) -> Option<f32> {
        match self {
            Self::Px(px) => Some(px),
            Self::Pct(_) => None,
        }
    }

    pub const fn is_pct(self) -> bool {
        matches!(self, Self::Pct(_))
    }
}

/// Whether a box participates in its parent's flow.
"""
assert s.count(old) == 1, ('Length insert', s.count(old))
s = s.replace(old, new)

# 2) The fields.
old = """    pub width: Option<f32>,
    pub height: Option<f32>,
    /// Fraction of leftover space this box claims along the parent's main axis."""
new = """    /// `None` is "unspecified", which is not the same as zero: the layer lowering
    /// this decides whether an unspecified dimension is auto or filled.
    pub width: Option<Length>,
    pub height: Option<Length>,
    /// Fraction of leftover space this box claims along the parent's main axis."""
assert s.count(old) == 1, ('fields', s.count(old))
s = s.replace(old, new)

# 3) Builders: keep `width`/`height` taking pixels, add the percentage forms.
old = """    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub const fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }"""
new = """    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(Length::Px(width));
        self
    }

    pub const fn height(mut self, height: f32) -> Self {
        self.height = Some(Length::Px(height));
        self
    }

    /// Width as a percentage of the containing block. See [`Length`].
    pub const fn width_pct(mut self, percent: f32) -> Self {
        self.width = Some(Length::Pct(percent));
        self
    }

    /// Height as a percentage of the containing block. See [`Length`].
    pub const fn height_pct(mut self, percent: f32) -> Self {
        self.height = Some(Length::Pct(percent));
        self
    }"""
assert s.count(old) == 1, ('builders', s.count(old))
s = s.replace(old, new)

# 4) SpaceBetween: the layout layer already has the value; only the vocabulary
#    was missing.
old = """    /// Occupy the full cross size (the flex `stretch` default).
    #[default]
    Stretch,
    Start,
    Center,
    End,
}"""
new = """    /// Occupy the full cross size (the flex `stretch` default).
    #[default]
    Stretch,
    Start,
    Center,
    End,
    /// Distribute the free space *between* the children, none at either end.
    ///
    /// Only meaningful as a `main_align`, which is why it is here rather than in
    /// the padding vocabulary: it distributes, it does not place one box. floem-ui
    /// asks for it 38 times, nearly always as v1's `justify_between()`.
    SpaceBetween,
}"""
assert s.count(old) == 1, ('Align', s.count(old))
s = s.replace(old, new)

io.open(p, 'w', encoding='utf-8').write(s)

# re-export
p = 'crates/floom-core/src/lib.rs'
s = io.open(p, encoding='utf-8').read()
old = "pub use compose::{Align, Axis, Child, ChildBuilder, ChildKey, Inset, LayoutRequest, Position};"
new = "pub use compose::{\n    Align, Axis, Child, ChildBuilder, ChildKey, Inset, LayoutRequest, Length, Position,\n};"
assert s.count(old) == 1, ('re-export', s.count(old))
io.open(p, 'w', encoding='utf-8').write(s.replace(old, new))

# ------------------------------------------------------------- floem-layout
p = 'crates/floom-layout/src/taffy_adapter.rs'
s = io.open(p, encoding='utf-8').read()

old = """use floem_types::{Insets, Point, Rect, Size as GeoSize};"""
new = """use floom_core::Length;
use floom_types::{Insets, Point, Rect, Size as GeoSize};"""
assert s.count(old) == 1, ('import', s.count(old))
s = s.replace(old, new)

old = """    pub width: Option<f32>,
    pub height: Option<f32>,"""
new = """    pub width: Option<Length>,
    pub height: Option<Length>,"""
assert s.count(old) == 1, ('style fields', s.count(old))
s = s.replace(old, new)

old = """            size: taffy::Size {
                width: self
                    .width
                    .map(Dimension::length)
                    .unwrap_or(Dimension::auto()),
                height: self
                    .height
                    .map(Dimension::length)
                    .unwrap_or(Dimension::auto()),
            },"""
new = """            size: taffy::Size {
                width: dimension(self.width),
                height: dimension(self.height),
            },"""
assert s.count(old) == 1, ('to_taffy size', s.count(old))
s = s.replace(old, new)

old = """/// `auto` when unset, otherwise a fixed offset from the containing block edge."""
new = """/// Lowers one axis's length. `None` is auto; a percentage resolves against the
/// containing block's content box, which is what makes `width_pct(100.0)` a
/// full-width row rather than a guess.
fn dimension(length: Option<Length>) -> Dimension {
    match length {
        Some(Length::Px(px)) => Dimension::length(px),
        Some(Length::Pct(percent)) => Dimension::percentage(f64::from(percent) / 100.0),
        None => Dimension::auto(),
    }
}

/// `auto` when unset, otherwise a fixed offset from the containing block edge."""
assert s.count(old) == 1, ('dimension fn', s.count(old))
s = s.replace(old, new)
io.open(p, 'w', encoding='utf-8').write(s)
print('Length + SpaceBetween: core and layout done')
