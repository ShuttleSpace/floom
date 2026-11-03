#[derive(Debug, Clone, PartialEq)]
pub struct CodeTokens {
    /// Horizontal padding of the inline code chip.
    pub padding_horizontal: f32,
    /// Vertical padding of the inline code chip.
    pub padding_vertical: f32,
    /// Corner radius of the inline code chip.
    pub border_radius: f32,
    /// Code chip font size.
    pub font_size: f32,
}

impl Default for CodeTokens {
    fn default() -> Self {
        Self {
            padding_horizontal: 6.0,
            padding_vertical: 2.0,
            border_radius: 4.0,
            font_size: 13.0,
        }
    }
}
