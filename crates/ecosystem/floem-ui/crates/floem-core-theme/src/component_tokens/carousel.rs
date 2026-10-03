#[derive(Debug, Clone, PartialEq)]
pub struct CarouselTokens {
    /// Height of the slide viewport.
    pub height: f32,
    /// Diameter of an indicator dot.
    pub dot_size: f32,
    /// Gap between indicator dots.
    pub dot_gap: f32,
    /// Opacity of the inactive (non-current) dots.
    pub dot_inactive_opacity: f32,
    /// Size of the arrow controls.
    pub arrow_size: f32,
    /// Corner radius of the slide surface.
    pub border_radius: f32,
    /// Gap between the arrows and the viewport edge.
    pub slide_gap: f32,
}

impl Default for CarouselTokens {
    fn default() -> Self {
        Self {
            height: 200.0,
            dot_size: 8.0,
            dot_gap: 6.0,
            dot_inactive_opacity: 0.4,
            arrow_size: 28.0,
            border_radius: 12.0,
            slide_gap: 8.0,
        }
    }
}
