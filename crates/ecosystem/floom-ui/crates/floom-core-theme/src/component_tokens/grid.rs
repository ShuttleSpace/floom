#[derive(Debug, Clone, PartialEq)]
pub struct GridTokens {
    /// Gap between columns within a row.
    pub gap: f32,
    /// Gap between rows.
    pub row_gap: f32,
}

impl Default for GridTokens {
    fn default() -> Self {
        Self {
            gap: 12.0,
            row_gap: 12.0,
        }
    }
}
