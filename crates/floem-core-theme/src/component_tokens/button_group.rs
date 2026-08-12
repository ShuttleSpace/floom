#[derive(Debug, Clone, PartialEq)]
pub struct ButtonGroupTokens {
    /// Gap between grouped buttons. 0 produces a joined/segmented group.
    pub gap: f32,
    /// Corner radius applied to the outer group when joined.
    pub border_radius: f32,
}

impl Default for ButtonGroupTokens {
    fn default() -> Self {
        Self {
            gap: 0.0,
            border_radius: 12.0,
        }
    }
}
