#[derive(Debug, Clone, PartialEq)]
pub struct IconButtonTokens {
    /// Outer button size (width & height) in px.
    pub size: f32,
    /// Rendered icon size in px.
    pub icon_size: f32,
    /// Corner radius in px.
    pub radius: f32,
    /// Inner padding in px.
    pub padding: f32,
}

impl Default for IconButtonTokens {
    fn default() -> Self {
        Self {
            size: 40.0,
            icon_size: 24.0,
            radius: 20.0,
            padding: 8.0,
        }
    }
}
