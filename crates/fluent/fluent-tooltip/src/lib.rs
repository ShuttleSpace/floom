use floem::context::{LayoutChanged, LayoutChangedListener};
use floem::event::EventPropagation;
use floem::prelude::*;
use floem::view::AnyView;
use floem::views::Decorators;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::tooltip::TooltipProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentTooltipStyle {
    pub design_system: &'static str,
    pub bubble_color: Color,
    pub text_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub font_size: f32,
}

pub struct FluentTooltip {
    props: TooltipProps,
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
    hover: RwSignal<bool>,
}

impl FluentTooltip {
    pub fn new() -> Self {
        Self {
            props: TooltipProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
            child: None,
            hover: RwSignal::new(false),
        }
    }
    /// Set the trigger content shown underneath the tooltip.
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
    pub fn resolved_style(&self) -> FluentTooltipStyle {
        let theme = self.theme.get();
        let ComponentTokens::Tooltip(tk) = theme.component(ComponentId::Tooltip) else {
            unreachable!("ComponentId::Tooltip must resolve to TooltipTokens")
        };
        FluentTooltipStyle {
            design_system: DESIGN_SYSTEM,
            bubble_color: with_alpha(theme.colors().on_surface, 230),
            text_color: theme.colors().surface,
            padding: tk.padding,
            border_radius: tk.border_radius,
            font_size: tk.font_size,
        }
    }

    /// Build a renderable Fluent tooltip: a hoverable trigger whose text bubble
    /// appears on top while hovered.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let hover = self.hover;
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());
        let tooltip_text = self.props.text.clone();

        let origin = RwSignal::new((0.0f64, 0.0f64));
        let trigger_h = RwSignal::new(0.0f32);

        let bubble = Container::new(text(tooltip_text)).style(move |s| {
            let t = theme.get();
            let ComponentTokens::Tooltip(tk) = t.component(ComponentId::Tooltip) else {
                unreachable!()
            };
            let (ox, oy) = origin.get();
            let h = trigger_h.get() as f64;
            s.absolute()
                .inset_top(oy + h)
                .inset_left(ox)
                .padding(tk.padding)
                .border_radius(tk.border_radius)
                .font_size(tk.font_size)
                .background(with_alpha(t.colors().on_surface, 230).to_peniko())
                .color(t.colors().surface.to_peniko())
                .apply_if(!hover.get(), |s| s.hide())
        });

        let overlay = Overlay::new(bubble);

        Container::new(v_stack((child, overlay)))
            .style(|s| s.items_center())
            .on_event(LayoutChangedListener, move |_cx, layout: &LayoutChanged| {
                origin.set((layout.new_window_origin.x, layout.new_window_origin.y));
                trigger_h.set(layout.new_box.size().height as f32);
                EventPropagation::Continue
            })
            .on_event(listener::PointerEnter, move |_, _| {
                hover.set(true);
                EventPropagation::Continue
            })
            .on_event(listener::PointerLeave, move |_, _| {
                hover.set(false);
                EventPropagation::Continue
            })
    }
}

impl Default for FluentTooltip {
    fn default() -> Self {
        Self::new()
    }
}
