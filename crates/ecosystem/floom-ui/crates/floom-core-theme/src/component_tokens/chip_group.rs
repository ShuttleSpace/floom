#[derive(Debug, Clone, PartialEq)]
pub struct ChipGroupTokens {
    /// Gap between chips in px.
    pub gap: f32,
}

impl Default for ChipGroupTokens {
    fn default() -> Self {
        Self { gap: 8.0 }
    }
}
