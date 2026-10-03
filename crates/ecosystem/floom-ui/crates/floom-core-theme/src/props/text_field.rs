pub struct TextFieldProps {
    pub placeholder: String,
    pub value: String,
    pub is_password: bool,
    pub max_length: Option<usize>,
    pub disabled: bool,
    pub on_change: Option<Box<dyn Fn(&str)>>,
}
impl Clone for TextFieldProps {
    fn clone(&self) -> Self {
        Self {
            placeholder: self.placeholder.clone(),
            value: self.value.clone(),
            is_password: self.is_password,
            max_length: self.max_length,
            disabled: self.disabled,
            on_change: None,
        }
    }
}
impl std::fmt::Debug for TextFieldProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextFieldProps")
            .field("placeholder", &self.placeholder)
            .field("value", &self.value)
            .field("is_password", &self.is_password)
            .field("disabled", &self.disabled)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(&str)"))
            .finish()
    }
}
impl Default for TextFieldProps {
    fn default() -> Self {
        Self {
            placeholder: String::new(),
            value: String::new(),
            is_password: false,
            max_length: None,
            disabled: false,
            on_change: None,
        }
    }
}
