#[derive(Debug, Clone, PartialEq)]
pub struct ProgressLinearTokens {
    /// Height of the track in dp.
    pub track_height: f32,
    /// Corner radius of the track and fill.
    pub border_radius: f32,
}

impl Default for ProgressLinearTokens {
    fn default() -> Self {
        Self {
            track_height: 4.0,
            border_radius: 2.0,
        }
    }
}
