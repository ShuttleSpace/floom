use crate::tokens::color::Color;

#[derive(Debug, Clone, PartialEq)]
pub struct ImageTokens {
    /// Corner radius when the image is `rounded`.
    pub corner_radius: f32,
    /// Background shown behind the image / while loading.
    pub placeholder_color: Color,
}

impl Default for ImageTokens {
    fn default() -> Self {
        Self {
            corner_radius: 12.0,
            placeholder_color: Color::rgb(224, 224, 224),
        }
    }
}
