#[derive(Debug, Clone, PartialEq)]
pub struct DataIteratorTokens {
    /// Inner padding of the container.
    pub padding: f32,
    /// Gap between consecutive items when not divided.
    pub item_gap: f32,
}

impl Default for DataIteratorTokens {
    fn default() -> Self {
        Self {
            padding: 8.0,
            item_gap: 0.0,
        }
    }
}
