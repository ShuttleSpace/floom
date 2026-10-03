#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FadeTransitionTokens {
    /// Enter/leave duration in milliseconds.
    pub duration_ms: f32,
}

impl Default for FadeTransitionTokens {
    fn default() -> Self {
        Self { duration_ms: 200.0 }
    }
}
