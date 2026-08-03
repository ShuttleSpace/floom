use floem_core_theme::props::dialog::DialogProps;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentDialogStyle {
    pub design_system: &'static str,
}

pub struct FluentDialog {
    props: DialogProps,
}

impl FluentDialog {
    pub fn new() -> Self {
        Self {
            props: DialogProps::default(),
        }
    }
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = title.into();
        self
    }
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.props.content = content.into();
        self
    }
    pub fn open(mut self, open: bool) -> Self {
        self.props.open = open;
        self
    }
    pub fn on_close(mut self, callback: impl Fn() + 'static) -> Self {
        self.props.on_close = Some(Box::new(callback));
        self
    }
    pub fn props(&self) -> &DialogProps {
        &self.props
    }
    pub fn resolved_style(&self) -> FluentDialogStyle {
        FluentDialogStyle {
            design_system: DESIGN_SYSTEM,
        }
    }
}
impl Default for FluentDialog {
    fn default() -> Self {
        Self::new()
    }
}
