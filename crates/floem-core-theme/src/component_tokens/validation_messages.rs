#[derive(Debug, Clone, PartialEq)]
pub struct ValidationMessagesTokens {
    /// Font size for each message in dp.
    pub font_size: f32,
    /// Vertical gap between individual messages in dp.
    pub gap: f32,
}

impl Default for ValidationMessagesTokens {
    fn default() -> Self {
        Self {
            font_size: 12.0,
            gap: 4.0,
        }
    }
}
