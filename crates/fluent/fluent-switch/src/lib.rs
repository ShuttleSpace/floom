use floem_core_theme::props::switch::SwitchProps;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentSwitchStyle {
    pub design_system: &'static str,
}

pub struct FluentSwitch {
    props: SwitchProps,
}

impl FluentSwitch {
    pub fn new() -> Self {
        Self {
            props: SwitchProps::default(),
        }
    }
    pub fn checked(mut self, checked: bool) -> Self {
        self.props.checked = checked;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }
    pub fn on_change(mut self, callback: impl Fn(bool) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }
    pub fn props(&self) -> &SwitchProps {
        &self.props
    }
    pub fn resolved_style(&self) -> FluentSwitchStyle {
        FluentSwitchStyle {
            design_system: DESIGN_SYSTEM,
        }
    }
}
impl Default for FluentSwitch {
    fn default() -> Self {
        Self::new()
    }
}
