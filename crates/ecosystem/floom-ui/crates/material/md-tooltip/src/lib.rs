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
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialTooltipStyle {
    pub design_system: &'static str,
    /// The bubble's fill.
    pub bubble_color: Color,
    /// The text's ink.
    pub text_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub font_size: f32,
    pub gap: f32,
}

/// Material's tooltip.
pub struct MaterialTooltip {
    props: TooltipProps,
    child: Option<Child>,
    theme: RwSignal<MultiTheme>,
}

impl MaterialTooltip {
    pub fn new() -> Self {
        Self {
            props: TooltipProps::default(),
            child: None,
            theme: theme_signal(),
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

    pub fn resolved_style(&self) -> MaterialTooltipStyle {
        let theme = self.theme.get();
        let ComponentTokens::Tooltip(tokens) = theme.component(ComponentId::Tooltip) else {
            unreachable!("ComponentId::Tooltip must resolve to TooltipTokens")
        };
        MaterialTooltipStyle {
            design_system: "material",
            bubble_color: with_alpha(theme.colors().on_surface, 230),
            text_color: theme.colors().surface,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            font_size: tokens.font_size,
            gap: 4.0,
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

impl Default for MaterialTooltip {
    fn default() -> Self {
        Self::new()
    }
}
