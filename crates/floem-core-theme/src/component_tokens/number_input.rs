#[derive(Debug, Clone, PartialEq)]
pub struct NumberInputTokens {
    pub height: f32,
    pub width: f32,
    pub button_size: f32,
    pub padding_horizontal: f32,
    pub border: f32,
    pub radius: f32,
    pub font_size: f32,
}

impl Default for NumberInputTokens {
    fn default() -> Self {
        Self {
            height: 40.0,
            width: 200.0,
            button_size: 32.0,
            padding_horizontal: 12.0,
            border: 1.0,
            radius: 8.0,
            font_size: 16.0,
        }
    }
}
