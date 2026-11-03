/// Neutral props shared by every design system's `Stepper`.
///
/// A `Stepper` renders an ordered, horizontally-laid-out progression of step
/// markers (circular) connected by a line. The current step is held in an
/// internal `RwSignal<usize>`; selection can be driven by the consumer and a
/// click on an eligible step fires the `on_change` callback (mirrors the
/// `selected`/`on_change` pattern used by `BottomNavigation`).
#[derive(Debug, Clone)]
pub struct StepperProps {
    /// Gap between step markers in dp.
    pub gap: f32,
    /// Width of the connector line between steps in dp.
    pub connector_width: f32,
    /// Diameter of each step circle in dp.
    pub step_size: f32,
    /// Label font size in dp.
    pub label_font_size: f32,
    /// Whether completed/earlier steps can be clicked to navigate back.
    pub editable: bool,
    /// Allow jumping to any step (not just the next one) when clicked.
    pub non_linear: bool,
}

impl Default for StepperProps {
    fn default() -> Self {
        Self {
            gap: 16.0,
            connector_width: 2.0,
            step_size: 32.0,
            label_font_size: 13.0,
            editable: false,
            non_linear: false,
        }
    }
}
