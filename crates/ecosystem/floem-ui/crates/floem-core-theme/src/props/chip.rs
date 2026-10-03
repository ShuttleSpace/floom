pub struct ChipProps {
    /// Display label.
    pub label: String,
    /// Whether the chip is in its selected/filtered state.
    pub selected: bool,
    /// Whether the chip is interactive.
    pub disabled: bool,
    /// Called when the chip is clicked.
    pub on_click: Option<Box<dyn Fn()>>,
}

impl Clone for ChipProps {
    fn clone(&self) -> Self {
        Self {
            label: self.label.clone(),
            selected: self.selected,
            disabled: self.disabled,
            on_click: None,
        }
    }
}

impl std::fmt::Debug for ChipProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChipProps")
            .field("label", &self.label)
            .field("selected", &self.selected)
            .field("disabled", &self.disabled)
            .field("on_click", &self.on_click.as_ref().map(|_| "Fn()"))
            .finish()
    }
}

impl Default for ChipProps {
    fn default() -> Self {
        Self {
            label: String::new(),
            selected: false,
            disabled: false,
            on_click: None,
        }
    }
}
