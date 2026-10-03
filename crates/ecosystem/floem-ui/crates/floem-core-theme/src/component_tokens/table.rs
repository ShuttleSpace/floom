#[derive(Debug, Clone, PartialEq)]
pub struct TableTokens {
    /// Header row height.
    pub header_height: f32,
    /// Body row height.
    pub row_height: f32,
    /// Cell inner padding.
    pub cell_padding: f32,
    /// Outer + row divider border width.
    pub border_width: f32,
    /// Outer corner radius.
    pub border_radius: f32,
    /// Body cell font size.
    pub font_size: f32,
    /// Header cell font size.
    pub header_font_size: f32,
    /// Minimum table width.
    pub min_width: f32,
}

impl Default for TableTokens {
    fn default() -> Self {
        Self {
            header_height: 44.0,
            row_height: 52.0,
            cell_padding: 16.0,
            border_width: 1.0,
            border_radius: 12.0,
            font_size: 14.0,
            header_font_size: 13.0,
            min_width: 360.0,
        }
    }
}
