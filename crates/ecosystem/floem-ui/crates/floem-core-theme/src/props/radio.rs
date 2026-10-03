pub struct RadioProps {
    pub label: String,
    pub value: String,
    pub checked: bool,
    pub disabled: bool,
    pub on_change: Option<Box<dyn Fn(bool)>>,
}
impl Clone for RadioProps {
    fn clone(&self) -> Self {
        Self {
            label: self.label.clone(),
            value: self.value.clone(),
            checked: self.checked,
            disabled: self.disabled,
            on_change: None,
        }
    }
}
impl std::fmt::Debug for RadioProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RadioProps")
            .field("label", &self.label)
            .field("value", &self.value)
            .field("checked", &self.checked)
            .field("disabled", &self.disabled)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(bool)"))
            .finish()
    }
}
impl Default for RadioProps {
    fn default() -> Self {
        Self {
            label: String::new(),
            value: String::new(),
            checked: false,
            disabled: false,
            on_change: None,
        }
    }
}
