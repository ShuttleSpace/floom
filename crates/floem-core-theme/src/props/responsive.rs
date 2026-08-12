#[derive(Debug, Clone, PartialEq)]
pub struct ResponsiveProps {
    /// Width / height ratio the container maintains (e.g. `16.0 / 9.0`).
    pub aspect_ratio: f32,
}

impl Default for ResponsiveProps {
    fn default() -> Self {
        Self {
            aspect_ratio: 1.0,
        }
    }
}
