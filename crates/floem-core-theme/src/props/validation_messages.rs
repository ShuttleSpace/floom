/// Neutral props shared by every design system's `ValidationMessages`.
///
/// Renders a reactive list of error strings (bound to `messages_signal()`)
/// in the theme `error` colour at a small font size, so it can sit directly
/// beneath a field. This is the Vuetify "input validation messages" slot.
#[derive(Debug, Clone)]
pub struct ValidationMessagesProps {
    /// Vertical gap between individual messages in dp.
    pub gap: f32,
    /// Font size for each message in dp.
    pub font_size: f32,
}

impl Default for ValidationMessagesProps {
    fn default() -> Self {
        Self {
            gap: 4.0,
            font_size: 12.0,
        }
    }
}
