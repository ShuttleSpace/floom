//! Fluent card, ported to the Floom 2.0 widget API.

use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Insets, Shadow};
use floom_widgets::{Ripple, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::card::CardProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_core_theme::tokens::shadow::ShadowData;

pub type ContentBuilder =
    std::sync::Arc<dyn Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static>;

fn shadow_from(data: ShadowData) -> Shadow {
    Shadow {
        color: to_paint(with_alpha(BLACK, data.color_a)),
        blur: f64::from(data.blur),
        offset_x: f64::from(data.offset_x),
        offset_y: f64::from(data.offset_y),
        spread: f64::from(data.spread),
    }
}

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentCardStyle {
    pub design_system: &'static str,
    pub container_color: Color,
    pub content_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub shadow: ShadowData,
}

pub struct FluentCard {
    props: CardProps,
    theme: RwSignal<MultiTheme>,
    child: Option<ContentBuilder>,
}

impl FluentCard {
    pub fn new() -> Self {
        Self {
            props: CardProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            child: None,
        }
    }

    pub fn elevation(mut self, elevation: u8) -> Self {
        self.props.elevation = elevation.min(5);
        self
    }

    pub fn child(
        mut self,
        child: impl Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.child = Some(std::sync::Arc::new(child));
        self
    }

    pub fn props(&self) -> &CardProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentCardStyle {
        let theme = self.theme.get();
        let ComponentTokens::Card(tokens) = theme.component(ComponentId::Card) else {
            unreachable!("ComponentId::Card must resolve to CardTokens")
        };
        let level = usize::from(self.props.elevation.min(5));
        FluentCardStyle {
            design_system: DESIGN_SYSTEM,
            container_color: theme.colors().surface,
            content_color: theme.colors().on_surface,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            shadow: theme.shadow().levels[level],
        }
    }

    pub fn corner_radius(&self) -> f32 {
        self.theme.get().radius().xl
    }

    pub fn padding(&self) -> Insets {
        Insets::uniform(f64::from(self.resolved_style().padding))
    }

    pub fn content_color(&self) -> PaintColor {
        to_paint(self.resolved_style().content_color)
    }

    pub fn shadow(&self) -> Shadow {
        shadow_from(self.resolved_style().shadow)
    }

    pub fn surface(&self) -> Surface {
        let style = self.resolved_style();
        surface_from(
            to_paint(style.container_color),
            self.corner_radius(),
            self.padding(),
            self.shadow(),
        )
    }

    pub fn view(self) -> Ripple {
        let style = self.resolved_style();
        let background = to_paint(style.container_color);
        let radius = self.corner_radius();
        let padding = self.padding();
        let shadow = self.shadow();
        let content = self.child;

        Ripple::new()
            .child(floom_core::LayoutRequest::new(), move || {
                let mut surface = surface_from(background, radius, padding, shadow);
                if let Some(content) = &content {
                    surface = surface.push({
                        let content = content.clone();
                        move || content()
                    });
                }
                Box::new(surface)
            })
            .radius(f64::from(radius))
    }
}

fn surface_from(background: PaintColor, radius: f32, padding: Insets, shadow: Shadow) -> Surface {
    Surface::new()
        .background(background)
        .radius(radius)
        .padding(padding)
        .shadow(shadow)
}

impl Default for FluentCard {
    fn default() -> Self {
        Self::new()
    }
}
