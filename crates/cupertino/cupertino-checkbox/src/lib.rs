use floem_core_theme::props::checkbox::CheckboxProps;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoCheckboxStyle {
    pub design_system: &'static str,
}

pub struct CupertinoCheckbox {
    props: CheckboxProps,
}

impl CupertinoCheckbox {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            props: CheckboxProps {
                label: label.into(),
                ..Default::default()
            },
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
    pub fn props(&self) -> &CheckboxProps {
        &self.props
    }
    pub fn resolved_style(&self) -> CupertinoCheckboxStyle {
        CupertinoCheckboxStyle {
            design_system: DESIGN_SYSTEM,
        }
    }
}
