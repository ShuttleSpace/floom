#[derive(Debug, Clone, PartialEq)]
pub struct AvatarTokens {
    /// Diameter of the avatar circle in dp.
    pub size: f32,
    /// Font size of the initials text.
    pub font_size: f32,
}

impl Default for AvatarTokens {
    fn default() -> Self {
        Self {
            size: 40.0,
            font_size: 16.0,
        }
    }
}
