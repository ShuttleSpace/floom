/// Shared, design-system-agnostic props for the `ExpansionPanels` component.
pub struct ExpansionPanelsProps {
    /// Allow more than one panel to be open at the same time.
    pub multiple: bool,
    /// Called with `(panel_index, expanded)` whenever a panel's state changes.
    pub on_change: Option<Box<dyn Fn(usize, bool)>>,
}

impl Clone for ExpansionPanelsProps {
    fn clone(&self) -> Self {
        Self {
            multiple: self.multiple,
            on_change: None,
        }
    }
}

impl std::fmt::Debug for ExpansionPanelsProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExpansionPanelsProps")
            .field("multiple", &self.multiple)
            .field(
                "on_change",
                &self.on_change.as_ref().map(|_| "Fn(usize, bool)"),
            )
            .finish()
    }
}

impl Default for ExpansionPanelsProps {
    fn default() -> Self {
        Self {
            multiple: false,
            on_change: None,
        }
    }
}
