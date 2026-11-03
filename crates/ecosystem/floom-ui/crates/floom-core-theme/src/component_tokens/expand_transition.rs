#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExpandTransitionTokens {
    /// Enter/leave duration in milliseconds.
    pub duration_ms: f32,
}

impl Default for ExpandTransitionTokens {
    fn default() -> Self {
        Self { duration_ms: 200.0 }
    }
}
