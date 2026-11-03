//! Cupertino tooltip, ported to the Floom 2.0 widget API.
//!

use floom_core::Child;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::{Tooltip, TooltipStyle};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::tooltip::TooltipProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

const DESIGN_SYSTEM: &str = "cupertino";

/// The tooltip's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoTooltipStyle {
    pub design_system: &'static str,
    /// The bubble's fill.
    pub bubble_color: Color,
    /// The text's ink.
    pub text_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub font_size: f32,
    /// The gap between the control and the bubble. A v1 literal, and this kit's
    /// is zero.
    pub gap: f32,
}

/// Cupertino's tooltip.
pub struct CupertinoTooltip {
    props: TooltipProps,
    child: Option<Child>,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoTooltip {
    pub fn new() -> Self {
        Self {
            props: TooltipProps::default(),
            child: None,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
        }
    }

    /// The control the pointer rests on.
    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    /// What the tooltip says.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.props.text = text.into();
        self
    }

    pub fn props(&self) -> &TooltipProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoTooltipStyle {
        let theme = self.theme.get();
        let ComponentTokens::Tooltip(tokens) = theme.component(ComponentId::Tooltip) else {
            unreachable!("ComponentId::Tooltip must resolve to TooltipTokens")
        };
        CupertinoTooltipStyle {
            design_system: DESIGN_SYSTEM,
            // v1's paint: `with_alpha(t.colors().on_surface, 230)`.
            bubble_color: with_alpha(theme.colors().on_surface, 230),
            text_color: theme.colors().surface,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            font_size: tokens.font_size,
            // v1's Cupertino bubble had no offset.
            gap: 0.0,
        }
    }

    /// The tooltip: the control, the text, and the bubble that follows them.
    pub fn view(self) -> Tooltip {
        let style = self.resolved_style();
        let mut tooltip = Tooltip::new()
            .text(self.props.text.clone())
            .style(TooltipStyle {
                background: to_paint(style.bubble_color),
                foreground: to_paint(style.text_color),
                padding: f64::from(style.padding),
                radius: style.border_radius,
                font_size: style.font_size,
                gap: f64::from(style.gap),
            });
        if let Some(child) = self.child {
            tooltip = tooltip.trigger(child);
        }
        tooltip
    }
}

impl Default for CupertinoTooltip {
    fn default() -> Self {
        Self::new()
    }
}
