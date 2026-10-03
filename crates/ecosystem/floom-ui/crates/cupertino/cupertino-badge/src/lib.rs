use floom::prelude::*;
use floom::reactive::RwSignal;
use floom::view::AnyView;
use floom::IntoView;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::badge::BadgeProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
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
    child: Option<AnyView>,
}

impl CupertinoBadge {
    pub fn new() -> Self {
        Self {
            props: BadgeProps {
                content: String::new(),
                dot: false,
            },
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
            child: None,
        }
    }
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.props.content = content.into();
        self
    }
    pub fn dot(mut self, dot: bool) -> Self {
        self.props.dot = dot;
        self
    }
    /// Set the view that the badge is attached to.
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
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

    /// Build a renderable Cupertino badge wrapping its child with a bubble at top-right.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let dot = self.props.dot;
        let content = self.props.content.clone();
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());

        let tokens = match theme.get().component(ComponentId::Badge) {
            ComponentTokens::Badge(t) => t,
            _ => unreachable!(),
        };
        let font_size = tokens.font_size;
        let min_size = tokens.min_size;
        let padding = tokens.padding;
        let dot_size = tokens.dot_size;

        let is_empty = !dot && content.is_empty();

        let bubble = if dot {
            Container::new(Empty::new())
                .style(move |s| {
                    let t = theme.get();
                    let off = dot_size / 2.0;
                    s.absolute()
                        .inset_top(-off)
                        .inset_right(-off)
                        .size(dot_size, dot_size)
                        .background(t.colors().error.to_peniko())
                        .border_radius(t.radius().full)
                        .apply_if(is_empty, |s| s.hide())
                })
                .into_any()
        } else {
            Container::new(Label::new(content))
                .style(move |s| {
                    let t = theme.get();
                    let off = min_size / 2.0;
                    s.absolute()
                        .inset_top(-off)
                        .inset_right(-off)
                        .min_width(min_size)
                        .min_height(min_size)
                        .padding_horiz(padding)
                        .background(t.colors().error.to_peniko())
                        .color(t.colors().on_error.to_peniko())
                        .font_size(font_size)
                        .border_radius(t.radius().full)
                        .items_center()
                        .justify_center()
                        .apply_if(is_empty, |s| s.hide())
                })
                .into_any()
        };

        Container::new(stack((child, bubble))).style(|s| s.items_start())
    }
}

impl Default for CupertinoBadge {
    fn default() -> Self {
        Self::new()
    }
}
