#[derive(Debug, Clone)]
pub struct TooltipProps {
    /// Text shown in the floating tooltip when the trigger is hovered.
    pub text: String,
}

impl Default for TooltipProps {
    fn default() -> Self {
        Self {
            text: String::new(),
        }
    }
}
