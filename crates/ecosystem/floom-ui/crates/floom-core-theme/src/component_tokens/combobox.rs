#[derive(Debug, Clone, PartialEq)]
pub struct ComboboxTokens {
    /// Height of the search field / trigger.
    pub height: f32,
    /// Horizontal inner padding of the trigger.
    pub padding_horizontal: f32,
    /// Corner radius of the trigger + dropdown surface.
    pub border_radius: f32,
    /// Minimum width of the trigger/dropdown.
    pub min_width: f32,
    /// Height of each option row in the dropdown.
    pub item_height: f32,
    /// Max height of the scrolling dropdown panel.
    pub panel_max_height: f32,
}

impl Default for ComboboxTokens {
    fn default() -> Self {
        Self {
            height: 56.0,
            padding_horizontal: 16.0,
            border_radius: 4.0,
            min_width: 220.0,
            item_height: 48.0,
            panel_max_height: 240.0,
        }
    }
}
