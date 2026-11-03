use crate::tokens::spacing::SpacingTokens;
#[derive(Debug, Clone, PartialEq)]
pub struct ButtonTokens {
    pub height: f32,
    pub padding_horizontal: f32,
    pub gap: f32,
    pub icon_size: f32,
}
impl ButtonTokens {
    pub fn from_spacing(s: &SpacingTokens) -> Self {
        Self {
            height: 40.0,
            padding_horizontal: s.lg,
            gap: s.sm,
            icon_size: 18.0,
        }
    }
}
