#[derive(Debug, Clone)]
pub struct DisplayProviderProps {
    /// When true, descendants should render in compact (dense) density.
    pub dense: bool,
}

impl Default for DisplayProviderProps {
    fn default() -> Self {
        Self { dense: false }
    }
}
