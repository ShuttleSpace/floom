#[derive(Debug, Clone, PartialEq)]
pub struct KbdTokens {
    /// Horizontal padding of the keycap.
    pub padding_horizontal: f32,
    /// Vertical padding of the keycap.
    pub padding_vertical: f32,
    /// Keycap corner radius.
    pub border_radius: f32,
    /// Keycap border width.
    pub border_width: f32,
    /// Keycap font size.
    pub font_size: f32,
}

impl Default for KbdTokens {
    fn default() -> Self {
        Self {
            padding_horizontal: 6.0,
            padding_vertical: 2.0,
            border_radius: 6.0,
            border_width: 1.0,
            font_size: 13.0,
        }
    }
}
