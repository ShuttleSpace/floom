#[derive(Debug, Clone)]
pub struct TextProps {
    pub content: String,
    pub max_lines: Option<usize>,
    pub is_selectable: bool,
}
impl Default for TextProps {
    fn default() -> Self {
        Self {
            content: String::new(),
            max_lines: None,
            is_selectable: false,
        }
    }
}
