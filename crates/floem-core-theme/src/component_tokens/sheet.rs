use crate::tokens::color::Color;

#[derive(Debug, Clone, PartialEq)]
pub struct SheetTokens {
    /// Corner radius of the panel (applied on the anchored edge(s)).
    pub corner_radius: f32,
    /// Inner padding of the panel content.
    pub padding: f32,
    /// Width of start/end (side) sheets.
    pub side_width: f32,
    /// Elevation shadow blur.
    pub shadow_blur: f32,
    /// Elevation shadow spread.
    pub shadow_spread: f32,
    /// Scrim colour drawn behind the panel.
    pub scrim_color: Color,
}

impl Default for SheetTokens {
    fn default() -> Self {
        Self {
            corner_radius: 28.0,
            padding: 16.0,
            side_width: 360.0,
            shadow_blur: 24.0,
            shadow_spread: 1.0,
            scrim_color: Color::rgba(0, 0, 0, 128),
        }
    }
}
