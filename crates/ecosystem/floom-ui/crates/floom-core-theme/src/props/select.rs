pub struct SelectProps {
    /// Selectable options shown in the dropdown.
    pub options: Vec<String>,
    /// Index of the currently selected option (`None` = placeholder).
    pub selected: Option<usize>,
    /// Text shown when nothing is selected.
    pub placeholder: String,
    /// Whether the select is interactive.
    pub disabled: bool,
    /// Called with the index that was just chosen.
    ///
    /// A plain index rather than v1's `Option<usize>`: v1's select held a
    /// `RwSignal<Option<usize>>` that could be cleared, and the engine's cannot
    /// -- `Select::choose` writes `Some(index)` on every successful pick, so
    /// there is no "unchoose" state to carry. `Send + Sync` because the widget
    /// a kit builds out of these props is shared across threads, so a closure
    /// that moves into it has to be both. This is the same shape
    /// `CounterProps::on_change` uses, for the same reason.
    pub on_change: Option<Box<dyn Fn(usize) + Send + Sync>>,
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
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(usize)"))
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
