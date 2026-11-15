//! Fluent hover, ported to the Floom 2.0 widget API.

use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::HoverBox;
use floom_widgets::{read_flag, shared_flag, SharedFlag};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::hover::HoverProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

type Callback = Arc<dyn Fn(bool) + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentHoverStyle {
    pub design_system: &'static str,
    pub highlight_color: Color,
    pub highlight_alpha: f32,
    pub transition_ms: f32,
}

pub struct FluentHover {
    props: HoverProps,
    child: Option<ChildBuilder>,
    theme: RwSignal<MultiTheme>,
    hovered: SharedFlag,
    on_hover_change: Option<Callback>,
}

impl FluentHover {
    pub fn new() -> Self {
        Self {
            props: HoverProps::default(),
            child: None,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            hovered: shared_flag(false),
            on_hover_change: None,
        }
    }

    pub fn child(mut self, child: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.child = Some(Arc::new(child));
        self
    }

    pub fn on_hover_change(mut self, callback: impl Fn(bool) + Send + Sync + 'static) -> Self {
        self.on_hover_change = Some(Arc::new(callback));
        self
    }

    pub fn hovered_flag(&self) -> SharedFlag {
        self.hovered.clone()
    }

    pub fn with_hovered_flag(mut self, hovered: SharedFlag) -> Self {
        self.hovered = hovered;
        self
    }

    pub fn props(&self) -> &HoverProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentHoverStyle {
        let theme = self.theme.get();
        let ComponentTokens::Hover(tokens) = theme.component(ComponentId::Hover) else {
            unreachable!("ComponentId::Hover must resolve to HoverTokens")
        };
        FluentHoverStyle {
            design_system: "fluent",
            highlight_color: with_alpha(theme.colors().on_surface, tokens.highlight_alpha as u8),
            highlight_alpha: tokens.highlight_alpha,
            transition_ms: tokens.transition_ms,
        }
    }

    pub fn view(self) -> HoverBox {
        let style = self.resolved_style();
        let child = match self.child {
            Some(child) => Child::new(ChildKey::index(0), LayoutRequest::new(), move || child()),
            None => Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                Box::new(floom_widgets::Text::new(""))
            }),
        };
        let mut hover_box = HoverBox::new()
            .child(child)
            .highlight(to_paint(style.highlight_color))
            .hovered_flag(self.hovered);
        if let Some(callback) = self.on_hover_change {
            hover_box = hover_box.on_hover_change(move |hovered| callback(hovered));
        }
        hover_box
    }
}

impl Default for FluentHover {
    fn default() -> Self {
        Self::new()
    }
}
