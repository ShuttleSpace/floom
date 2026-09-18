#[derive(Debug, Clone, PartialEq)]
pub struct IconTokens {
    pub size: f32,
}

impl Default for IconTokens {
    fn default() -> Self {
        Self { size: 24.0 }
    }
}
