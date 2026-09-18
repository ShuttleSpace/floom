#[derive(Debug, Clone, PartialEq)]
pub struct ToolbarTokens {
    /// Bar height in dp.
    pub height: f32,
    /// Horizontal padding inside the bar.
    pub padding: f32,
    /// Title font size.
    pub title_font_size: f32,
}

impl Default for ToolbarTokens {
    fn default() -> Self {
        Self {
            height: 48.0,
            padding: 12.0,
            title_font_size: 16.0,
        }
    }
}
