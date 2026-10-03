/// Neutral props shared by every design system's `Container`.
///
/// A container centers its content and caps its maximum width, matching the
/// Vuetify `v-container` behaviour. Set `fluid` to let it span the full width.
#[derive(Debug, Clone)]
pub struct ContainerProps {
    /// Maximum content width in dp. Ignored when `fluid` is true.
    pub max_width: f32,
    /// Horizontal padding inside the container.
    pub padding: f32,
    /// When true the container spans the full available width (no max-width cap).
    pub fluid: bool,
}

impl Default for ContainerProps {
    fn default() -> Self {
        Self {
            max_width: 1200.0,
            padding: 16.0,
            fluid: false,
        }
    }
}
