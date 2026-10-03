#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HoverTokens {
    pub highlight_alpha: f32,
    pub transition_ms: f32,
}

impl Default for HoverTokens {
    fn default() -> Self {
        Self {
            highlight_alpha: 8.0,
            transition_ms: 120.0,
        }
    }
}
