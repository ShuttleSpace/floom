//! Fluent window, ported to the Floom 2.0 widget API.
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
//! The engine's vocabulary, plus one widget of the kit's own: the **viewer**,
//! which is the thing v1's `Stack::new(children)` was -- a box that holds the
//! page on show and the arrows and the dots over it, and that re-declares
//! itself when the page changes.
//!
//! The viewer is a shared shape: all three kits are the same widget with
//! different tokens, so it belongs in `floom-ui-composites`. The engine has no
//! viewer composite -- its `Tabs` is the nearest thing to one -- so that piece
//! is missing, and it is the gap this port names. Until it exists, each kit
//! carries its own copy.
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
//! v1's Fluent window took a **snapshot** of its own design system's theme
//! rather than reading the app-wide signal, so an app-wide theme change
//! does not re-theme it. The port keeps that difference.
//!
//! ## The height
//!
//! v1's window kept its own copy of the height and left `WindowProps::height`
//! dead. The port writes the props' field, so `props().height` reports what
//! the caller asked for, and falls back to the token's height when they asked
//! for nothing.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{
    Child, ChildKey, CreateContext, ElementId, Inset, LayoutRequest, PaintContext, UpdateContext,
    Widget,
};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{ClipShape, Color as PaintColor};
use floom_widgets::{Align, Axis, Container, Pressable, Revision, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::window::WindowProps;
use floom_core_theme::theme::{MultiTheme, Theme};
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

/// The key the page on show is mounted under. The index is part of it, so a
/// new page is a new slot and the old one is unmounted.
const PAGE_KEY: u64 = 0x5700_0000;
/// The key the previous arrow is mounted under.
const PREVIOUS_ARROW_KEY: u64 = 0x5700_1000;
/// The key the next arrow is mounted under.
const NEXT_ARROW_KEY: u64 = 0x5700_2000;
/// The key the dots are mounted under. The index is part of it, so the dots
/// are rebuilt -- and so the active one changes colour -- when the page does.
const DOTS_KEY: u64 = 0x5700_3000;

/// The window's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentWindowStyle {
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
pub struct FluentWindow {
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
    pub fn view(self) -> FluentWindowViewer {
        let style = self.resolved_style();
        let index = Arc::new(AtomicUsize::new(
            self.selected.min(self.pages.len().saturating_sub(1)),
        ));
        let decoration = Container::new()
            .background(to_paint(style.surface))
            .radius(style.border_radius)
            .border(style.border_width, to_paint(style.border_color));
        FluentWindowViewer {
            decoration,
            height: style.height,
            pages: self.pages,
            index,
            // Impossible, so the first frame always reconciles.
            declared: usize::MAX,
            show_arrows: self.props.show_arrows,
            show_dots: self.props.show_dots,
            continuous: self.props.continuous,
            on_change: self.on_change,
            style,
            revision: Revision::default(),
        }
    }
}

impl Default for FluentWindow {
    fn default() -> Self {
        Self::new()
    }
}

/// The viewer: the box, the page on show, the arrows and the dots.
///
/// The shape v1's `Stack::new(children)` had, in a widget that can say which
/// page is on show: 2.0 mounts a tree once, so a widget whose children
/// depend on a value that changes has to say so, and re-declare itself when
/// it does.
pub struct FluentWindowViewer {
    /// The box: its fill, its radius and its outline.
    decoration: Container,
    /// The box's height.
    height: f32,
    /// The pages, as 2.0 slots.
    pages: Vec<Child>,
    /// The page on show. Shared with the arrows and the dots, which move it.
    index: Arc<AtomicUsize>,
    /// The page the children were last declared for.
    declared: usize,
    show_arrows: bool,
    show_dots: bool,
    continuous: bool,
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
    style: FluentWindowStyle,
    revision: Revision,
}

impl FluentWindowViewer {
    /// The page on show.
    pub fn index(&self) -> usize {
        self.current()
    }

    /// The page on show, clamped to the pages there are.
    fn current(&self) -> usize {
        let count = self.pages.len();
        if count == 0 {
            return 0;
        }
        self.index.load(Ordering::SeqCst).min(count - 1)
    }

    /// The arrow at one edge of the box: a glyph, and the click that moves the
    /// page on.
    fn arrow(&self, next: bool) -> Child {
        let style = self.style;
        let shared = self.index.clone();
        let count = self.pages.len();
        let continuous = self.continuous;
        let on_change = self.on_change.clone();
        let glyph = if next { "›" } else { "‹" };
        let ink = to_paint(style.on_surface);
        let font_size = style.arrow_font_size;

        let mut request = LayoutRequest::new()
            .width(style.arrow_width)
            .height_pct(100.0)
            // v1's `.items_center()` on the arrow's box.
            .main_align(Align::Center)
            .cross_align(Align::Center)
            .absolute_fill();
        request.inset = Some(if next {
            Inset {
                left: None,
                top: Some(0.0),
                right: Some(0.0),
                bottom: None,
            }
        } else {
            Inset {
                left: Some(0.0),
                top: Some(0.0),
                right: None,
                bottom: None,
            }
        });

        Child::new(
            ChildKey::keyed(if next { NEXT_ARROW_KEY } else { PREVIOUS_ARROW_KEY }),
            request,
            move || {
                // The slot is a replayed `Fn`, so the handles it hands to the
                // click are copies of its own.
                let shared = shared.clone();
                let on_change = on_change.clone();
                Box::new(
                    Pressable::new(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new(),
                        move || Box::new(Text::new(glyph).size(font_size).color(ink)),
                    ))
                    .on_click(move || {
                        let current = shared.load(Ordering::SeqCst);
                        let target = if next {
                            if current + 1 >= count {
                                if continuous { 0 } else { count - 1 }
                            } else {
                                current + 1
                            }
                        } else if current == 0 {
                            if continuous {
                                count - 1
                            } else {
                                0
                            }
                        } else {
                            current - 1
                        };
                        shared.store(target, Ordering::SeqCst);
                        if let Some(cb) = &on_change {
                            cb(target);
                        }
                    }),
                )
            },
        )
    }

    /// The dots along the bottom of the box: one per page, the active one
    /// filled with the primary.
    fn dots(&self, index: usize) -> Child {
        let style = self.style;
        let count = self.pages.len();
        let mut row = Stack::new(Axis::Horizontal)
            // v1's `.justify_center().items_center()` on the row.
            .align(Align::Center, Align::Center);

        for dot in 0..count {
            let active = dot == index;
            let fill = to_paint(if active {
                style.primary
            } else {
                style.dot_inactive_color
            });
            let size = style.dot_size;
            let radius = style.dot_size / 2.0;
            let shared = self.index.clone();
            let on_change = self.on_change.clone();
            row = row.push_layout(
                // v1's `.margin(dot_gap / 2.0)` on each dot.
                LayoutRequest::new().margin_horiz(f64::from(style.dot_gap / 2.0)),
                move || {
                    // The slot is a replayed `Fn`, so the handles it hands to
                    // the click are copies of its own.
                    let shared = shared.clone();
                    let on_change = on_change.clone();
                    Box::new(
                        Pressable::new(Child::new(
                            ChildKey::index(0),
                            LayoutRequest::new().width(size).height(size),
                            move || Box::new(Surface::new().background(fill).radius(radius)),
                        ))
                        .on_click(move || {
                            shared.store(dot, Ordering::SeqCst);
                            if let Some(cb) = &on_change {
                                cb(dot);
                            }
                        }),
                    )
                },
            );
        }

        let mut request = LayoutRequest::new()
            .height(style.dot_size * 2.0)
            // v1's `.items_center().justify_center()` on the dots' box.
            .main_align(Align::Center)
            .cross_align(Align::Center)
            .absolute_fill();
        request.inset = Some(Inset {
            left: Some(0.0),
            top: None,
            right: Some(0.0),
            // v1's `inset_bottom_pct(2.0)`: a percentage of the box's height.
            bottom: Some(self.height * style.dots_inset_pct / 100.0),
        });

        Child::new(
            // The index is part of the key, so the dots are rebuilt -- and the
            // active one changes colour -- when the page changes.
            ChildKey::keyed(DOTS_KEY + index as u64),
            request,
            move || Box::new(row.clone()),
        )
    }
}

