#[derive(Debug, Clone, PartialEq)]
pub struct FileInputTokens {
    /// Height of the trigger button.
    pub height: f32,
    /// Horizontal padding inside the button.
    pub padding_horizontal: f32,
    /// Corner radius of the button.
    pub border_radius: f32,
    /// Font size of the label and the selected-file name.
    pub font_size: f32,
}

impl Default for FileInputTokens {
    fn default() -> Self {
        Self {
            height: 44.0,
            padding_horizontal: 16.0,
            border_radius: 12.0,
            font_size: 14.0,
        }
    }
}
