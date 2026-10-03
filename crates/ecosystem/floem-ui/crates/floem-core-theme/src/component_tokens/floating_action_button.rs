#[derive(Debug, Clone, PartialEq)]
pub struct FloatingActionButtonTokens {
    /// Diameter of the circular FAB (also the height of an extended FAB).
    pub size: f32,
    /// Corner radius (9999 = full circle for the standard FAB).
    pub border_radius: f32,
    /// Elevation shadow level.
    pub elevation: u8,
    /// Size of the leading icon.
    pub icon_size: f32,
}

impl Default for FloatingActionButtonTokens {
    fn default() -> Self {
        Self {
            size: 56.0,
            border_radius: 9999.0,
            elevation: 3,
            icon_size: 24.0,
        }
    }
}
