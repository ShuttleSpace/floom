#[derive(Debug, Clone, PartialEq)]
pub struct CalendarTokens {
    /// Size of each day cell (square).
    pub cell_size: f32,
    /// Height of the month header bar.
    pub header_height: f32,
    /// Month/year title font size.
    pub header_font_size: f32,
    /// Weekday label font size.
    pub weekday_font_size: f32,
    /// Day number font size.
    pub day_font_size: f32,
    /// Corner radius of day cells / surface.
    pub border_radius: f32,
    /// Width of the popup panel.
    pub panel_width: f32,
    /// Width reserved for the optional week-number column.
    pub weeknumber_width: f32,
}

impl Default for CalendarTokens {
    fn default() -> Self {
        Self {
            cell_size: 40.0,
            header_height: 40.0,
            header_font_size: 15.0,
            weekday_font_size: 11.0,
            day_font_size: 14.0,
            border_radius: 8.0,
            panel_width: 300.0,
            weeknumber_width: 28.0,
        }
    }
}
