#[derive(Debug, Clone, PartialEq)]
pub struct SpacerTokens {
    /// Default flex grow factor for the empty spacer view.
    pub grow: f32,
}

impl Default for SpacerTokens {
    fn default() -> Self {
        Self { grow: 1.0 }
    }
}
