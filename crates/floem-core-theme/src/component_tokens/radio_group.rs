#[derive(Debug, Clone, PartialEq)]
pub struct RadioGroupTokens {
    /// Diameter of each radio indicator.
    pub radio_size: f32,
    /// Height of a single option row.
    pub item_height: f32,
    /// Gap between option rows.
    pub gap: f32,
}

impl Default for RadioGroupTokens {
    fn default() -> Self {
        Self {
            radio_size: 20.0,
            item_height: 40.0,
            gap: 8.0,
        }
    }
}
