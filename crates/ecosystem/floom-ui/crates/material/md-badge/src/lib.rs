use floom::prelude::*;
use floom::view::AnyView;
use floom::IntoView;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::badge::BadgeProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdBadgeStyle {
    pub design_system: &'static str,
    pub bubble_color: Color,
    pub text_color: Color,
    pub font_size: f32,
    pub min_size: f32,
    pub padding: f32,
    pub dot_size: f32,
}

pub struct MdBadge {
    props: BadgeProps,
    child: Option<AnyView>,
    theme: RwSignal<MultiTheme>,
}

impl MdBadge {
    pub fn new() -> Self {
        Self {
            props: BadgeProps {
                content: String::new(),
                dot: false,
            },
            child: None,
            theme: theme_signal(),
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

    /// Set the element the badge is overlaid on (top-right corner).
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
        self
    }

    pub fn resolved_style(&self) -> MdBadgeStyle {
        let theme = self.theme.get();
        let ComponentTokens::Badge(tokens) = theme.component(ComponentId::Badge) else {
            unreachable!("ComponentId::Badge must resolve to BadgeTokens")
        };
        MdBadgeStyle {
            design_system: "material",
            bubble_color: theme.colors().error,
            text_color: theme.colors().on_error,
            font_size: tokens.font_size,
            min_size: tokens.min_size,
            padding: tokens.padding,
            dot_size: tokens.dot_size,
        }
    }

    /// Build a renderable Material 3 badge: a child with a bubble at top-right.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let content = self.props.content.clone();
        let dot = self.props.dot;
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());

        let tokens = match theme.get().component(ComponentId::Badge) {
            ComponentTokens::Badge(t) => t,
            _ => unreachable!(),
        };
        let font_size = tokens.font_size;
        let min_size = tokens.min_size;
        let padding = tokens.padding;
        let dot_size = tokens.dot_size;

        let bubble: AnyView = if dot {
            Container::new(Empty::new())
                .style(move |s| {
                    let t = theme.get();
                    s.absolute()
                        .inset_top_pct(0.0)
                        .inset_right_pct(0.0)
                        .translate_x(dot_size as f64 / 2.0)
                        .translate_y(-(dot_size as f64 / 2.0))
                        .size(dot_size, dot_size)
                        .border_radius(dot_size / 2.0)
                        .background(t.colors().error.to_peniko())
                })
                .into_any()
        } else if !content.is_empty() {
            Container::new(Label::new(content))
                .style(move |s| {
                    let t = theme.get();
                    s.absolute()
                        .inset_top_pct(0.0)
                        .inset_right_pct(0.0)
                        .translate_x(min_size as f64 / 2.0)
                        .translate_y(-(min_size as f64 / 2.0))
                        .min_width(min_size)
                        .height(min_size)
                        .padding_horiz(padding)
                        .border_radius(min_size / 2.0)
                        .background(t.colors().error.to_peniko())
                        .color(t.colors().on_error.to_peniko())
                        .font_size(font_size)
                        .items_center()
                        .justify_center()
                })
                .into_any()
        } else {
            Empty::new().into_any()
        };

        Stack::new((Container::new(child), bubble))
    }
}

impl Default for MdBadge {
    fn default() -> Self {
        Self::new()
    }
}
