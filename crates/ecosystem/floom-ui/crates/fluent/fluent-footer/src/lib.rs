//! Fluent footer, ported to the Floom 2.0 widget API.

use floom_core::{LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Insets, Shadow};
use floom_widgets::{Align, Edges, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, FooterTokens};
use floom_core_theme::props::footer::FooterProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;

const BLACK: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

const ELEVATION_BLUR: f32 = 6.0;

const ELEVATION_ALPHA: u8 = 30;

pub type ChildBuilder = std::sync::Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

fn footer_tokens(theme: &MultiTheme) -> FooterTokens {
    match theme.component(ComponentId::Footer) {
        ComponentTokens::Footer(t) => t,
        _ => unreachable!("ComponentId::Footer must resolve to FooterTokens"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentFooterStyle {
    pub surface: Color,
    pub content: Color,
    pub outline: Color,
    pub height: f32,
    pub padding: f32,
    pub border: f32,
    pub bordered: bool,
    pub elevated: bool,
}

pub struct FluentFooter {
    props: FooterProps,
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
}

impl FluentFooter {
    pub fn new() -> Self {
        Self {
            props: FooterProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            child: None,
        }
    }

    pub fn height(mut self, height: f32) -> Self {
        self.props.height = Some(height);
        self
    }

    pub fn pad(mut self, pad: f32) -> Self {
        self.props.pad = Some(pad);
        self
    }

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.props.bordered = bordered;
        self
    }

    pub fn elevation(mut self, elevation: u8) -> Self {
        self.props.elevation = elevation;
        self
    }

    pub fn child(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.child = Some(std::sync::Arc::new(build));
        self
    }

    pub fn props(&self) -> &FooterProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentFooterStyle {
        let theme = self.theme.get();
        let tokens = footer_tokens(&theme);
        FluentFooterStyle {
            surface: theme.colors().surface,
            content: theme.colors().on_surface,
            outline: theme.colors().outline_variant,
            height: self.props.height.unwrap_or(tokens.height),
            padding: self.props.pad.unwrap_or(tokens.pad),
            border: tokens.border,
            bordered: self.props.bordered,
            elevated: self.props.elevation > 0,
        }
    }

    pub fn request(&self) -> LayoutRequest {
        let style = self.resolved_style();
        LayoutRequest::new()
            .height(style.height)
            .width_pct(100.0)
            .main_align(Align::Center)
            .cross_align(Align::Center)
    }

    pub fn padding(&self) -> Insets {
        Insets::uniform(f64::from(self.resolved_style().padding))
    }

    pub fn border(&self) -> Option<(f32, PaintColor)> {
        let style = self.resolved_style();
        style
            .bordered
            .then_some((style.border, to_paint(style.outline)))
    }

    pub fn border_edges(&self) -> Edges {
        if self.resolved_style().bordered {
            Edges::TOP
        } else {
            Edges::NONE
        }
    }

    pub fn shadow(&self) -> Option<Shadow> {
        self.resolved_style().elevated.then(|| Shadow {
            color: to_paint(with_alpha(BLACK, ELEVATION_ALPHA)),
            blur: f64::from(ELEVATION_BLUR),
            offset_x: 0.0,
            offset_y: 0.0,
            spread: 0.0,
        })
    }

    pub fn content_color(&self) -> PaintColor {
        to_paint(self.resolved_style().content)
    }

    pub fn publish_layout(&self) {
        floom_core_theme::layout::register_layout(
            floom_core_theme::layout::LayoutSlot::Footer,
            self.resolved_style().height,
        );
    }

    pub fn view(self) -> Surface {
        self.publish_layout();

        let style = self.resolved_style();
        let request = self.request();
        let padding = self.padding();
        let border = self.border();
        let edges = self.border_edges();
        let shadow = self.shadow();
        let surface_colour = to_paint(style.surface);
        let child = self.child;

        let mut surface = Surface::new()
            .request(request)
            .padding(padding)
            .background(surface_colour);
        if let Some((width, colour)) = border {
            surface = surface.border_edges(width, colour, edges);
        }
        if let Some(shadow) = shadow {
            surface = surface.shadow(shadow);
        }
        match child {
            Some(child) => surface.push(move || child()),
            None => surface,
        }
    }
}

impl Default for FluentFooter {
    fn default() -> Self {
        Self::new()
    }
}
