#[derive(Debug, Clone, PartialEq)]
pub struct CheckboxTokens {
    pub size: f32,
    pub border_radius: f32,
}
impl Default for CheckboxTokens {
    fn default() -> Self {
        Self {
            size: 18.0,
            border_radius: 2.0,
        }
    }
}
