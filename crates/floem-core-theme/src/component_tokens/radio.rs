#[derive(Debug, Clone, PartialEq)]
pub struct RadioTokens {
    pub size: f32,
    pub border_width: f32,
    pub label_font_size: f32,
}
impl Default for RadioTokens {
    fn default() -> Self {
        Self {
            size: 20.0,
            border_width: 2.0,
            label_font_size: 14.0,
        }
    }
}
