#[derive(Debug, Clone, PartialEq)]
pub struct PullToRefreshTokens {
    pub indicator_size: f32,
    pub threshold: f32,
    pub max_pull: f32,
}

impl Default for PullToRefreshTokens {
    fn default() -> Self {
        Self {
            indicator_size: 24.0,
            threshold: 80.0,
            max_pull: 120.0,
        }
    }
}
