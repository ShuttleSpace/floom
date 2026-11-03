#[derive(Debug, Clone, PartialEq)]
pub struct SpeedDialTokens {
    /// Size of the main floating action button in px.
    pub fab_size: f32,
    /// Size of each action button in px.
    pub action_size: f32,
    /// Gap between the FAB and the action buttons (and between actions) in px.
    pub gap: f32,
    /// Corner radius of the circular buttons in px.
    pub radius: f32,
    /// Open/close animation duration in ms.
    pub animation_ms: f32,
}

impl Default for SpeedDialTokens {
    fn default() -> Self {
        Self {
            fab_size: 56.0,
            action_size: 48.0,
            gap: 12.0,
            radius: 28.0,
            animation_ms: 180.0,
        }
    }
}
