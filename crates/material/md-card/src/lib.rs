use floem::reactive::{RwSignal, SignalGet};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::card::CardProps;
use floem_core_theme::theme::Theme;
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::shadow::ShadowData;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdCardStyle {
    pub container_color: Color,
    pub content_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub shadow: ShadowData,
}

pub struct MdCard {
    props: CardProps,
    theme: RwSignal<floem_core_theme::theme::MaterialTheme>,
}

impl MdCard {
    pub fn new() -> Self {
        Self {
            props: CardProps::default(),
            theme: theme_signal(),
        }
    }

    pub fn elevation(mut self, elevation: u8) -> Self {
        self.props.elevation = elevation.min(5);
        self
    }

    pub fn props(&self) -> &CardProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<floem_core_theme::theme::MaterialTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: floem_core_theme::theme::MaterialTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdCardStyle {
        let theme = self.theme.get();
        let ComponentTokens::Card(tokens) = theme.component(ComponentId::Card) else {
            unreachable!("ComponentId::Card must resolve to CardTokens")
        };
        let level = usize::from(self.props.elevation.min(5));
        MdCardStyle {
            container_color: theme.colors().surface,
            content_color: theme.colors().on_surface,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            shadow: theme.shadow().levels[level],
        }
    }
}

impl Default for MdCard {
    fn default() -> Self {
        Self::new()
    }
}
