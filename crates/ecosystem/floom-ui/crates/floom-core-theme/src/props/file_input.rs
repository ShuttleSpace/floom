pub struct FileInputProps {
    /// Label of the trigger button.
    pub label: String,
    /// Optional accept filter (e.g. ".png,image/*") — hint only, not enforced by the view.
    pub accept: String,
    /// Hint whether multiple files may be selected (not enforced by the view).
    pub multiple: bool,
    /// Whether the control is interactive.
    pub disabled: bool,
    /// Called with the selected file path (or `None` when cleared).
    pub on_change: Option<Box<dyn Fn(Option<String>)>>,
}

impl Clone for FileInputProps {
    fn clone(&self) -> Self {
        Self {
            label: self.label.clone(),
            accept: self.accept.clone(),
            multiple: self.multiple,
            disabled: self.disabled,
            on_change: None,
        }
    }
}

impl std::fmt::Debug for FileInputProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileInputProps")
            .field("label", &self.label)
            .field("accept", &self.accept)
            .field("multiple", &self.multiple)
            .field("disabled", &self.disabled)
            .field(
                "on_change",
                &self.on_change.as_ref().map(|_| "Fn(Option<String>)"),
            )
            .finish()
    }
}

impl Default for FileInputProps {
    fn default() -> Self {
        Self {
            label: "Upload".to_string(),
            accept: String::new(),
            multiple: false,
            disabled: false,
            on_change: None,
        }
    }
}
