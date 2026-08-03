use floem::reactive::{RwSignal, SignalGet};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::menu::{MenuItem, MenuProps};
use floem_core_theme::theme::Theme;
use floem_core_theme::tokens::color::Color;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdMenuStyle {
    pub container_color: Color,
    pub text_color: Color,
    pub min_width: f32,
    pub item_height: f32,
    pub padding: f32,
}

pub struct MdMenu {
    props: MenuProps,
    theme: RwSignal<floem_core_theme::theme::MaterialTheme>,
}

impl MdMenu {
    pub fn new() -> Self {
        Self {
            props: MenuProps::default(),
            theme: theme_signal(),
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

    pub fn select(&self, index: usize) {
        if let Some(item) = self.props.items.get(index) {
            if let Some(callback) = &item.on_select {
                callback();
            }
        }
    }

    pub fn props(&self) -> &MenuProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<floem_core_theme::theme::MaterialTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: floem_core_theme::theme::MaterialTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdMenuStyle {
        let theme = self.theme.get();
        let ComponentTokens::Menu(tokens) = theme.component(ComponentId::Menu) else {
            unreachable!("ComponentId::Menu must resolve to MenuTokens")
        };
        MdMenuStyle {
            container_color: theme.colors().surface,
            text_color: theme.colors().on_surface,
            min_width: tokens.min_width,
            item_height: tokens.item_height,
            padding: tokens.padding,
        }
    }
}

impl Default for MdMenu {
    fn default() -> Self {
        Self::new()
    }
}
