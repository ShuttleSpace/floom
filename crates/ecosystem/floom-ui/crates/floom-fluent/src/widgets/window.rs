//! Fluent window, ported to the Floom 2.0 widget API.

use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use floom_core::Child;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
pub use floom_ui_composites::WindowViewer;
use floom_ui_composites::WindowViewerStyle;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::window::WindowProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

const DOT_GAP: f32 = 6.0;
const DOT_INACTIVE_ALPHA: u8 = 102;
const ARROW_WIDTH: f32 = 40.0;
const ARROW_FONT_RATIO: f32 = 0.6;
const BORDER_WIDTH: f32 = 1.0;
const BORDER_ALPHA: u8 = 80;
const DOTS_INSET_PCT: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentWindowStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub primary: Color,
    pub dot_inactive_color: Color,
    pub border_radius: f32,
    pub height: f32,
    pub dot_size: f32,
    pub arrow_size: f32,
    pub arrow_font_size: f32,
    pub arrow_width: f32,
    pub dot_gap: f32,
    pub border_width: f32,
    pub border_color: Color,
    pub dots_inset_pct: f32,
}

pub struct FluentWindow {
    props: WindowProps,
    theme: RwSignal<MultiTheme>,
    pages: Vec<Child>,
    selected: usize,
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
}

impl FluentWindow {
    pub fn new() -> Self {
        Self {
            props: WindowProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            pages: Vec::new(),
            selected: 0,
            on_change: None,
        }
    }

    pub fn items(mut self, items: Vec<Child>) -> Self {
        self.pages = items;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.props.height = height;
        self
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }

    pub fn show_arrows(mut self, show_arrows: bool) -> Self {
        self.props.show_arrows = show_arrows;
        self
    }

    pub fn show_dots(mut self, show_dots: bool) -> Self {
        self.props.show_dots = show_dots;
        self
    }

    pub fn continuous(mut self, continuous: bool) -> Self {
        self.props.continuous = continuous;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    pub fn props(&self) -> &WindowProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentWindowStyle {
        let theme = self.theme.get();
        let ComponentTokens::Window(tokens) = theme.component(ComponentId::Window) else {
            unreachable!("ComponentId::Window must resolve to WindowTokens")
        };
        FluentWindowStyle {
            design_system: "fluent",
            surface: theme.colors().surface,
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            primary: theme.colors().primary,
            dot_inactive_color: with_alpha(theme.colors().on_surface, DOT_INACTIVE_ALPHA),
            border_radius: tokens.border_radius,
            height: if self.props.height > 0.0 {
                self.props.height
            } else {
                tokens.height
            },
            dot_size: tokens.dot_size,
            arrow_size: tokens.arrow_size,
            arrow_font_size: tokens.arrow_size * ARROW_FONT_RATIO,
            arrow_width: ARROW_WIDTH,
            dot_gap: DOT_GAP,
            border_width: BORDER_WIDTH,
            border_color: with_alpha(theme.colors().outline, BORDER_ALPHA),
            dots_inset_pct: DOTS_INSET_PCT,
        }
    }

    pub fn view(self) -> WindowViewer {
        let style = self.resolved_style();
        let index = Arc::new(AtomicUsize::new(
            self.selected.min(self.pages.len().saturating_sub(1)),
        ));
        let mut viewer = WindowViewer::new()
            .style(WindowViewerStyle {
                surface: to_paint(style.surface),
                on_surface: to_paint(style.on_surface),
                on_surface_variant: to_paint(style.on_surface_variant),
                primary: to_paint(style.primary),
                dot_inactive_color: to_paint(style.dot_inactive_color),
                border_radius: style.border_radius,
                height: style.height,
                dot_size: style.dot_size,
                arrow_size: style.arrow_size,
                arrow_font_size: style.arrow_font_size,
                arrow_width: style.arrow_width,
                dot_gap: style.dot_gap,
                border_width: style.border_width,
                border_color: to_paint(style.border_color),
                dots_inset_pct: style.dots_inset_pct,
            })
            .pages(self.pages)
            .shared_index(index)
            .show_arrows(self.props.show_arrows)
            .show_dots(self.props.show_dots)
            .continuous(self.props.continuous);
        if let Some(on_change) = self.on_change {
            viewer = viewer.on_change(move |index| on_change(index));
        }
        viewer
    }
}

impl Default for FluentWindow {
    fn default() -> Self {
        Self::new()
    }
}
