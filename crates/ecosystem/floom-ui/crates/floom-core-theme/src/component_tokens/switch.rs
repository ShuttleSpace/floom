#[derive(Debug, Clone, PartialEq)]
pub struct SwitchTokens {
    pub width: f32,
    pub height: f32,
    pub thumb_size: f32,
}
impl Default for SwitchTokens {
    fn default() -> Self {
        Self {
            width: 52.0,
            height: 32.0,
            thumb_size: 24.0,
        }
    }
}
