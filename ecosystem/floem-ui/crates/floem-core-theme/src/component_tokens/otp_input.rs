#[derive(Debug, Clone, PartialEq)]
pub struct OtpInputTokens {
    /// Width/height of each single-character box.
    pub box_size: f32,
    /// Corner radius of each box.
    pub border_radius: f32,
    /// Gap between boxes.
    pub gap: f32,
    /// Font size of the entered character.
    pub font_size: f32,
}

impl Default for OtpInputTokens {
    fn default() -> Self {
        Self {
            box_size: 48.0,
            border_radius: 4.0,
            gap: 8.0,
            font_size: 24.0,
        }
    }
}
