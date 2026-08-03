use floem::reactive::{RwSignal, SignalGet};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::divider::{DividerOrientation, DividerProps};
use floem_core_theme::theme::{FluentTheme, Theme};
use floem_core_theme::tokens::color::Color;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentDividerStyle {
    pub design_system: &'static str,
    pub color: Color,
    pub thickness: f32,
    pub inset: f32,
    pub orientation: DividerOrientation,
}

pub struct FluentDivider {
    props: DividerProps,
    theme: RwSignal<FluentTheme>,
}

impl FluentDivider {
    pub fn new() -> Self {
        Self {
            props: DividerProps::default(),
            theme: RwSignal::new(FluentTheme::new()),
        }
    }

    pub fn orientation(mut self, orientation: DividerOrientation) -> Self {
        self.props.orientation = orientation;
        self
    }

    pub fn inset(mut self, inset: f32) -> Self {
        self.props.inset = inset;
        self
    }

    pub fn props(&self) -> &DividerProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<FluentTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: FluentTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentDividerStyle {
        let theme = self.theme.get();
        let ComponentTokens::Divider(tokens) = theme.component(ComponentId::Divider) else {
            unreachable!("ComponentId::Divider must resolve to DividerTokens")
        };
        FluentDividerStyle {
            design_system: DESIGN_SYSTEM,
            color: theme.colors().outline_variant,
            thickness: tokens.thickness,
            inset: self.props.inset,
            orientation: self.props.orientation,
        }
    }
}

impl Default for FluentDivider {
    fn default() -> Self {
        Self::new()
    }
}
