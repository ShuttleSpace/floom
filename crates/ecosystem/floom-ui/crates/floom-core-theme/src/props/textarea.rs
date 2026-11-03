pub struct TextareaProps {
    /// Current text content.
    pub value: String,
    /// Placeholder shown when empty.
    pub placeholder: String,
    /// Whether the textarea is editable.
    pub disabled: bool,
    /// Visible number of rows (drives min height).
    pub rows: usize,
    /// Called with the new text on every edit.
    pub on_change: Option<Box<dyn Fn(String)>>,
}

impl Clone for TextareaProps {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            placeholder: self.placeholder.clone(),
            disabled: self.disabled,
            rows: self.rows,
            on_change: None,
        }
    }
}

impl std::fmt::Debug for TextareaProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextareaProps")
            .field("value", &self.value)
            .field("placeholder", &self.placeholder)
            .field("disabled", &self.disabled)
            .field("rows", &self.rows)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(String)"))
            .finish()
    }
}

impl Default for TextareaProps {
    fn default() -> Self {
        Self {
            value: String::new(),
            placeholder: "Type here…".to_string(),
            disabled: false,
            rows: 3,
            on_change: None,
        }
    }
}
