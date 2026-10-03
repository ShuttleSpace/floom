#[derive(Debug, Clone, PartialEq)]
pub struct FormTokens {
    /// Inner padding of the form surface in dp.
    pub pad: f32,
    /// Vertical gap between stacked children in dp.
    pub gap: f32,
    /// Height of the submit button in dp.
    pub submit_height: f32,
    /// Corner radius of the submit button in dp.
    pub submit_radius: f32,
}

impl Default for FormTokens {
    fn default() -> Self {
        Self {
            pad: 16.0,
            gap: 12.0,
            submit_height: 40.0,
            submit_radius: 8.0,
        }
    }
}
