#[derive(Debug, Clone, PartialEq)]
pub struct StepperTokens {
    /// Diameter of each step circle in dp.
    pub step_size: f32,
    /// Width of the connector line between steps in dp.
    pub connector_width: f32,
    /// Gap between step markers in dp.
    pub gap: f32,
    /// Label font size in dp.
    pub label_font_size: f32,
}

impl Default for StepperTokens {
    fn default() -> Self {
        Self {
            step_size: 32.0,
            connector_width: 2.0,
            gap: 16.0,
            label_font_size: 13.0,
        }
    }
}
