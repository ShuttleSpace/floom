#[derive(Debug, Clone, PartialEq)]
pub struct TabsTokens {
    /// Height of the tab bar.
    pub height: f32,
    /// Height of the active-tab indicator line.
    pub indicator_height: f32,
    /// Horizontal padding inside each tab label.
    pub padding: f32,
}

impl Default for TabsTokens {
    fn default() -> Self {
        Self {
            height: 48.0,
            indicator_height: 3.0,
            padding: 16.0,
        }
    }
}
