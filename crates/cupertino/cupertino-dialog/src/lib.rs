use floem_core_theme::props::dialog::DialogProps;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoDialogStyle {
    pub design_system: &'static str,
}

pub struct CupertinoDialog {
    props: DialogProps,
}

impl CupertinoDialog {
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
    pub fn resolved_style(&self) -> CupertinoDialogStyle {
        CupertinoDialogStyle {
            design_system: DESIGN_SYSTEM,
        }
    }
}
impl Default for CupertinoDialog {
    fn default() -> Self {
        Self::new()
    }
}
