//! fluent divider, ported to the Floom 2.0 widget API.

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{Divider, RuleAlignment};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, DividerTokens};
use floom_core_theme::props::divider::{DividerOrientation, DividerProps};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

const DESIGN_SYSTEM: &str = "fluent";

fn divider_tokens(theme: &MultiTheme) -> DividerTokens {
    match theme.component(ComponentId::Divider) {
        ComponentTokens::Divider(t) => t,
        _ => unreachable!("ComponentId::Divider must resolve to DividerTokens"),
    }
}

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
    theme: RwSignal<MultiTheme>,
}

impl FluentDivider {
    pub fn new() -> Self {
        Self {
            props: DividerProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
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

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentDividerStyle {
        let theme = self.theme.get();
        let tokens = divider_tokens(&theme);
        FluentDividerStyle {
            design_system: DESIGN_SYSTEM,
            color: theme.colors().outline_variant,
            thickness: tokens.thickness,
            inset: self.props.inset,
            orientation: self.props.orientation,
        }
    }

    pub fn divider(&self) -> Divider {
        let style = self.resolved_style();
        Divider::new()
            .color(to_paint(style.color))
            .thickness(style.thickness as f64)
            .axis(match style.orientation {
                DividerOrientation::Horizontal => floom_core::Axis::Horizontal,
                DividerOrientation::Vertical => floom_core::Axis::Vertical,
            })
            .indent(style.inset as f64)
            .alignment(RuleAlignment::Start)
    }
}

impl Default for FluentDivider {
    fn default() -> Self {
        Self::new()
    }
}
