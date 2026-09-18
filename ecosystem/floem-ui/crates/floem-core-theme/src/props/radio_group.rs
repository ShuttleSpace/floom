pub struct RadioGroupProps {
    /// Selectable option labels, in display order.
    pub options: Vec<String>,
    /// Index of the currently selected option (`None` = nothing selected).
    pub selected: Option<usize>,
    /// Whether every option is disabled.
    pub disabled: bool,
    /// Called with the selected index when the selection changes.
    pub on_change: Option<Box<dyn Fn(usize)>>,
}

impl Clone for RadioGroupProps {
    fn clone(&self) -> Self {
        Self {
            options: self.options.clone(),
            selected: self.selected,
            disabled: self.disabled,
            on_change: None,
        }
    }
}

impl std::fmt::Debug for RadioGroupProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RadioGroupProps")
            .field("options", &self.options)
            .field("selected", &self.selected)
            .field("disabled", &self.disabled)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(usize)"))
            .finish()
    }
}

impl Default for RadioGroupProps {
    fn default() -> Self {
        Self {
            options: Vec::new(),
            selected: None,
            disabled: false,
            on_change: None,
        }
    }
}
