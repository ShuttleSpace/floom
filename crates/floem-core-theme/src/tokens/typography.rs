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
