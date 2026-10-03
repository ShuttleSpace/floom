#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlideTransitionTokens {
    /// Enter/leave duration in milliseconds.
    pub duration_ms: f32,
    /// Travel distance in pixels from the off-screen start position to the resting position.
    pub offset: f32,
}

impl Default for SlideTransitionTokens {
    fn default() -> Self {
        Self {
            duration_ms: 250.0,
            offset: 100.0,
        }
    }
}
