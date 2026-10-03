use floom::event::EventPropagation;
use floom::view::AnyView;
use floom::IntoView;
use floom::views::{Decorators, Empty, Label, Stack};
use floom::{
    prelude::*,
    reactive::RwSignal,
};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::tooltip::TooltipProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoTooltipStyle {
    pub design_system: &'static str,
    pub bubble_color: Color,
    pub text_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub font_size: f32,
}

pub struct CupertinoTooltip {
    props: TooltipProps,
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
    hover: RwSignal<bool>,
}

impl CupertinoTooltip {
    pub fn new() -> Self {
        Self {
            props: TooltipProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
            child: None,
            hover: RwSignal::new(false),
        }
    }
    /// Set the view that triggers the tooltip on hover.
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
        self
    }
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.props.text = text.into();
        self
    }
    pub fn props(&self) -> &TooltipProps {
        &self.props
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
            bubble_color: theme.colors().on_surface,
            text_color: theme.colors().surface,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            font_size: tokens.font_size,
        }
    }

    /// Build a renderable Cupertino tooltip wrapper.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let hover = self.hover;
        let text_str = self.props.text.clone();
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());

        let tokens = match theme.get().component(ComponentId::Tooltip) {
            ComponentTokens::Tooltip(t) => t,
            _ => unreachable!(),
        };
        let padding = tokens.padding;
        let border_radius = tokens.border_radius;
        let font_size = tokens.font_size;

        let bubble = Container::new(Label::new(text_str))
            .style(move |s| {
                let t = theme.get();
                s.padding(padding)
                    .border_radius(border_radius)
                    .font_size(font_size)
                    .background(with_alpha(t.colors().on_surface, 230).to_peniko())
                    .color(t.colors().surface.to_peniko())
                    .apply_if(!hover.get(), |s| s.hide())
            });

        Container::new(Stack::vertical((child, Overlay::new(bubble))))
            .style(|s| s.items_center())
            .on_event(listener::PointerEnter, {
                let hover = hover;
                move |_, _| {
                    hover.set(true);
                    EventPropagation::Continue
                }
            })
            .on_event(listener::PointerLeave, {
                let hover = hover;
                move |_, _| {
                    hover.set(false);
                    EventPropagation::Continue
                }
            })
    }
}

impl Default for CupertinoTooltip {
    fn default() -> Self {
        Self::new()
    }
}
