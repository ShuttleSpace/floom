#[derive(Debug, Clone)]
pub struct ProgressCircularProps {
    /// Progress percentage in the range 0.0..=100.0.
    pub value: f32,
    /// When true, ignore `value` and render an indeterminate animation.
    pub indeterminate: bool,
    /// Diameter of the indicator in dp.
    pub size: f32,
}

impl Default for ProgressCircularProps {
    fn default() -> Self {
        Self {
            value: 0.0,
            indeterminate: false,
            size: 36.0,
        }
    }
}
