#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KbdProps {
    /// Render the content in a monospace font.
    pub mono: bool,
}

impl Default for KbdProps {
    fn default() -> Self {
        Self { mono: true }
    }
}
