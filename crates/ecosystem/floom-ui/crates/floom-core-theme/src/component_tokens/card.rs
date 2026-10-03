use crate::tokens::spacing::SpacingTokens;
#[derive(Debug, Clone, PartialEq)]
pub struct CardTokens {
    pub elevation: u8,
    pub border_radius: f32,
    pub padding: f32,
}
impl CardTokens {
    pub fn from_spacing(s: &SpacingTokens) -> Self {
        Self {
            elevation: 1,
            border_radius: 12.0,
            padding: s.md,
        }
    }
}
