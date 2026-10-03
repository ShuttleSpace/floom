use floom::event::EventPropagation;
use floom::prelude::*;
use floom::context::{LayoutChanged, LayoutChangedListener};
use floom::view::AnyView;
use floom::views::{Decorators, Empty, Label, Stack};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::tooltip::TooltipProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_runtime::theme_signal;

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

        let origin = RwSignal::new((0.0f64, 0.0f64));
        let trigger_h = RwSignal::new(0.0f32);

        let bubble = Container::new(Label::new(tip_text))
            .style(move |s| {
                let t = theme.get();
                let (ox, oy) = origin.get();
                let h = trigger_h.get() as f64;
                s.absolute()
                    .inset_top(oy + h)
                    .inset_left(ox)
                    .translate_y(4.0)
                    .padding(padding)
                    .border_radius(border_radius)
                    .font_size(font_size)
                    .background(with_alpha(t.colors().on_surface, 230).to_peniko())
                    .color(t.colors().surface.to_peniko())
                    .apply_if(!hover.get(), |s| s.hide())
            });
        let dropdown = Overlay::new(bubble);

        Container::new(Stack::vertical((trigger, dropdown)))
            .style(|s| s.items_start())
            .on_event(LayoutChangedListener, move |_cx, layout: &LayoutChanged| {
                origin.set((layout.new_window_origin.x, layout.new_window_origin.y));
                trigger_h.set(layout.new_box.size().height as f32);
                EventPropagation::Continue
            })
    }
}

impl Default for MdTooltip {
    fn default() -> Self {
        Self::new()
    }
}
