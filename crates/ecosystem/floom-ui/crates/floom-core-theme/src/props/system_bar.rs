/// Neutral props shared by every design system's `SystemBar`.
///
/// A system bar is a thin status strip (window controls, clock, battery, etc.)
/// at the very top of the application shell. It mirrors Vuetify's `v-system-bar`.
#[derive(Debug, Clone)]
pub struct SystemBarProps {
    /// Bar height in dp. `None` defers to the active design system's
    /// `SystemBarTokens::height` (M3 24 / Cupertino 22 / Fluent 28).
    pub height: Option<f32>,
    /// Dark variant: uses a darker, higher-contrast background.
    pub dark: bool,
    /// Horizontal padding inside the bar. `None` defers to the active design
    /// system's `SystemBarTokens::pad` (M3 8 / Cupertino 6 / Fluent 8).
    pub pad: Option<f32>,
}

impl Default for SystemBarProps {
    fn default() -> Self {
        Self {
            height: None,
            dark: false,
            pad: None,
        }
    }
}
