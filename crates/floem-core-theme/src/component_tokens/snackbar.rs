#[derive(Debug, Clone, PartialEq)]
pub struct SnackbarTokens {
    /// Inner padding of the snackbar surface.
    pub padding: f32,
    /// Corner radius of the snackbar surface.
    pub border_radius: f32,
    /// Minimum height of the snackbar surface.
    pub min_height: f32,
    /// Body font size.
    pub font_size: f32,
    /// Maximum width of the snackbar surface.
    pub max_width: f32,
    /// Gap between the message and the action button.
    pub action_gap: f32,
}

impl Default for SnackbarTokens {
    fn default() -> Self {
        Self {
            padding: 14.0,
            border_radius: 8.0,
            min_height: 48.0,
            font_size: 14.0,
            max_width: 560.0,
            action_gap: 16.0,
        }
    }
}
