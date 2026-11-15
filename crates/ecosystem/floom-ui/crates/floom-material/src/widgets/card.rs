use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Insets, Shadow};
use floom_widgets::Surface;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::card::CardProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_core_theme::tokens::shadow::ShadowData;
use floom_widget_runtime::theme_signal;

/// Opaque black, the base every elevation shadow tints.
const BLACK: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdCardStyle {
    pub container_color: Color,
    pub content_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub shadow: ShadowData,
}

pub struct MdCard {
    props: CardProps,
    theme: RwSignal<MultiTheme>,
}

impl MdCard {
    pub fn new() -> Self {
        Self {
            props: CardProps::default(),
            theme: theme_signal(),
        }
    }

    pub fn elevation(mut self, elevation: u8) -> Self {
        self.props.elevation = elevation.min(5);
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

    pub fn resolved_style(&self) -> MdCardStyle {
        let theme = self.theme.get();
        let ComponentTokens::Card(tokens) = theme.component(ComponentId::Card) else {
            unreachable!("ComponentId::Card must resolve to CardTokens")
        };
        let level = usize::from(self.props.elevation.min(5));
        MdCardStyle {
            container_color: theme.colors().surface,
            content_color: theme.colors().on_surface,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            shadow: theme.shadow().levels[level],
        }
    }

    pub fn shadow(&self) -> Shadow {
        shadow_from(self.resolved_style().shadow)
    }

    pub fn surface(&self) -> Surface {
        let style = self.resolved_style();
        Surface::new()
            .background(to_paint(style.container_color))
            .radius(style.border_radius)
            .padding(self.padding())
            .shadow(shadow_from(style.shadow))
    }

    pub fn empty_surface(&self) -> Surface {
        self.surface()
    }

    pub fn padding(&self) -> Insets {
        Insets::uniform(f64::from(self.resolved_style().padding))
    }
}

impl Default for MdCard {
    fn default() -> Self {
        Self::new()
    }
}

fn shadow_from(data: ShadowData) -> Shadow {
    Shadow {
        color: to_paint(with_alpha(BLACK, data.color_a)),
        blur: f64::from(data.blur),
        offset_x: f64::from(data.offset_x),
        offset_y: f64::from(data.offset_y),
        spread: f64::from(data.spread),
    }
}
