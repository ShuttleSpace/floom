/// Shared, design-system-agnostic props for the `CheckboxBtn` component.
pub struct CheckboxBtnProps {
    /// Initial checked state.
    pub checked: bool,
    /// Whether the button is non-interactive.
    pub disabled: bool,
    /// Optional text label rendered inside the button.
    pub label: Option<String>,
    /// Called with the new checked state whenever it toggles.
    pub on_change: Option<Box<dyn Fn(bool)>>,
}

impl Clone for CheckboxBtnProps {
    fn clone(&self) -> Self {
        Self {
            checked: self.checked,
            disabled: self.disabled,
            label: self.label.clone(),
            on_change: None,
        }
    }
}

impl std::fmt::Debug for CheckboxBtnProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheckboxBtnProps")
            .field("checked", &self.checked)
            .field("disabled", &self.disabled)
            .field("label", &self.label)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(bool)"))
            .finish()
    }
}

impl Default for CheckboxBtnProps {
    fn default() -> Self {
        Self {
            checked: false,
            disabled: false,
            label: None,
            on_change: None,
        }
    }
}
