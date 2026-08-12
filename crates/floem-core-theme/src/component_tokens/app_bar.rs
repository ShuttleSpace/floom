#[derive(Debug, Clone, PartialEq)]
pub struct AppBarTokens {
    /// Height of the bar in dp.
    pub height: f32,
    /// Horizontal inner padding in dp.
    pub padding_horizontal: f32,
    /// Font size of the title text.
    pub title_font_size: f32,
    /// Default elevation/shadow depth.
    pub elevation: u8,
}

impl Default for AppBarTokens {
    fn default() -> Self {
        Self {
            height: 56.0,
            padding_horizontal: 16.0,
            title_font_size: 20.0,
            elevation: 2,
        }
    }
}
