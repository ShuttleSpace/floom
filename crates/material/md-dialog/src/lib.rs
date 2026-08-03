use floem::reactive::{RwSignal, SignalGet, SignalUpdate};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::dialog::DialogProps;
use floem_core_theme::theme::Theme;
use floem_core_theme::tokens::color::Color;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdDialogStyle {
    pub scrim_color: Color,
    pub container_color: Color,
    pub content_color: Color,
    pub max_width: f32,
    pub padding: f32,
    pub border_radius: f32,
}

pub struct MdDialog {
    props: DialogProps,
    open: RwSignal<bool>,
    theme: RwSignal<floem_core_theme::theme::MaterialTheme>,
}

impl MdDialog {
    pub fn new() -> Self {
        let props = DialogProps::default();
        Self {
            open: RwSignal::new(props.open),
            theme: theme_signal(),
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

    pub fn props(&self) -> &DialogProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<floem_core_theme::theme::MaterialTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: floem_core_theme::theme::MaterialTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdDialogStyle {
        let theme = self.theme.get();
        let ComponentTokens::Dialog(tokens) = theme.component(ComponentId::Dialog) else {
            unreachable!("ComponentId::Dialog must resolve to DialogTokens")
        };
        MdDialogStyle {
            scrim_color: theme.colors().scrim,
            container_color: theme.colors().surface,
            content_color: theme.colors().on_surface,
            max_width: tokens.max_width,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
        }
    }
}

impl Default for MdDialog {
    fn default() -> Self {
        Self::new()
    }
}
