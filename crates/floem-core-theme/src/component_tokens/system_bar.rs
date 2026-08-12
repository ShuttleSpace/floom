#[derive(Debug, Clone, PartialEq)]
pub struct SystemBarTokens {
    /// Bar height in dp.
    pub height: f32,
    /// Horizontal padding in dp.
    pub pad: f32,
}

impl Default for SystemBarTokens {
    fn default() -> Self {
        Self {
            height: 24.0,
            pad: 8.0,
        }
    }
}
