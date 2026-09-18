pub struct AutocompleteProps {
    /// Selectable options suggested (filtered) in the searchable dropdown.
    pub options: Vec<String>,
    /// Index of the currently selected option (`None` = free text / placeholder).
    pub selected: Option<usize>,
    /// Text shown when nothing is selected.
    pub placeholder: String,
    /// Whether the autocomplete is interactive.
    pub disabled: bool,
    /// Called with the committed text (option text or free-typed text, `None` if cleared).
    pub on_change: Option<Box<dyn Fn(Option<String>)>>,
}

impl Clone for AutocompleteProps {
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

impl std::fmt::Debug for AutocompleteProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AutocompleteProps")
            .field("options", &self.options)
            .field("selected", &self.selected)
            .field("placeholder", &self.placeholder)
            .field("disabled", &self.disabled)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(Option<String>)"))
            .finish()
    }
}

impl Default for AutocompleteProps {
    fn default() -> Self {
        Self {
            options: Vec::new(),
            selected: None,
            placeholder: "Type to search…".to_string(),
            disabled: false,
            on_change: None,
        }
    }
}
