/// Neutral props shared by every design system's `SystemBar`.
///
/// A system bar is a thin status strip (window controls, clock, battery, etc.)
/// at the very top of the application shell. It mirrors Vuetify's `v-system-bar`.
#[derive(Debug, Clone)]
pub struct SystemBarProps {
    /// Bar height in dp.
    pub height: f32,
    /// Dark variant: uses a darker, higher-contrast background.
    pub dark: bool,
    /// Horizontal padding inside the bar.
    pub pad: f32,
}

impl Default for SystemBarProps {
    fn default() -> Self {
        Self {
            height: 24.0,
            dark: false,
            pad: 8.0,
        }
    }
}
