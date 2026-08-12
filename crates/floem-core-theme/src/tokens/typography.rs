#[derive(Debug, Clone, PartialEq)]
pub struct TypographyTokens {
    pub display: TypeScale,
    pub headline: TypeScale,
    pub title: TypeScale,
    pub body: TypeScale,
    pub label: TypeScale,
}
#[derive(Debug, Clone, PartialEq)]
pub struct TypeScale {
    pub large: TextStyle,
    pub medium: TextStyle,
    pub small: TextStyle,
}
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub font_size: f32,
    pub line_height: f32,
    pub letter_spacing: f32,
    pub font_weight: FontWeight,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    Regular,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

impl FontWeight {
    /// Convert to Floem's `parley`-based `FontWeight` for use with `Style::font_weight`.
    pub fn to_floem(self) -> floem::text::FontWeight {
        use floem::text::FontWeight as F;
        match self {
            FontWeight::Thin => F::THIN,
            FontWeight::ExtraLight => F::EXTRA_LIGHT,
            FontWeight::Light => F::LIGHT,
            FontWeight::Regular => F::NORMAL,
            FontWeight::Medium => F::MEDIUM,
            FontWeight::SemiBold => F::SEMI_BOLD,
            FontWeight::Bold => F::BOLD,
            FontWeight::ExtraBold => F::EXTRA_BOLD,
            FontWeight::Black => F::EXTRA_BOLD,
        }
    }
}
