#[derive(Debug, Clone, PartialEq)]
pub struct TimelineTokens {
    /// Diameter of each timeline dot in dp.
    pub dot_size: f32,
    /// Thickness of the connector line in dp.
    pub line_width: f32,
    /// Vertical gap between timeline entries in dp.
    pub item_gap: f32,
    /// Padding around the timeline content in dp.
    pub padding: f32,
}

impl Default for TimelineTokens {
    fn default() -> Self {
        Self {
            dot_size: 12.0,
            line_width: 2.0,
            item_gap: 16.0,
            padding: 8.0,
        }
    }
}
