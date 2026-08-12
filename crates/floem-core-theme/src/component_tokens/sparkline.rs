#[derive(Debug, Clone, PartialEq)]
pub struct SparklineTokens {
    /// Default rendered width.
    pub width: f32,
    /// Default rendered height.
    pub height: f32,
    /// Bar width in px.
    pub line_width: f32,
    /// Translucent fill alpha (0..=255) used when `fill` is set.
    pub fill_opacity: f32,
}

impl Default for SparklineTokens {
    fn default() -> Self {
        Self {
            width: 200.0,
            height: 48.0,
            line_width: 4.0,
            fill_opacity: 90.0,
        }
    }
}
