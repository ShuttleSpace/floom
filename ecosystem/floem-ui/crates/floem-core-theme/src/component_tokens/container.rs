#[derive(Debug, Clone, PartialEq)]
pub struct ContainerTokens {
    /// Default maximum content width in dp.
    pub max_width: f32,
    /// Default horizontal padding in dp.
    pub padding: f32,
}

impl Default for ContainerTokens {
    fn default() -> Self {
        Self {
            max_width: 1200.0,
            padding: 16.0,
        }
    }
}
