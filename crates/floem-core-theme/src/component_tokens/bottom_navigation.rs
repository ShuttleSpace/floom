#[derive(Debug, Clone, PartialEq)]
pub struct BottomNavigationTokens {
    /// Height of the navigation bar in dp.
    pub height: f32,
    /// Minimum tappable width of a single destination.
    pub item_min_width: f32,
    /// Size of the destination icon in dp.
    pub icon_size: f32,
    /// Font size of the destination label.
    pub label_font_size: f32,
}

impl Default for BottomNavigationTokens {
    fn default() -> Self {
        Self {
            height: 80.0,
            item_min_width: 80.0,
            icon_size: 24.0,
            label_font_size: 12.0,
        }
    }
}
