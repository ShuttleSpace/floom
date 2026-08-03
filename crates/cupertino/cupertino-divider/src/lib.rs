use floem::reactive::{RwSignal, SignalGet};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::divider::{DividerOrientation, DividerProps};
use floem_core_theme::theme::{CupertinoTheme, Theme};
use floem_core_theme::tokens::color::Color;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoDividerStyle {
    pub design_system: &'static str,
    pub color: Color,
    pub thickness: f32,
    pub inset: f32,
    pub orientation: DividerOrientation,
}

pub struct CupertinoDivider {
    props: DividerProps,
    theme: RwSignal<CupertinoTheme>,
}

impl CupertinoDivider {
    pub fn new() -> Self {
        Self {
            props: DividerProps::default(),
            theme: RwSignal::new(CupertinoTheme::new()),
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

    pub fn theme(&self) -> RwSignal<CupertinoTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: CupertinoTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoDividerStyle {
        let theme = self.theme.get();
        let ComponentTokens::Divider(tokens) = theme.component(ComponentId::Divider) else {
            unreachable!("ComponentId::Divider must resolve to DividerTokens")
        };
        CupertinoDividerStyle {
            design_system: DESIGN_SYSTEM,
            color: theme.colors().outline_variant,
            thickness: tokens.thickness,
            inset: self.props.inset,
            orientation: self.props.orientation,
        }
    }
}

impl Default for CupertinoDivider {
    fn default() -> Self {
        Self::new()
    }
}
