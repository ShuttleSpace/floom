#[derive(Debug, Clone, PartialEq)]
pub struct SlideGroupTokens {
    pub height: f32,
    pub gap: f32,
    pub active_indicator_height: f32,
    pub arrow_size: f32,
}

impl Default for SlideGroupTokens {
    fn default() -> Self {
        Self {
            height: 48.0,
            gap: 8.0,
            active_indicator_height: 3.0,
            arrow_size: 32.0,
        }
    }
}
