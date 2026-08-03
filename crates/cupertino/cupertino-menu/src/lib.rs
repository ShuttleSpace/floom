use floem_core_theme::props::menu::{MenuItem, MenuProps};

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoMenuStyle {
    pub design_system: &'static str,
}

pub struct CupertinoMenu {
    props: MenuProps,
}

impl CupertinoMenu {
    pub fn new() -> Self {
        Self {
            props: MenuProps::default(),
        }
    }
    pub fn item(mut self, label: impl Into<String>, callback: impl Fn() + 'static) -> Self {
        self.props.items.push(MenuItem {
            label: label.into(),
            on_select: Some(Box::new(callback)),
        });
        self
    }
    pub fn open(mut self, open: bool) -> Self {
        self.props.open = open;
        self
    }
    pub fn props(&self) -> &MenuProps {
        &self.props
    }
    pub fn resolved_style(&self) -> CupertinoMenuStyle {
        CupertinoMenuStyle {
            design_system: DESIGN_SYSTEM,
        }
    }
}
impl Default for CupertinoMenu {
    fn default() -> Self {
        Self::new()
    }
}
