#[derive(Debug, Clone, PartialEq)]
pub struct BadgeTokens {
    /// Font size of the numeric/string content.
    pub font_size: f32,
    /// Minimum width/height of the badge bubble (grows with content).
    pub min_size: f32,
    /// Horizontal inner padding when there is text content.
    pub padding: f32,
    /// Diameter of the dot-only variant.
    pub dot_size: f32,
}

impl Default for BadgeTokens {
    fn default() -> Self {
        Self {
            font_size: 11.0,
            min_size: 18.0,
            padding: 6.0,
            dot_size: 8.0,
        }
    }
}
