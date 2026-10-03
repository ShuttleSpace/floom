#[derive(Debug, Clone, PartialEq)]
pub struct VirtualScrollerTokens {
    /// Height of a single row (must match the actual rendered item height).
    pub item_height: f32,
    /// Gap between rows.
    pub row_gap: f32,
    /// Inner padding.
    pub padding: f32,
    /// Extra rows rendered above/below the viewport.
    pub overscan: usize,
    /// Row font size.
    pub font_size: f32,
}

impl Default for VirtualScrollerTokens {
    fn default() -> Self {
        Self {
            item_height: 44.0,
            row_gap: 0.0,
            padding: 0.0,
            overscan: 4,
            font_size: 14.0,
        }
    }
}