impl Widget for FluentWindowViewer {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    /// The page moved: the children have to be declared again, because the
    /// page on show is a different slot and the dots are a different colour.
    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
        let current = self.current();
        if current != self.declared {
            self.declared = current;
            cx.mark_children_dirty(id);
        }
    }

    fn children(&self) -> Vec<Child> {
        let index = self.current();
        let mut out = Vec::new();

        // The page on show, and only the page on show: 2.0 has no `hide()`.
        if let Some(page) = self.pages.get(index) {
            out.push(Child {
                // The index is part of the key, so the page that was on show
                // is unmounted and the new one mounted.
                key: ChildKey::keyed(PAGE_KEY + index as u64),
                // v1's page was absolute, at the box's top left, and as big as
                // the box.
                layout: LayoutRequest::new().absolute_fill(),
                build: page.build.clone(),
                shared_key: page.shared_key,
            });
        }

        if self.show_arrows && self.pages.len() > 1 {
            out.push(self.arrow(false));
            out.push(self.arrow(true));
        }

        if self.show_dots && !self.pages.is_empty() {
            out.push(self.dots(index));
        }

        out
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        // v1's box: the full width of its parent, and the height.
        Some(
            LayoutRequest::new()
                .width_pct(100.0)
                .height(self.height),
        )
    }

    fn paint(&self, cx: &mut dyn PaintContext, id: ElementId) {
        self.decoration.paint(cx, id);
    }

    /// v1 clipped the box's contents to the box.
    fn clip(&self) -> Option<ClipShape> {
        Some(ClipShape::RoundedBox {
            radius: f64::from(self.style.border_radius),
        })
    }
}

impl floom_core::WidgetAny for FluentWindowViewer {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// The viewer publishes nothing of its own: the page inside it, and the
    /// arrows and the dots, are what speak.
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
