#[derive(Debug, Clone, PartialEq)]
pub struct EmptyStateTokens {
    /// Font size / box size of the icon glyph.
    pub icon_size: f32,
    /// Font size of the title.
    pub title_font_size: f32,
    /// Font size of the description.
    pub description_font_size: f32,
    /// Gap between icon / title / description blocks.
    pub gap: f32,
    /// Max width of the centered column.
    pub max_width: f32,
}

impl Default for EmptyStateTokens {
    fn default() -> Self {
        Self {
            icon_size: 48.0,
            title_font_size: 18.0,
            description_font_size: 14.0,
            gap: 8.0,
            max_width: 320.0,
        }
    }
}
