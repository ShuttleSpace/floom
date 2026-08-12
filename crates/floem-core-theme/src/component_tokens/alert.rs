#[derive(Debug, Clone, PartialEq)]
pub struct AlertTokens {
    /// Inner padding of the alert surface.
    pub padding: f32,
    /// Corner radius of the alert surface.
    pub border_radius: f32,
    /// Body message font size.
    pub font_size: f32,
    /// Title font size.
    pub title_font_size: f32,
}

impl Default for AlertTokens {
    fn default() -> Self {
        Self {
            padding: 12.0,
            border_radius: 8.0,
            font_size: 14.0,
            title_font_size: 16.0,
        }
    }
}
