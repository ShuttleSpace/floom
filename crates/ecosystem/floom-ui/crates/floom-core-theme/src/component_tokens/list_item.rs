#[derive(Debug, Clone, PartialEq)]
pub struct ListItemTokens {
    /// Minimum row height.
    pub min_height: f32,
    /// Horizontal padding inside the row.
    pub padding_horizontal: f32,
    /// Gap between the leading slot and the text column.
    pub gap: f32,
    /// Primary line font size.
    pub title_font_size: f32,
    /// Secondary line font size.
    pub subtitle_font_size: f32,
}

impl Default for ListItemTokens {
    fn default() -> Self {
        Self {
            min_height: 56.0,
            padding_horizontal: 16.0,
            gap: 16.0,
            title_font_size: 16.0,
            subtitle_font_size: 14.0,
        }
    }
}
