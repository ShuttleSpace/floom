#[derive(Debug, Clone, PartialEq)]
pub struct ListSubheaderProps {
    /// Subheader text.
    pub text: String,
    /// Whether the subheader is inset (greater left padding for nested lists).
    pub inset: bool,
}

impl Default for ListSubheaderProps {
    fn default() -> Self {
        Self {
            text: String::new(),
            inset: false,
        }
    }
}
