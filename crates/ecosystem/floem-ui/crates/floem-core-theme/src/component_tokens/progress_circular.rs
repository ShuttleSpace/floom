#[derive(Debug, Clone, PartialEq)]
pub struct ProgressCircularTokens {
    /// Diameter of the indicator in dp.
    pub size: f32,
    /// Thickness of the progress stroke in dp.
    pub stroke_width: f32,
}

impl Default for ProgressCircularTokens {
    fn default() -> Self {
        Self {
            size: 48.0,
            stroke_width: 4.0,
        }
    }
}
