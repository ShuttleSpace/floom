#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CodeProps {
    /// Render the content in a monospace font.
    pub mono: bool,
}

impl Default for CodeProps {
    fn default() -> Self {
        Self { mono: true }
    }
}
