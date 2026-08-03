use floem::reactive::{RwSignal, SignalGet};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::menu::{MenuItem, MenuProps};
use floem_core_theme::theme::{FluentTheme, Theme};
use floem_core_theme::tokens::color::Color;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentMenuStyle {
    pub design_system: &'static str,
    pub container_color: Color,
    pub text_color: Color,
    pub min_width: f32,
    pub item_height: f32,
    pub padding: f32,
}

pub struct FluentMenu {
    props: MenuProps,
    theme: RwSignal<FluentTheme>,
}

impl FluentMenu {
    pub fn new() -> Self {
        Self {
            props: MenuProps::default(),
            theme: RwSignal::new(FluentTheme::new()),
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
    pub fn select(&self, index: usize) {
        if let Some(item) = self.props.items.get(index) {
            if let Some(callback) = &item.on_select {
                callback();
            }
        }
    }
    pub fn theme(&self) -> RwSignal<FluentTheme> {
        self.theme
    }
    pub fn with_theme(mut self, theme: FluentTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> FluentMenuStyle {
        let theme = self.theme.get();
        let ComponentTokens::Menu(tokens) = theme.component(ComponentId::Menu) else {
            unreachable!("ComponentId::Menu must resolve to MenuTokens")
        };
        FluentMenuStyle {
            design_system: DESIGN_SYSTEM,
            container_color: theme.colors().surface,
            text_color: theme.colors().on_surface,
            min_width: tokens.min_width,
            item_height: tokens.item_height,
            padding: tokens.padding,
        }
    }
}
impl Default for FluentMenu {
    fn default() -> Self {
        Self::new()
    }
}
