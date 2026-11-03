//! Cupertino button group, ported to the Floom 2.0 widget API.
//!

use std::sync::Arc;

use floom_core::{LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Insets};
use floom_widgets::{Align, Axis, Stack, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::button_group::ButtonGroupProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

/// A builder for one button in the group, replayed each time it is built.
pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoButtonGroupStyle {
    pub surface_variant: Color,
    pub gap: f32,
    pub border_radius: f32,
    pub joined: bool,
    pub vertical: bool,
}

/// Cupertino's button group.
pub struct CupertinoButtonGroup {
    props: ButtonGroupProps,
    theme: RwSignal<MultiTheme>,
    children: Vec<ChildBuilder>,
}

impl CupertinoButtonGroup {
    pub fn new() -> Self {
        Self {
            props: ButtonGroupProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            children: Vec::new(),
        }
    }

    /// Append a child (typically a button) to the group.
    pub fn child(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.children.push(Arc::new(build));
        self
    }

    /// Render the buttons with no gap and a shared outer rounding (segmented).
    pub fn joined(mut self, joined: bool) -> Self {
        self.props.joined = joined;
        self
    }

    /// Lay the buttons out vertically instead of horizontally.
    pub fn vertical(mut self, vertical: bool) -> Self {
        self.props.vertical = vertical;
        self
    }

    pub fn props(&self) -> &ButtonGroupProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoButtonGroupStyle {
        let theme = self.theme.get();
        let ComponentTokens::ButtonGroup(tokens) = theme.component(ComponentId::ButtonGroup) else {
            unreachable!("ComponentId::ButtonGroup must resolve to ButtonGroupTokens")
        };
        CupertinoButtonGroupStyle {
            surface_variant: theme.colors().surface_variant,
            // v1: `if joined { 0.0 } else { tokens.gap }`.
            gap: if self.props.joined { 0.0 } else { tokens.gap },
            border_radius: tokens.border_radius,
            joined: self.props.joined,
            vertical: self.props.vertical,
        }
    }

    /// The group: a filled surface with the buttons inside it.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let axis = if style.vertical {
            Axis::Vertical
        } else {
            Axis::Horizontal
        };
        let surface_variant = to_paint(style.surface_variant);
        let gap = style.gap;
        let radius = style.border_radius;
        let joined = style.joined;

        let mut inner = Stack::new(axis).gap(gap);
        for child in self.children {
            inner = inner.push(move || child());
        }

        Surface::new()
            // v1's `items_center()`.
            .request(LayoutRequest::new().cross_align(Align::Center))
            .background(surface_variant)
            // v1: `border_radius(if joined { radius } else { 0.0 })`.
            .radius(if joined { radius } else { 0.0 })
            // v1: `padding(if joined { 0.0 } else { 4.0 })`.
            .padding(if joined {
                Insets::ZERO
            } else {
                Insets::uniform(4.0)
            })
            .push(move || Box::new(inner.clone()))
    }
}

impl Default for CupertinoButtonGroup {
    fn default() -> Self {
        Self::new()
    }
}
