use crate::tokens::spacing::SpacingTokens;
#[derive(Debug, Clone, PartialEq)]
pub struct DialogTokens {
    pub max_width: f32,
    pub padding: f32,
    pub border_radius: f32,
}
impl DialogTokens {
    pub fn from_spacing(s: &SpacingTokens) -> Self {
        Self {
            max_width: 560.0,
            padding: s.lg,
            border_radius: 28.0,
        }
    }
}
