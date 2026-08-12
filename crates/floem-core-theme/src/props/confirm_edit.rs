/// Shared, design-system-agnostic props for the `ConfirmEdit` component.
pub struct ConfirmEditProps {
    /// The value being edited (displayed when not in edit mode).
    pub value: String,
    /// Called with the new value when the user confirms the edit.
    pub on_save: Option<Box<dyn Fn(String)>>,
}

impl Clone for ConfirmEditProps {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            on_save: None,
        }
    }
}

impl std::fmt::Debug for ConfirmEditProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfirmEditProps")
            .field("value", &self.value)
            .field("on_save", &self.on_save.as_ref().map(|_| "Fn(String)"))
            .finish()
    }
}

impl Default for ConfirmEditProps {
    fn default() -> Self {
        Self {
            value: String::new(),
            on_save: None,
        }
    }
}
