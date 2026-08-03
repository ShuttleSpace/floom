use floem::reactive::{RwSignal, SignalGet, SignalUpdate};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::text_field::TextFieldProps;
use floem_core_theme::theme::Theme;
use floem_core_theme::tokens::color::Color;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdTextFieldStyle {
    pub border_color: Color,
    pub text_color: Color,
    pub height: f32,
    pub padding_horizontal: f32,
    pub border_radius: f32,
}

pub struct MdTextField {
    props: TextFieldProps,
    value: RwSignal<String>,
    focused: RwSignal<bool>,
    theme: RwSignal<floem_core_theme::theme::MaterialTheme>,
}

impl MdTextField {
    pub fn new() -> Self {
        let props = TextFieldProps::default();
        Self {
            value: RwSignal::new(props.value.clone()),
            focused: RwSignal::new(false),
            theme: theme_signal(),
            props,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.props.placeholder = placeholder.into();
        self
    }

    pub fn initial_value(mut self, value: impl Into<String>) -> Self {
        let value = self.clamp_value(value.into());
        self.value.set(value.clone());
        self.props.value = value;
        self
    }

    pub fn password(mut self, is_password: bool) -> Self {
        self.props.is_password = is_password;
        self
    }

    pub fn max_length(mut self, max_length: usize) -> Self {
        self.props.max_length = Some(max_length);
        let value = self.clamp_value(self.props.value.clone());
        self.value.set(value.clone());
        self.props.value = value;
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

    pub fn set_value(&mut self, value: impl Into<String>) {
        let value = self.clamp_value(value.into());
        self.value.set(value.clone());
        self.props.value = value.clone();
        if let Some(callback) = &self.props.on_change {
            callback(&value);
        }
    }

    pub fn value(&self) -> String {
        self.value.get()
    }

    pub fn set_focused(&self, focused: bool) {
        self.focused.set(focused);
    }

    pub fn is_focused(&self) -> bool {
        self.focused.get()
    }

    pub fn props(&self) -> &TextFieldProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<floem_core_theme::theme::MaterialTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: floem_core_theme::theme::MaterialTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdTextFieldStyle {
        let theme = self.theme.get();
        let ComponentTokens::TextField(tokens) = theme.component(ComponentId::TextField) else {
            unreachable!("ComponentId::TextField must resolve to TextFieldTokens")
        };
        MdTextFieldStyle {
            border_color: if self.props.disabled {
                Color {
                    a: 97,
                    ..theme.colors().on_surface
                }
            } else if self.is_focused() {
                theme.colors().primary
            } else {
                theme.colors().outline
            },
            text_color: if self.props.disabled {
                Color {
                    a: 97,
                    ..theme.colors().on_surface
                }
            } else {
                theme.colors().on_surface
            },
            height: tokens.height,
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,
        }
    }

    fn clamp_value(&self, value: String) -> String {
        match self.props.max_length {
            Some(max_length) => value.chars().take(max_length).collect(),
            None => value,
        }
    }
}

impl Default for MdTextField {
    fn default() -> Self {
        Self::new()
    }
}
