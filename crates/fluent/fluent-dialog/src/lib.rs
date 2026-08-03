use floem::reactive::{RwSignal, SignalGet, SignalUpdate};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::dialog::DialogProps;
use floem_core_theme::theme::{FluentTheme, Theme};
use floem_core_theme::tokens::color::Color;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentDialogStyle {
    pub design_system: &'static str,
    pub scrim_color: Color,
    pub container_color: Color,
    pub content_color: Color,
    pub max_width: f32,
    pub padding: f32,
    pub border_radius: f32,
}

pub struct FluentDialog {
    props: DialogProps,
    open: RwSignal<bool>,
    theme: RwSignal<FluentTheme>,
}

impl FluentDialog {
    pub fn new() -> Self {
        let props = DialogProps::default();
        Self {
            open: RwSignal::new(props.open),
            theme: RwSignal::new(FluentTheme::new()),
            props,
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
        self.open.set(open);
        self
    }
    pub fn on_close(mut self, callback: impl Fn() + 'static) -> Self {
        self.props.on_close = Some(Box::new(callback));
        self
    }
    pub fn props(&self) -> &DialogProps {
        &self.props
    }
    pub fn close(&mut self) {
        self.props.open = false;
        self.open.set(false);
        if let Some(callback) = &self.props.on_close {
            callback();
        }
    }
    pub fn is_open(&self) -> bool {
        self.open.get()
    }
    pub fn theme(&self) -> RwSignal<FluentTheme> {
        self.theme
    }
    pub fn with_theme(mut self, theme: FluentTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> FluentDialogStyle {
        let theme = self.theme.get();
        let ComponentTokens::Dialog(tokens) = theme.component(ComponentId::Dialog) else {
            unreachable!("ComponentId::Dialog must resolve to DialogTokens")
        };
        FluentDialogStyle {
            design_system: DESIGN_SYSTEM,
            scrim_color: theme.colors().scrim,
            container_color: theme.colors().surface,
            content_color: theme.colors().on_surface,
            max_width: tokens.max_width,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
        }
    }
}
impl Default for FluentDialog {
    fn default() -> Self {
        Self::new()
    }
}
