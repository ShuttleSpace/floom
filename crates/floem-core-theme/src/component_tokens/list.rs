#[derive(Debug, Clone, PartialEq)]
pub struct ListTokens {
    /// Height of a single row.
    pub item_height: f32,
    /// Inner padding of the list surface.
    pub padding: f32,
    /// Minimum width of the list surface.
    pub min_width: f32,
    /// Gap between rows (0 = flush rows).
    pub gap: f32,
    /// Horizontal inner padding of the list surface in px.
    pub padding_horizontal: f32,
}

impl Default for ListTokens {
    fn default() -> Self {
        Self {
            item_height: 48.0,
            padding: 8.0,
            min_width: 200.0,
            gap: 0.0,
            padding_horizontal: 16.0,
        }
    }
}
