#[derive(Debug, Clone, PartialEq)]
pub struct RatingTokens {
    /// Diameter of each star in dp.
    pub star_size: f32,
    /// Gap between stars in dp.
    pub gap: f32,
}

impl Default for RatingTokens {
    fn default() -> Self {
        Self {
            star_size: 24.0,
            gap: 4.0,
        }
    }
}
