pub struct SnackbarProps {
    /// Body message.
    pub message: String,
    /// Optional action button label (e.g. "Undo"). `None` hides the action.
    pub action_label: Option<String>,
    /// Called when the action button is clicked.
    pub on_action: Option<Box<dyn Fn()>>,
    /// Auto-dismiss duration in milliseconds. `0` means it stays until dismissed.
    pub duration_ms: u32,
}

impl Clone for SnackbarProps {
    fn clone(&self) -> Self {
        Self {
            message: self.message.clone(),
            action_label: self.action_label.clone(),
            on_action: None,
            duration_ms: self.duration_ms,
        }
    }
}

impl std::fmt::Debug for SnackbarProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SnackbarProps")
            .field("message", &self.message)
            .field("action_label", &self.action_label)
            .field("on_action", &self.on_action.as_ref().map(|_| "Fn()"))
            .field("duration_ms", &self.duration_ms)
            .finish()
    }
}

impl Default for SnackbarProps {
    fn default() -> Self {
        Self {
            message: String::new(),
            action_label: None,
            on_action: None,
            duration_ms: 4000,
        }
    }
}
