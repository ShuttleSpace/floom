#[derive(Debug, Clone)]
pub struct AppBarProps {
    /// Title text shown in the bar (centered-left after the leading slot).
    pub title: String,
    /// Elevation/shadow depth of the bar.
    pub elevation: u8,
    /// Height of the bar in dp.
    pub height: f32,
}

impl Default for AppBarProps {
    fn default() -> Self {
        Self {
            title: String::new(),
            elevation: 2,
            height: 56.0,
        }
    }
}
