use floem_core_theme::props::switch::SwitchProps;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoSwitchStyle {
    pub design_system: &'static str,
}

pub struct CupertinoSwitch {
    props: SwitchProps,
}

impl CupertinoSwitch {
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
    pub fn resolved_style(&self) -> CupertinoSwitchStyle {
        CupertinoSwitchStyle {
            design_system: DESIGN_SYSTEM,
        }
    }
}
impl Default for CupertinoSwitch {
    fn default() -> Self {
        Self::new()
    }
}
