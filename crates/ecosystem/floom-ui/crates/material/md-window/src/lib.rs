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
use floom_widget_runtime::theme_signal;

const DOT_GAP: f32 = 6.0;
const DOT_INACTIVE_ALPHA: u8 = 102;
const ARROW_WIDTH: f32 = 40.0;
const ARROW_FONT_RATIO: f32 = 0.6;
const BORDER_WIDTH: f32 = 1.0;
const BORDER_ALPHA: u8 = 80;
/// How far the dots sit above the bottom edge, in per cent of the box's
/// height. A v1 literal.
const DOTS_INSET_PCT: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialWindowStyle {
    pub design_system: &'static str,
    /// The box's fill.
    pub surface: Color,
    /// The arrow glyphs' ink.
    pub on_surface: Color,
    pub on_surface_variant: Color,
    /// The active dot's fill.
    pub primary: Color,
    /// The inactive dots' fill.
    pub dot_inactive_color: Color,
    /// The box's corner radius.
    pub border_radius: f32,
    /// The box's height.
    pub height: f32,
    /// The dots' diameter.
    pub dot_size: f32,
    /// The token's arrow size.
    pub arrow_size: f32,
    /// The arrow glyph's size: 0.6 of the token's arrow size.
    pub arrow_font_size: f32,
    /// The width of the box the arrow glyph sits in.
    pub arrow_width: f32,
    /// The gap between the dots.
    pub dot_gap: f32,
    /// The outline's thickness.
    pub border_width: f32,
    /// The outline's colour.
    pub border_color: Color,
    /// How far the dots sit above the bottom edge, in per cent of the height.
    pub dots_inset_pct: f32,
}

/// Material's window: a viewer of pages.
pub struct MaterialWindow {
    props: WindowProps,
    theme: RwSignal<MultiTheme>,
    /// The pages, as 2.0 slots.
    pages: Vec<Child>,
    /// The page that is on show before the window is built.
    selected: usize,
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
}

impl MaterialWindow {
    pub fn new() -> Self {
        Self {
            props: WindowProps::default(),
            theme: theme_signal(),
            pages: Vec::new(),
            selected: 0,
            on_change: None,
        }
    }

    /// The pages, in display order.
    pub fn items(mut self, items: Vec<Child>) -> Self {
        self.pages = items;
        self
    }

    /// The box's height. The token's height when nothing is asked for.
    pub fn height(mut self, height: f32) -> Self {
        self.props.height = height;
        self
    }

    /// The page that is on show before the window is built.
    pub fn selected(mut self, selected: usize) -> Self {
        self.selected = selected;
        self
    }

    /// Whether the arrows at the edges of the box are drawn.
    pub fn show_arrows(mut self, show_arrows: bool) -> Self {
        self.props.show_arrows = show_arrows;
        self
    }

    /// Whether the dots along the bottom of the box are drawn.
    pub fn show_dots(mut self, show_dots: bool) -> Self {
        self.props.show_dots = show_dots;
        self
    }

    /// Whether the arrows wrap around at the ends.
    pub fn continuous(mut self, continuous: bool) -> Self {
        self.props.continuous = continuous;
        self
    }

    /// Called with the index of the page that is now on show.
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

    pub fn resolved_style(&self) -> MaterialWindowStyle {
        let theme = self.theme.get();
        let ComponentTokens::Window(tokens) = theme.component(ComponentId::Window) else {
            unreachable!("ComponentId::Window must resolve to WindowTokens")
        };
        MaterialWindowStyle {
            design_system: "material",
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

    /// The viewer: the box, and the page on show inside it.
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

impl Default for MaterialWindow {
    fn default() -> Self {
        Self::new()
    }
}
