#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScaleTransitionTokens {
    /// Enter/leave duration in milliseconds.
    pub duration_ms: f32,
    /// Scale at the start of the enter animation and end of the leave animation (1.0 = natural size).
    pub min_scale: f32,
}

impl Default for ScaleTransitionTokens {
    fn default() -> Self {
        Self {
            duration_ms: 200.0,
            min_scale: 0.8,
        }
    }
}
