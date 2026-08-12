#[derive(Debug, Clone, PartialEq)]
pub struct DataTableTokens {
    /// Height of the sticky header row.
    pub header_height: f32,
    /// Height of a standard body row.
    pub row_height: f32,
    /// Height of a row when `dense` is set.
    pub dense_row_height: f32,
    /// Horizontal padding inside each cell.
    pub cell_padding: f32,
    /// Corner radius of the table surface.
    pub border_radius: f32,
    /// Border width of the table surface / dividers.
    pub border_width: f32,
    /// Body cell font size.
    pub font_size: f32,
    /// Header cell font size.
    pub header_font_size: f32,
    /// Minimum width of the whole table.
    pub min_width: f32,
}

impl Default for DataTableTokens {
    fn default() -> Self {
        Self {
            header_height: 48.0,
            row_height: 52.0,
            dense_row_height: 40.0,
            cell_padding: 16.0,
            border_radius: 12.0,
            border_width: 1.0,
            font_size: 14.0,
            header_font_size: 13.0,
            min_width: 320.0,
        }
    }
}
