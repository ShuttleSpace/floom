//! Cupertino badge, ported to the Floom 2.0 widget API.
//!
//! v1 wrapped a child with an absolutely-positioned bubble at the top-right
//! corner. 2.0's `Badge` is the bubble itself. So what is left is the token
//! translation: read the palette (`error` / `on_error`) and the `BadgeTokens`,
//! hand them to `Badge`, and return it.
//!
//! See `floom-widget-md-badge`'s module docs for the full rationale of what the
//! port drops (the overlay over a child). The only thing this kit answers
//! differently from Material is nothing in the palette -- both use `error` /
//! `on_error` -- so the two are kept only because the engine widget plus a
//! two-line palette is the whole port and the family is trivially three crates.

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::badge::BadgeProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::Badge;
use floom_widget_runtime::theme_signal;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoBadgeStyle {
    pub design_system: &'static str,
    pub bubble_color: Color,
    pub text_color: Color,
    pub font_size: f32,
    pub min_size: f32,
    pub padding: f32,
    pub dot_size: f32,
}

pub struct CupertinoBadge {
    props: BadgeProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoBadge {
    pub fn new() -> Self {
        Self {
            props: BadgeProps {
                content: String::new(),
                dot: false,
            },
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
        }
    }

    /// Set the text shown inside the badge. Ignored when `dot` is set.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.props.content = content.into();
        self
    }

    /// Render as a plain dot with no text.
    pub fn dot(mut self, dot: bool) -> Self {
        self.props.dot = dot;
        self
    }

    pub fn props(&self) -> &BadgeProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoBadgeStyle {
        let theme = self.theme.get();
        let ComponentTokens::Badge(tokens) = theme.component(ComponentId::Badge) else {
            unreachable!("ComponentId::Badge must resolve to BadgeTokens")
        };
        CupertinoBadgeStyle {
            design_system: DESIGN_SYSTEM,
            bubble_color: theme.colors().error,
            text_color: theme.colors().on_error,
            font_size: tokens.font_size,
            min_size: tokens.min_size,
            padding: tokens.padding,
            dot_size: tokens.dot_size,
        }
    }

    /// Build a renderable Cupertino badge bubble.
    pub fn view(self) -> Badge {
        let style = self.resolved_style();
        Badge::new()
            .content(self.props.content)
            .dot(self.props.dot)
            .background(to_paint(style.bubble_color))
            .foreground(to_paint(style.text_color))
            .font_size(style.font_size)
            .min_size(style.min_size)
            .padding(style.padding)
            .dot_size(style.dot_size)
    }
}

impl Default for CupertinoBadge {
    fn default() -> Self {
        Self::new()
    }
}

