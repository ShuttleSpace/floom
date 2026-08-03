#[derive(Debug, Clone, PartialEq)]
pub struct TextTokens {
    pub max_lines_default: usize,
}
impl Default for TextTokens {
    fn default() -> Self {
        Self {
            max_lines_default: 1,
        }
    }
}
