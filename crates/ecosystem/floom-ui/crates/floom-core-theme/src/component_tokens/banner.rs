#[derive(Debug, Clone, PartialEq)]
pub struct BannerTokens {
    /// Minimum height of the banner bar.
    pub height: f32,
    /// Inner horizontal/vertical padding.
    pub padding: f32,
    /// Message font size.
    pub font_size: f32,
    /// Action button font size.
    pub action_font_size: f32,
    /// Gap between the icon, message and action.
    pub gap: f32,
    /// Leading icon size.
    pub icon_size: f32,
}

impl Default for BannerTokens {
    fn default() -> Self {
        Self {
            height: 48.0,
            padding: 8.0,
            font_size: 14.0,
            action_font_size: 13.0,
            gap: 12.0,
            icon_size: 20.0,
        }
    }
}
