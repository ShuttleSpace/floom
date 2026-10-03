#[derive(Debug, Clone, PartialEq)]
pub struct FooterTokens {
    /// Bar height in dp.
    pub height: f32,
    /// Horizontal padding in dp.
    pub pad: f32,
    /// Top border thickness in dp.
    pub border: f32,
}

impl Default for FooterTokens {
    fn default() -> Self {
        Self {
            height: 56.0,
            pad: 16.0,
            border: 1.0,
        }
    }
}
