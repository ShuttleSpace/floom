use floem::event::EventPropagation;
use floem::prelude::*;
use floem::view::AnyView;
use floem::views::Decorators;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::tooltip::TooltipProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdTooltipStyle {
    pub design_system: &'static str,
    pub bubble_color: Color,
    pub text_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub font_size: f32,
}

pub struct MdTooltip {
    props: TooltipProps,
    child: Option<AnyView>,
    theme: RwSignal<MultiTheme>,
}

impl MdTooltip {
    pub fn new() -> Self {
        Self {
            props: TooltipProps::default(),
            child: None,
            theme: theme_signal(),
        }
    }

    /// Set the trigger element shown under the tooltip.
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

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdTooltipStyle {
        let theme = self.theme.get();
        let ComponentTokens::Tooltip(tokens) = theme.component(ComponentId::Tooltip) else {
            unreachable!("ComponentId::Tooltip must resolve to TooltipTokens")
        };
        MdTooltipStyle {
            design_system: "material",
            bubble_color: theme.colors().on_surface,
            text_color: theme.colors().surface,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            font_size: tokens.font_size,
        }
    }

    /// Build a renderable, interactive Material 3 tooltip wrapper.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let hover = RwSignal::new(false);
        let tip_text = self.props.text.clone();

        let tokens = match theme.get().component(ComponentId::Tooltip) {
            ComponentTokens::Tooltip(t) => t,
            _ => unreachable!(),
        };
        let padding = tokens.padding;
        let border_radius = tokens.border_radius;
        let font_size = tokens.font_size;

        let child = self.child.unwrap_or_else(|| Empty::new().into_any());

        let trigger = Container::new(child)
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
            });

        let bubble = Container::new(text(tip_text))
            .style(move |s| {
                let t = theme.get();
                s.absolute()
                    .inset_top_pct(100.0)
                    .translate_y(4.0)
                    .padding(padding)
                    .border_radius(border_radius)
                    .font_size(font_size)
                    .background(with_alpha(t.colors().on_surface, 230).to_peniko())
                    .color(t.colors().surface.to_peniko())
                    .apply_if(!hover.get(), |s| s.hide())
            });

        Stack::new((trigger, bubble))
    }
}

impl Default for MdTooltip {
    fn default() -> Self {
        Self::new()
    }
}
