#[derive(Debug, Clone, PartialEq)]
pub struct ChipTokens {
    /// Height of the chip pill.
    pub height: f32,
    /// Horizontal inner padding of the chip.
    pub padding_horizontal: f32,
    /// Corner radius (large = fully rounded pill).
    pub border_radius: f32,
    /// Label font size.
    pub font_size: f32,
    /// Gap between the leading icon (if any) and the label.
    pub gap: f32,
}

impl Default for ChipTokens {
    fn default() -> Self {
        Self {
            height: 32.0,
            padding_horizontal: 12.0,
            border_radius: 16.0,
            font_size: 14.0,
            gap: 6.0,
        }
    }
}
