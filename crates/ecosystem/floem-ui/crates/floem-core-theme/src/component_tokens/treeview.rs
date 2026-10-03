#[derive(Debug, Clone, PartialEq)]
pub struct TreeviewTokens {
    /// Horizontal indent added per depth level.
    pub indent: f32,
    /// Height of a single tree row.
    pub row_height: f32,
    /// Vertical gap between top-level rows.
    pub item_gap: f32,
    /// Size of the leading icon.
    pub icon_size: f32,
    /// Size of the expand/collapse chevron.
    pub expander_size: f32,
    /// Row label font size.
    pub font_size: f32,
}

impl Default for TreeviewTokens {
    fn default() -> Self {
        Self {
            indent: 16.0,
            row_height: 40.0,
            item_gap: 0.0,
            icon_size: 20.0,
            expander_size: 18.0,
            font_size: 14.0,
        }
    }
}
