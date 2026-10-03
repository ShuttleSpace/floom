pub struct SelectProps {
    /// Selectable options shown in the dropdown.
    pub options: Vec<String>,
    /// Index of the currently selected option (`None` = placeholder).
    pub selected: Option<usize>,
    /// Text shown when nothing is selected.
    pub placeholder: String,
    /// Whether the select is interactive.
    pub disabled: bool,
    /// Called with the new selected index (or `None`) when the choice changes.
    pub on_change: Option<Box<dyn Fn(Option<usize>)>>,
}

impl Clone for SelectProps {
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

impl std::fmt::Debug for SelectProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectProps")
            .field("options", &self.options)
            .field("selected", &self.selected)
            .field("placeholder", &self.placeholder)
            .field("disabled", &self.disabled)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(Option<usize>)"))
            .finish()
    }
}

impl Default for SelectProps {
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
