#[derive(Debug, Clone)]
pub struct ProgressLinearProps {
    /// Progress percentage in the range 0.0..=100.0.
    pub value: f32,
    /// When true, ignore `value` and render an indeterminate animation.
    pub indeterminate: bool,
    /// Height of the track in dp.
    pub height: f32,
}

impl Default for ProgressLinearProps {
    fn default() -> Self {
        Self {
            value: 0.0,
            indeterminate: false,
            height: 4.0,
        }
    }
}
