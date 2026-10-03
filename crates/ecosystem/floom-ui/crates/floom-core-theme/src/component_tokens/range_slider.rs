#[derive(Debug, Clone, PartialEq)]
pub struct RangeSliderTokens {
    /// Height of the track line.
    pub track_height: f32,
    /// Diameter of each draggable thumb.
    pub thumb_size: f32,
    /// Corner radius of the thumb (large = circular).
    pub thumb_radius: f32,
}

impl Default for RangeSliderTokens {
    fn default() -> Self {
        Self {
            track_height: 4.0,
            thumb_size: 20.0,
            thumb_radius: 9999.0,
        }
    }
}
