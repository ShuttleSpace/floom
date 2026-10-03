#[derive(Debug, Clone)]
pub struct AvatarProps {
    /// Initials or short text shown when there is no image `child`.
    pub content: String,
    /// Diameter of the avatar circle in dp.
    pub size: f32,
}

impl Default for AvatarProps {
    fn default() -> Self {
        Self {
            content: String::new(),
            size: 40.0,
        }
    }
}
