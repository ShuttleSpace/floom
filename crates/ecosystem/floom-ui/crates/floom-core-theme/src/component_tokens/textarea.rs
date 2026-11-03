#[derive(Debug, Clone, PartialEq)]
pub struct TextareaTokens {
    /// Inner padding of the text area.
    pub padding: f32,
    /// Corner radius of the text area surface.
    pub border_radius: f32,
    /// Minimum height (driven by `rows`).
    pub min_height: f32,
    /// Body font size.
    pub font_size: f32,
    /// Maximum width of the text area.
    pub max_width: f32,
}

impl Default for TextareaTokens {
    fn default() -> Self {
        Self {
            padding: 16.0,
            border_radius: 4.0,
            min_height: 80.0,
            font_size: 16.0,
            max_width: 360.0,
        }
    }
}
