/// Neutral props shared by every design system's `Spacer`.
///
/// A spacer is an empty, flex-growing view that pushes siblings apart inside a
/// flex row/column. It mirrors Vuetify's `<v-spacer>` element.
#[derive(Debug, Clone)]
pub struct SpacerProps {
    /// Flex grow factor applied to the empty spacer view.
    pub grow: f32,
}

impl Default for SpacerProps {
    fn default() -> Self {
        Self { grow: 1.0 }
    }
}
