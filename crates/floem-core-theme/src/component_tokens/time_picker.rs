#[derive(Debug, Clone, PartialEq)]
pub struct TimePickerTokens {
    /// Height of the trigger field.
    pub height: f32,
    /// Horizontal inner padding of the trigger.
    pub padding_horizontal: f32,
    /// Corner radius of the trigger + panel.
    pub border_radius: f32,
    /// Minimum width of the trigger.
    pub min_width: f32,
    /// Width of the dropdown panel.
    pub panel_width: f32,
    /// Height of each hour/minute stepper row.
    pub cell_height: f32,
}

impl Default for TimePickerTokens {
    fn default() -> Self {
        Self {
            height: 40.0,
            padding_horizontal: 12.0,
            border_radius: 8.0,
            min_width: 160.0,
            panel_width: 220.0,
            cell_height: 40.0,
        }
    }
}
