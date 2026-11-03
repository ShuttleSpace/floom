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
    /// Size of the trigger chevron / dropdown arrow icon in px.
    pub icon_size: f32,
    /// Maximum height of the dropdown panel.
    pub panel_max_height: f32,
}

impl Default for SelectTokens {
    fn default() -> Self {
        Self {
            height: 56.0,
            padding_horizontal: 16.0,
            border_radius: 4.0,
            min_width: 180.0,
            item_height: 48.0,
            icon_size: 14.0,
            panel_max_height: 320.0,
        }
    }
}
