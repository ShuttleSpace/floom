use crate::tokens::spacing::SpacingTokens;
#[derive(Debug, Clone, PartialEq)]
pub struct TextFieldTokens {
    pub height: f32,
    pub padding_horizontal: f32,
    pub border_radius: f32,
}
impl TextFieldTokens {
    pub fn from_spacing(s: &SpacingTokens) -> Self {
        Self {
            height: 48.0,
            padding_horizontal: s.md,
            border_radius: 4.0,
        }
    }
}
