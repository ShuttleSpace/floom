#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InfiniteScrollerTokens {
    pub loading_indicator_size: f32,
    pub footer_padding: f32,
}

impl Default for InfiniteScrollerTokens {
    fn default() -> Self {
        Self {
            loading_indicator_size: 24.0,
            footer_padding: 12.0,
        }
    }
}
