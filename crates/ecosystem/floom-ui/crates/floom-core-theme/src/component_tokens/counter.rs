#[derive(Debug, Clone, PartialEq)]
pub struct CounterTokens {
    /// Size of the − / + step buttons in px.
    pub button_size: f32,
    /// Total width of the control in px.
    pub width: f32,
    /// Total height of the control in px.
    pub height: f32,
    /// Corner radius in px.
    pub radius: f32,
    /// Gap between the buttons and the value readout in px.
    pub gap: f32,
    /// Border width in px.
    pub border: f32,
}

impl Default for CounterTokens {
    fn default() -> Self {
        Self {
            button_size: 32.0,
            width: 140.0,
            height: 40.0,
            radius: 8.0,
            gap: 4.0,
            border: 1.0,
        }
    }
}
