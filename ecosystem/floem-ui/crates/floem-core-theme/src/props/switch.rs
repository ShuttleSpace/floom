pub struct SwitchProps {
    pub checked: bool,
    pub disabled: bool,
    pub on_change: Option<Box<dyn Fn(bool)>>,
}
impl Clone for SwitchProps {
    fn clone(&self) -> Self {
        Self {
            checked: self.checked,
            disabled: self.disabled,
            on_change: None,
        }
    }
}
impl std::fmt::Debug for SwitchProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SwitchProps")
            .field("checked", &self.checked)
            .field("disabled", &self.disabled)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(bool)"))
            .finish()
    }
}
impl Default for SwitchProps {
    fn default() -> Self {
        Self {
            checked: false,
            disabled: false,
            on_change: None,
        }
    }
}
