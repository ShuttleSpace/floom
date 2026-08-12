#[derive(Debug, Clone, PartialEq)]
pub struct SelectTokens {
    /// Height of the closed trigger control.
    pub height: f32,
    /// Horizontal inner padding of the trigger.
    pub padding_horizontal: f32,
    /// Corner radius of the trigger + dropdown surface.
    pub border_radius: f32,
    /// Minimum width of the trigger/dropdown.
    pub min_width: f32,
    /// Height of each option row in the dropdown.
    pub item_height: f32,
}

impl Default for SelectTokens {
    fn default() -> Self {
        Self {
            height: 40.0,
            padding_horizontal: 12.0,
            border_radius: 8.0,
            min_width: 180.0,
            item_height: 40.0,
        }
    }
}
