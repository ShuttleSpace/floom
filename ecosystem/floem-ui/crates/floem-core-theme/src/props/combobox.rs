pub struct ComboboxProps {
    /// Selectable options shown (filtered) in the searchable dropdown.
    pub options: Vec<String>,
    /// Index of the currently selected option (`None` = placeholder).
    pub selected: Option<usize>,
    /// Text shown when nothing is selected.
    pub placeholder: String,
    /// Whether the combobox is interactive.
    pub disabled: bool,
    /// Called with the new selected index (or `None`) when the choice changes.
    pub on_change: Option<Box<dyn Fn(Option<usize>)>>,
}

impl Clone for ComboboxProps {
    fn clone(&self) -> Self {
        Self {
            options: self.options.clone(),
            selected: self.selected,
            placeholder: self.placeholder.clone(),
            disabled: self.disabled,
            on_change: None,
        }
    }
}

impl std::fmt::Debug for ComboboxProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComboboxProps")
            .field("options", &self.options)
            .field("selected", &self.selected)
            .field("placeholder", &self.placeholder)
            .field("disabled", &self.disabled)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(Option<usize>)"))
            .finish()
    }
}

impl Default for ComboboxProps {
    fn default() -> Self {
        Self {
            options: Vec::new(),
            selected: None,
            placeholder: "Select…".to_string(),
            disabled: false,
            on_change: None,
        }
    }
}
