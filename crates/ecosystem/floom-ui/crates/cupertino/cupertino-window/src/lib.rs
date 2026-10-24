//! Cupertino window, ported to the Floom 2.0 widget API.
//!
//! ## What v1's window was
//!
//! v1's `window` was **not** an application shell. It was a *viewer*: a box
//! with a border, a set of pages inside it, arrows at the edges to step
//! through them, and dots along the bottom to show which page is on show. The
//! application shell in v1 was `md-app`; the OS window itself is the host's
//! business in 2.0, and this kit is what goes *inside* one.
//!
//! So the port carries the viewer: the box, the pages, the arrows and the
//! dots. What it cannot carry is the window around it -- there is no v1
//! window to carry, and 2.0's host owns the real one.
//!
//! ## What the port uses
//!
//! The engine's vocabulary, plus the engine's `WindowViewer`: the thing v1's
//! `Stack::new(children)` was -- a box that holds the page on show and the
//! arrows and the dots over it, and that re-declares itself when the page
//! changes.
//!
//! The viewer is a shared shape: all three kits were the same widget with
//! different tokens, so it belongs in `floom-ui-composites` and it is there
//! now. The engine's `Tabs` is the nearest thing to a viewer it had, and it
//! is not one: a tab bar draws a row of tabs, not a box with a page in it.
//!
//! ## What this kit answers
//!
//! * the box's height, its corner radius, the dots' size and the arrows' size:
//!   the tokens;
//! * the box's fill: `surface`; the arrow glyphs' ink: `on_surface`; the
//!   active dot's fill: `primary`;
//! * the inactive dots' fill: `on_surface` at 102/255 (v1 wrote 0.4 of 255);
//! * the outline: 1px of `outline` at 80/255;
//! * the gap between the dots (6) and the width of the arrow's box (40);
//! * the arrow glyph's size: 0.6 of the token's arrow size;
//! * how far the dots sit above the bottom edge: 2% of the box's height.
//!
//! ## The page that is not on show is not mounted
//!
//! v1 mounted every page and hid the ones that were not on show with
//! `hide()`, so a hidden page kept whatever state it had. 2.0 has no
//! `hide()`: [`LayoutRequest`] carries no visibility. The port mounts the page
//! on show and unmounts the rest, so a page's state is lost when the viewer
//! moves away from it. The engine's own `Tabs` composite makes the same
//! choice.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The pointer cursor.** v1 asked for `CursorStyle::Pointer` on the arrows
//!   and the dots. 2.0's `Pressable` does not answer `Widget::cursor`, so the
//!   pointer is not carried. The arrows and the dots are still clickable, and
//!   by keyboard too, which the pointer was not.
//! * **The callback's shape.** v1 kept the change callback in the kit rather
//!   than in the props; the port keeps it there and makes it `Send + Sync`,
//!   because 2.0's slots are shared between threads.
//! * **The accessors.** v1's window had no `props()`, no `theme()` and no
//!   `with_theme()`. The port gives it all three, because a kit that cannot be
//!   given a theme cannot be tested against one.
//!
//! ## The theme source
//!
//! v1's Cupertino window took a **snapshot** of its own design system's theme
//! rather than reading the app-wide signal, so an app-wide theme change
//! does not re-theme it. The port keeps that difference.
//!
//! ## The height
//!
//! v1's window kept its own copy of the height and left `WindowProps::height`
//! dead. The port writes the props' field, so `props().height` reports what
//! the caller asked for, and falls back to the token's height when they asked
//! for nothing.

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

/// The gap between the dots. A v1 literal.
const DOT_GAP: f32 = 6.0;
/// The inactive dots' alpha: v1 wrote 0.4 of 255.
const DOT_INACTIVE_ALPHA: u8 = 102;
/// The width of the box the arrow glyph sits in. A v1 literal.
const ARROW_WIDTH: f32 = 40.0;
/// The arrow glyph's size, as a fraction of the token's arrow size. A v1
/// literal.
const ARROW_FONT_RATIO: f32 = 0.6;
/// The outline's thickness. A v1 literal.
const BORDER_WIDTH: f32 = 1.0;
/// The outline's alpha. A v1 literal.
const BORDER_ALPHA: u8 = 80;
/// How far the dots sit above the bottom edge, in per cent of the box's
/// height. A v1 literal.
const DOTS_INSET_PCT: f32 = 2.0;

/// The window's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoWindowStyle {
    pub design_system: &'static str,
    /// The box's fill.
    pub surface: Color,
    /// The arrow glyphs' ink.
    pub on_surface: Color,
    /// v1's style reported this, but the paint never used it.
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
pub struct CupertinoWindow {
    props: WindowProps,
    theme: RwSignal<MultiTheme>,
    /// The pages, as 2.0 slots.
    pages: Vec<Child>,
    /// The page that is on show before the window is built.
    selected: usize,
    /// The change callback, in the shape 2.0's slots need.
    ///
    /// `Send + Sync` is required, not incidental: 2.0 rebuilds a widget inside a
    /// `Child` closure the host may hand between threads, so the callback is
    /// shared as an `Arc`. v1's callback was a `Box<dyn Fn(usize)>`, which
    /// cannot cross that boundary.
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
}

impl CupertinoWindow {
    pub fn new() -> Self {
        Self {
            props: WindowProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
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

    pub fn resolved_style(&self) -> CupertinoWindowStyle {
        let theme = self.theme.get();
        let ComponentTokens::Window(tokens) = theme.component(ComponentId::Window) else {
            unreachable!("ComponentId::Window must resolve to WindowTokens")
        };
        CupertinoWindowStyle {
            design_system: "cupertino",
            surface: theme.colors().surface,
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            primary: theme.colors().primary,
            // v1's inactive dot was `on_surface` at 0.4 of 255.
            dot_inactive_color: with_alpha(theme.colors().on_surface, DOT_INACTIVE_ALPHA),
            border_radius: tokens.border_radius,
            height: if self.props.height > 0.0 {
                self.props.height
            } else {
                tokens.height
            },
            dot_size: tokens.dot_size,
            arrow_size: tokens.arrow_size,
            // v1 drew the glyph at 0.6 of the token's arrow size.
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

impl Default for CupertinoWindow {
    fn default() -> Self {
        Self::new()
    }
}
