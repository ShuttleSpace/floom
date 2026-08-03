use floem_core_theme::props::button::{ButtonProps, ButtonSize, ButtonVariant};

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentButtonStyle {
    pub design_system: &'static str,
}

pub struct FluentButton {
    props: ButtonProps,
}

impl FluentButton {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            props: ButtonProps {
                label: label.into(),
                ..Default::default()
            },
        }
    }
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.props.variant = variant;
        self
    }
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.props.size = size;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }
    pub fn loading(mut self, loading: bool) -> Self {
        self.props.loading = loading;
        self
    }
    pub fn on_click(mut self, callback: impl Fn() + 'static) -> Self {
        self.props.on_click = Some(Box::new(callback));
        self
    }
    pub fn props(&self) -> &ButtonProps {
        &self.props
    }
    pub fn resolved_style(&self) -> FluentButtonStyle {
        FluentButtonStyle {
            design_system: DESIGN_SYSTEM,
        }
    }
}
