use floem::reactive::{RwSignal, SignalGet};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::card::CardProps;
use floem_core_theme::theme::{FluentTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::shadow::ShadowData;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentCardStyle {
    pub design_system: &'static str,
    pub container_color: Color,
    pub content_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub shadow: ShadowData,
}

pub struct FluentCard {
    props: CardProps,
    theme: RwSignal<FluentTheme>,
}

impl FluentCard {
    pub fn new() -> Self {
        Self {
            props: CardProps::default(),
            theme: RwSignal::new(FluentTheme::new()),
        }
    }
    pub fn elevation(mut self, elevation: u8) -> Self {
        self.props.elevation = elevation.min(5);
        self
    }
    pub fn props(&self) -> &CardProps {
        &self.props
    }
    pub fn theme(&self) -> RwSignal<FluentTheme> {
        self.theme
    }
    pub fn with_theme(mut self, theme: FluentTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> FluentCardStyle {
        let theme = self.theme.get();
        let ComponentTokens::Card(tokens) = theme.component(ComponentId::Card) else {
            unreachable!("ComponentId::Card must resolve to CardTokens")
        };
        let level = usize::from(self.props.elevation.min(5));
        FluentCardStyle {
            design_system: DESIGN_SYSTEM,
            container_color: theme.colors().surface,
            content_color: theme.colors().on_surface,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            shadow: theme.shadow().levels[level],
        }
    }
}
impl Default for FluentCard {
    fn default() -> Self {
        Self::new()
    }
}
