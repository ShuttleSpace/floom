use crate::tokens::color::Color;

#[derive(Debug, Clone, PartialEq)]
pub struct SkeletonTokens {
    /// Base placeholder colour.
    pub color: Color,
    /// Corner radius of placeholder blocks.
    pub corner_radius: f32,
    /// Height of a single text line.
    pub text_height: f32,
    /// Diameter of a circle/avatar placeholder.
    pub avatar_size: f32,
    /// Gap between consecutive placeholder blocks.
    pub gap: f32,
    /// Corner radius of a card placeholder.
    pub card_radius: f32,
}

impl Default for SkeletonTokens {
    fn default() -> Self {
        Self {
            color: Color::rgb(224, 224, 224),
            corner_radius: 6.0,
            text_height: 12.0,
            avatar_size: 40.0,
            gap: 8.0,
            card_radius: 12.0,
        }
    }
}
