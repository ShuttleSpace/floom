#[derive(Debug, Clone, PartialEq)]
pub struct TooltipTokens {
    /// Inner padding of the tooltip bubble.
    pub padding: f32,
    /// Corner radius of the tooltip bubble.
    pub border_radius: f32,
    /// Font size of the tooltip text.
    pub font_size: f32,
}

impl Default for TooltipTokens {
    fn default() -> Self {
        Self {
            padding: 6.0,
            border_radius: 4.0,
            font_size: 12.0,
        }
    }
}
