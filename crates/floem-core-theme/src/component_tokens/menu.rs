use crate::tokens::spacing::SpacingTokens;
#[derive(Debug, Clone, PartialEq)]
pub struct MenuTokens {
    pub min_width: f32,
    pub max_width: f32,
    pub item_height: f32,
    pub padding: f32,
}
impl MenuTokens {
    pub fn from_spacing(s: &SpacingTokens) -> Self {
        Self {
            min_width: 112.0,
            max_width: 280.0,
            item_height: 48.0,
            padding: s.sm,
        }
    }
}
