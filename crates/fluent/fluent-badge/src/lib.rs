use floem::context::{LayoutChanged, LayoutChangedListener};
use floem::event::EventPropagation;
use floem::prelude::*;
use floem::view::AnyView;
use floem::IntoView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::badge::BadgeProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentBadgeStyle {
    pub design_system: &'static str,
    pub bubble_color: Color,
    pub text_color: Color,
    pub font_size: f32,
    pub min_size: f32,
    pub padding: f32,
    pub dot_size: f32,
}

pub struct FluentBadge {
    props: BadgeProps,
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
}

impl FluentBadge {
    pub fn new() -> Self {
        Self {
            props: BadgeProps {
                content: String::new(),
                dot: false,
            },
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
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
    /// Set the wrapped content shown beneath the badge bubble.
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
    pub fn resolved_style(&self) -> FluentBadgeStyle {
        let theme = self.theme.get();
        let ComponentTokens::Badge(tokens) = theme.component(ComponentId::Badge) else {
            unreachable!("ComponentId::Badge must resolve to BadgeTokens")
        };
        FluentBadgeStyle {
            design_system: DESIGN_SYSTEM,
            bubble_color: theme.colors().primary,
            text_color: theme.colors().on_primary,
            font_size: tokens.font_size,
            min_size: tokens.min_size,
            padding: tokens.padding,
            dot_size: tokens.dot_size,
        }
    }

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

        let origin = RwSignal::new((0.0f64, 0.0f64));
        let child_size = RwSignal::new((0.0f64, 0.0f64));

        let inner: AnyView = if dot {
            Empty::new().into_any()
        } else {
            text(content).into_any()
        };

        let bubble = Container::new(inner)
        .style(move |s| {
            let t = theme.get();
            let accent = t.colors().primary;
            let s = s.background(accent.to_peniko()).border_radius(t.radius().full);
            if dot {
                s.width(dot_size).height(dot_size)
            } else {
                s.min_width(min_size)
                    .height(min_size)
                    .padding_horiz(padding)
                    .items_center()
                    .justify_center()
                    .color(t.colors().on_primary.to_peniko())
                    .font_size(font_size)
            }
        })
        .style(move |s| {
            let (ox, oy) = origin.get();
            let (cw, _ch) = child_size.get();
            let offset = if dot { dot_size / 2.0 } else { min_size / 2.0 };
            s.absolute()
                .inset_top(oy - offset as f64)
                .inset_left(ox + cw - offset as f64)
        });

        let overlay = Overlay::new(bubble);

        Container::new(v_stack((child, overlay)))
            .style(|s| s.items_start())
            .on_event(LayoutChangedListener, move |_cx, layout: &LayoutChanged| {
                origin.set((layout.new_window_origin.x, layout.new_window_origin.y));
                child_size.set((
                    layout.new_box.size().width as f64,
                    layout.new_box.size().height as f64,
                ));
                EventPropagation::Continue
            })
    }
}

impl Default for FluentBadge {
    fn default() -> Self {
        Self::new()
    }
}
