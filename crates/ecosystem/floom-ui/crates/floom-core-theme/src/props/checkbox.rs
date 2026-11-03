pub struct CheckboxProps {
    pub label: String,
    pub checked: bool,
    pub disabled: bool,
    pub on_change: Option<Box<dyn Fn(bool)>>,
}
impl Clone for CheckboxProps {
    fn clone(&self) -> Self {
        Self {
            label: self.label.clone(),
            checked: self.checked,
            disabled: self.disabled,
            on_change: None,
        }
    }
}
impl std::fmt::Debug for CheckboxProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheckboxProps")
            .field("label", &self.label)
            .field("checked", &self.checked)
            .field("disabled", &self.disabled)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(bool)"))
            .finish()
    }
}
impl Default for CheckboxProps {
    fn default() -> Self {
        Self {
            label: String::new(),
            checked: false,
            disabled: false,
            on_change: None,
        }
    }
}
