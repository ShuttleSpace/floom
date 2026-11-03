#[derive(Debug, Clone)]
pub struct ThemeProviderProps {
    /// Initial dark-mode flag provided to descendants.
    pub dark: bool,
}

impl Default for ThemeProviderProps {
    fn default() -> Self {
        Self { dark: false }
    }
}
