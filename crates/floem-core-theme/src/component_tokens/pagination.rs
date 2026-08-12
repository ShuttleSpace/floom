#[derive(Debug, Clone, PartialEq)]
pub struct PaginationTokens {
    /// Square size of each page button in dp.
    pub button_size: f32,
    /// Gap between page controls in dp.
    pub gap: f32,
}

impl Default for PaginationTokens {
    fn default() -> Self {
        Self {
            button_size: 32.0,
            gap: 4.0,
        }
    }
}
