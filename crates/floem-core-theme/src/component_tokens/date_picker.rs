#[derive(Debug, Clone, PartialEq)]
pub struct DatePickerTokens {
    /// Height of the trigger field.
    pub height: f32,
    /// Horizontal inner padding of the trigger.
    pub padding_horizontal: f32,
    /// Corner radius of the trigger + panel.
    pub border_radius: f32,
    /// Minimum width of the trigger.
    pub min_width: f32,
    /// Width of the dropdown calendar panel.
    pub panel_width: f32,
    /// Size of each day cell (square).
    pub cell_size: f32,
    /// Font size of the month/year header.
    pub header_font_size: f32,
}

impl Default for DatePickerTokens {
    fn default() -> Self {
        Self {
            height: 40.0,
            padding_horizontal: 12.0,
            border_radius: 8.0,
            min_width: 200.0,
            panel_width: 280.0,
            cell_size: 36.0,
            header_font_size: 14.0,
        }
    }
}
