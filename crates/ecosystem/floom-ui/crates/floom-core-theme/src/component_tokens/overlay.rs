/// Padding applied to the centered content layer above the scrim.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayTokens {
    pub content_padding: f32,
}
impl Default for OverlayTokens {
    fn default() -> Self {
        Self {
            content_padding: 24.0,
        }
    }
}
