#[derive(Debug, Clone, PartialEq)]
pub struct ResponsiveTokens {
    /// Minimum height the container will occupy in px (0 = no floor).
    pub min_height: f32,
}

impl Default for ResponsiveTokens {
    fn default() -> Self {
        Self { min_height: 0.0 }
    }
}
