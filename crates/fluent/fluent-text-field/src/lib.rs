use floem_core_theme::props::text_field::TextFieldProps;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentTextFieldStyle {
    pub design_system: &'static str,
}

pub struct FluentTextField {
    props: TextFieldProps,
}

impl FluentTextField {
    pub fn new() -> Self {
        Self {
            props: TextFieldProps::default(),
        }
    }
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.props.placeholder = placeholder.into();
        self
    }
    pub fn initial_value(mut self, value: impl Into<String>) -> Self {
        self.props.value = value.into();
        self
    }
    pub fn password(mut self, is_password: bool) -> Self {
        self.props.is_password = is_password;
        self
    }
    pub fn max_length(mut self, max_length: usize) -> Self {
        self.props.max_length = Some(max_length);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }
    pub fn on_change(mut self, callback: impl Fn(&str) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }
    pub fn props(&self) -> &TextFieldProps {
        &self.props
    }
    pub fn resolved_style(&self) -> FluentTextFieldStyle {
        FluentTextFieldStyle {
            design_system: DESIGN_SYSTEM,
        }
    }
}
impl Default for FluentTextField {
    fn default() -> Self {
        Self::new()
    }
}
