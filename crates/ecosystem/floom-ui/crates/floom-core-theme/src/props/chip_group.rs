/// Shared, design-system-agnostic props for the `ChipGroup` component.
pub struct ChipGroupProps {
    /// Allow multiple chips to be selected simultaneously.
    pub multiple: bool,
    /// Require at least one chip to remain selected.
    pub mandatory: bool,
    /// Called with the full set of selected indices whenever selection changes.
    pub on_change: Option<Box<dyn Fn(Vec<usize>)>>,
}

impl Clone for ChipGroupProps {
    fn clone(&self) -> Self {
        Self {
            multiple: self.multiple,
            mandatory: self.mandatory,
            on_change: None,
        }
    }
}

impl std::fmt::Debug for ChipGroupProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChipGroupProps")
            .field("multiple", &self.multiple)
            .field("mandatory", &self.mandatory)
            .field(
                "on_change",
                &self.on_change.as_ref().map(|_| "Fn(Vec<usize>)"),
            )
            .finish()
    }
}

impl Default for ChipGroupProps {
    fn default() -> Self {
        Self {
            multiple: false,
            mandatory: false,
            on_change: None,
        }
    }
}
