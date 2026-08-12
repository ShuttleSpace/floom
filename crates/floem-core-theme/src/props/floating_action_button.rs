pub struct FloatingActionButtonProps {
    /// Optional text label; when non-empty the FAB renders as an extended FAB.
    pub label: String,
    /// Whether to render the extended (label + icon) variant when a label is set.
    pub extended: bool,
    /// Whether the FAB is interactive.
    pub disabled: bool,
    /// Called when the FAB is activated.
    pub on_click: Option<Box<dyn Fn()>>,
}

impl Clone for FloatingActionButtonProps {
    fn clone(&self) -> Self {
        Self {
            label: self.label.clone(),
            extended: self.extended,
            disabled: self.disabled,
            on_click: None,
        }
    }
}

impl std::fmt::Debug for FloatingActionButtonProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FloatingActionButtonProps")
            .field("label", &self.label)
            .field("extended", &self.extended)
            .field("disabled", &self.disabled)
            .field("on_click", &self.on_click.as_ref().map(|_| "Fn()"))
            .finish()
    }
}

impl Default for FloatingActionButtonProps {
    fn default() -> Self {
        Self {
            label: String::new(),
            extended: false,
            disabled: false,
            on_click: None,
        }
    }
}
