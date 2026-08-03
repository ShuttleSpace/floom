use floem_core_theme::props::menu::{MenuItem, MenuProps};

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentMenuStyle {
    pub design_system: &'static str,
}

pub struct FluentMenu {
    props: MenuProps,
}

impl FluentMenu {
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
    pub fn resolved_style(&self) -> FluentMenuStyle {
        FluentMenuStyle {
            design_system: DESIGN_SYSTEM,
        }
    }
}
impl Default for FluentMenu {
    fn default() -> Self {
        Self::new()
    }
}
