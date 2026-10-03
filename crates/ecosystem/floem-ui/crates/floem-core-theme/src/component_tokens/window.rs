#[derive(Debug, Clone, PartialEq)]
pub struct WindowTokens {
    pub height: f32,
    pub border_radius: f32,
    pub dot_size: f32,
    pub arrow_size: f32,
}

impl Default for WindowTokens {
    fn default() -> Self {
        Self {
            height: 240.0,
            border_radius: 12.0,
            dot_size: 8.0,
            arrow_size: 36.0,
        }
    }
}
