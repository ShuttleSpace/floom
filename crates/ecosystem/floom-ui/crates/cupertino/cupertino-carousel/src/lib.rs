//! Cupertino carousel, ported to the Floom 2.0 widget API.
//!
//! A carousel shows one slide at a time, with arrows to step through them
//! and a row of dots to jump to one.
//!
//! ## What the port uses
//!
//! The engine's vocabulary:
//!
//! * a [`Surface`] for the slide area: the fill, the radius, the outline;
//! * a `Stack` for the area's contents, every one of them absolutely
//!   positioned the way v1 positioned them;
//! * [`Conditional`] for each slide's *presence*, so that only the current
//!   slide is mounted at all -- v1's `hide()`;
//! * [`Pressable`] for the arrows and the dots.
//!
//! ## The current slide is a set of flags
//!
//! v1 held the index in an `RwSignal<usize>` and every arrow and dot wrote
//! it. 2.0's slots are `Send + Sync` and an `RwSignal` is `Rc`-based, so the
//! index cannot be one. It is a *set of shared flags* instead -- one per
//! slide, exactly one of them set -- and the index is derived from them by
//! looking for the one that is set. That keeps a single source of truth: the
//! flags are the state, and the index is a reading of it, not a second copy.
//!
//! The engine's shared types cover a bool, a string and a map; a *set of
//! bools* is the first of those, repeated, and it is what the presence
//! machinery already speaks.
//!
//! ## What this kit answers
//!
//! * the area's fill: `surface`; its outline: `outline` at 80 alpha;
//! * the arrows' ink: `on_surface`; the current dot's: `primary`; the other
//!   dots': `on_surface` at the token's inactive opacity;
//! * the area's height: the *prop's* when it is positive, the token's
//!   otherwise;
//! * the radius, the dot's size and gap, and the arrow's size: the tokens.
//!
//! ## The token that never showed
//!
//! `CarouselTokens::slide_gap` (8) is read by nobody in v1: the slides were
//! absolutely positioned on top of each other, so there was no gap to keep.
//! The port keeps the number in the style and names it here.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The auto-play never ran.** v1's props carry `auto_play` and
//!   `interval_ms`, and its view reads neither: there is no timer in the
//!   kit. The port keeps both props, because they are part of the API a
//!   caller sets, and names them here.
//! * **The area was clipped.** v1 wrote `overflow_x(Clip).overflow_y(Clip)`
//!   on the area. 2.0's `Surface` has no clip builder -- a widget declares
//!   a clip through the `Widget` trait, and the box does not -- so the
//!   slides are not clipped. They fill the area, so it only shows when a
//!   slide's own content overflows.
//! * **The arrows' pointer cursor.** v1 set `cursor(Pointer)` on the arrows
//!   and on the dots. The dots keep it, because the dot is this kit's own
//!   widget and can answer it; the arrows do not, because 2.0's `Pressable`
//!   does not ask for a cursor and the arrow's child is a plain `Text`.
//! * **The slides are slots, not views.** v1's `slides(Vec<AnyView>)` took
//!   views; 2.0's takes [`Child`] slots, because a 2.0 widget is mounted by
//!   its parent rather than returned.
//! * **The accessors.** v1's carousel had no `props()`, `theme()` or
//!   `with_theme()`. The port gives all three kits the same three, because a
//!   kit that cannot be given a theme cannot be tested against one.

use std::sync::Arc;

use floom_core::{Child, ChildKey, CreateContext, ElementId, Inset, LayoutRequest, PaintContext, Position,
    UpdateContext, Widget, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::{CursorStyle, Insets};
use floom_widgets::text_input::{SharedFlag, read_flag, shared_flag, write_flag};
use floom_widgets::{Align, Axis, Conditional, Pressable, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::carousel::CarouselProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

/// The outline's alpha. A v1 literal.
const BORDER_ALPHA: u8 = 80;
/// The outline's width. A v1 literal.
const BORDER_WIDTH: f32 = 1.0;
/// The arrow's width. A v1 literal.
const ARROW_WIDTH: f32 = 40.0;
/// The arrow glyph's size, as a fraction of the arrow's size. A v1 literal.
const ARROW_FONT_RATIO: f32 = 0.6;
/// The dots' distance from the bottom, as a fraction of the area's height.
/// A v1 literal.
const DOTS_BOTTOM_RATIO: f32 = 0.02;

/// The carousel's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoCarouselStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub on_surface: Color,
    /// v1 reported this and the view never used it -- see the module note.
    pub on_surface_variant: Color,
    pub primary: Color,
    pub border_radius: f32,
    pub height: f32,
    pub dot_size: f32,
    pub arrow_size: f32,
    /// The outline's colour, before the alpha below. v1's style did not
    /// report it.
    pub outline: Color,
    /// The outline's width. A v1 literal.
    pub border_width: f32,
    /// The outline's alpha. A v1 literal.
    pub border_alpha: u8,
    /// The gap between the dots. The token.
    pub dot_gap: f32,
    /// The opacity of the dots that are not the current one. The token.
    pub dot_inactive_opacity: f32,
    /// The arrow glyph's size: v1 wrote `arrow_size * 0.6`.
    pub arrow_font_size: f32,
    /// The arrow's width. A v1 literal.
    pub arrow_width: f32,
    /// The token's slide gap. v1 never read it -- see the module note.
    pub slide_gap: f32,
}

/// Cupertino's carousel.
pub struct CupertinoCarousel {
    props: CarouselProps,
    theme: RwSignal<MultiTheme>,
    slides: Vec<Child>,
    height: f32,
    /// One flag per slide: exactly one of them is set, and that one is the
    /// slide on screen.
    flags: Vec<SharedFlag>,
    /// v1's `Box<dyn Fn(usize)>`, kept in a shareable form so a slot closure
    /// can hold it. The props' own slot stays filled so `props()` still
    /// reports one.
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
}

impl CupertinoCarousel {
    pub fn new() -> Self {
        Self {
            props: CarouselProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino,)),
            slides: Vec::new(),
            height: 0.0,
            flags: Vec::new(),
            on_change: None,
        }
    }

    /// The slides, in order. The first one is showing.
    pub fn slides(mut self, slides: Vec<Child>) -> Self {
        self.flags = slides.iter().map(|_| shared_flag(false)).collect();
        if let Some(first) = self.flags.first() {
            write_flag(first, true);
        }
        self.slides = slides;
        self
    }

    /// The area's height. v1's rule: the prop's when it is positive, the
    /// token's otherwise.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Whether the carousel advances on a timer.
    ///
    /// v1 carried this and never ran it -- see the module note.
    pub fn auto_play(mut self, auto_play: bool) -> Self {
        self.props.auto_play = auto_play;
        self
    }

    /// The auto-advance interval, in milliseconds.
    ///
    /// v1 carried this and never ran it -- see the module note.
    pub fn interval_ms(mut self, interval_ms: u64) -> Self {
        self.props.interval_ms = interval_ms;
        self
    }

    /// Whether the arrows are drawn.
    pub fn show_arrows(mut self, show_arrows: bool) -> Self {
        self.props.show_arrows = show_arrows;
        self
    }

    /// Whether the dots are drawn.
    pub fn show_dots(mut self, show_dots: bool) -> Self {
        self.props.show_dots = show_dots;
        self
    }

    /// Called with the new index when the slide changes.
    ///
    /// `Send + Sync`, because 2.0's slots are shared between threads and v1's
    /// `Box<dyn Fn(usize)>` was not.
    pub fn on_change(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        let shared: Arc<dyn Fn(usize) + Send + Sync + 'static> = Arc::new(callback);
        self.props.on_change = Some(Box::new({
            let shared = shared.clone();
            // The props' callback takes no index; the shared one does, and the
            // slide that is showing is the one it names.
            move |index| shared(index)
        }));
        self.on_change = Some(shared);
        self
    }

    /// The slide that is showing.
    pub fn index(&self) -> usize {
        current_index(&self.flags)
    }

    /// Show a slide, as the arrows and the dots do.
    pub fn set_index(&self, index: usize) {
        set_index(&self.flags, index);
    }

    pub fn props(&self) -> &CarouselProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoCarouselStyle {
        let theme = self.theme.get();
        let ComponentTokens::Carousel(tokens) = theme.component(ComponentId::Carousel) else {
            unreachable!("ComponentId::Carousel must resolve to CarouselTokens")
        };
        let height = if self.height > 0.0 {
            self.height
        } else {
            tokens.height
        };
        CupertinoCarouselStyle {
            design_system: "cupertino",
            surface: theme.colors().surface,
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            primary: theme.colors().primary,
            border_radius: tokens.border_radius,
            height,
            dot_size: tokens.dot_size,
            arrow_size: tokens.arrow_size,
            outline: theme.colors().outline,
            border_width: BORDER_WIDTH,
            border_alpha: BORDER_ALPHA,
            dot_gap: tokens.dot_gap,
            dot_inactive_opacity: tokens.dot_inactive_opacity,
            arrow_font_size: tokens.arrow_size * ARROW_FONT_RATIO,
            arrow_width: ARROW_WIDTH,
            slide_gap: tokens.slide_gap,
        }
    }

    /// The carousel: the area, the slides, the arrows and the dots.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let count = self.slides.len();
        let flags = self.flags.clone();
        let on_change = self.on_change.clone();

        let surface = to_paint(style.surface);
        let on_surface = to_paint(style.on_surface);
        let primary = to_paint(style.primary);
        let border_color = to_paint(with_alpha(style.outline, style.border_alpha));
        let arrow_font_size = style.arrow_font_size;

        let mut area = Stack::new(Axis::Vertical);
        let mut key = 0usize;

        // The slides: only the current one is mounted.
        for (index, slide) in self.slides.into_iter().enumerate() {
            let flag = flags[index].clone();
            area = area.child(Child::new(
                ChildKey::index(key),
                // v1's slides were absolutely positioned over the area.
                LayoutRequest::new().absolute_fill(),
                move || {
                    Box::new(
                        Conditional::new(false)
                            .when_flag(flag.clone())
                            .show(Child::new(
                                ChildKey::index(0),
                                LayoutRequest::new().width_pct(100.0).height_pct(100.0),
                                {
                                    let slide = slide.clone();
                                    move || slide.build()
                                },
                            )),
                    ) as Box<dyn WidgetAny>
                },
            ));
            key += 1;
        }

        // The arrows: only when there is somewhere to go.
        //
        // `Pressable` is a builder rather than a `Clone` value, so each arrow
        // is built inside the closure that mounts it.
        if self.props.show_arrows && count > 1 {
            area = area.child(Child::new(
                ChildKey::index(key),
                arrow_request(true, style.arrow_width),
                {
                    let flags_prev = flags.clone();
                    let on_change_prev = on_change.clone();
                    move || {
                        let flags_prev = flags_prev.clone();
                        let on_change_prev = on_change_prev.clone();
                        Box::new(
                            Pressable::new(Child::new(
                                ChildKey::index(0),
                                // v1's `items_center()` on a row: the glyph is
                                // centred on the cross axis.
                                LayoutRequest::new().cross_align(Align::Center),
                                move || {
                                    Box::new(
                                        Text::new("‹").size(arrow_font_size).color(on_surface),
                                    )
                                },
                            ))
                            .on_click(move || {
                                let current = current_index(&flags_prev);
                                let next = if current == 0 { count - 1 } else { current - 1 };
                                set_index(&flags_prev, next);
                                if let Some(on_change) = &on_change_prev {
                                    on_change(next);
                                }
                            }),
                        ) as Box<dyn WidgetAny>
                    }
                },
            ));
            key += 1;

            area = area.child(Child::new(
                ChildKey::index(key),
                arrow_request(false, style.arrow_width),
                {
                    let flags_next = flags.clone();
                    let on_change_next = on_change.clone();
                    move || {
                        let flags_next = flags_next.clone();
                        let on_change_next = on_change_next.clone();
                        Box::new(
                            Pressable::new(Child::new(
                                ChildKey::index(0),
                                LayoutRequest::new().cross_align(Align::Center),
                                move || {
                                    Box::new(
                                        Text::new("›").size(arrow_font_size).color(on_surface),
                                    )
                                },
                            ))
                            .on_click(move || {
                                let current = current_index(&flags_next);
                                let next = if current >= count - 1 { 0 } else { current + 1 };
                                set_index(&flags_next, next);
                                if let Some(on_change) = &on_change_next {
                                    on_change(next);
                                }
                            }),
                        ) as Box<dyn WidgetAny>
                    }
                },
            ));
            key += 1;
        }

        // The dots: one per slide, the current one in the primary colour.
        if self.props.show_dots && count > 0 {
            let mut dots = Stack::new(Axis::Horizontal)
                // v1's `justify_center().items_center().gap(0.0)`.
                .align(Align::Center, Align::Center)
                .gap(0.0);
            for index in 0..count {
                let dot = Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new(),
                    {
                        let dot = CupertinoCarouselDot {
                            flag: flags[index].clone(),
                            size: style.dot_size,
                            // v1's `border_radius(dot_size / 2.0)`: a circle.
                            radius: style.dot_size / 2.0,
                            active: primary,
                            inactive: to_paint(with_alpha(
                                style.on_surface,
                                (style.dot_inactive_opacity * 255.0) as u8,
                            )),
                            reported: false,
                        };
                        move || Box::new(dot.clone())
                    },
                );
                dots = dots.child(Child::new(
                    ChildKey::index(index),
                    LayoutRequest::new()
                        .width(style.dot_size)
                        .height(style.dot_size)
                        // v1's `margin(dot_gap / 2.0)`.
                        .margin(Insets::uniform(f64::from(style.dot_gap / 2.0))),
                    {
                        let flags_dot = flags.clone();
                        let on_change_dot = on_change.clone();
                        move || {
                            let flags_dot = flags_dot.clone();
                            let on_change_dot = on_change_dot.clone();
                            Box::new(
                                Pressable::new(dot.clone()).on_click(move || {
                                    set_index(&flags_dot, index);
                                    if let Some(on_change) = &on_change_dot {
                                        on_change(index);
                                    }
                                }),
                            ) as Box<dyn WidgetAny>
                        }
                    },
                ));
            }
            // v1's overlay: the row, at the bottom of the area, centred.
            let overlay = Stack::new(Axis::Horizontal)
                .align(Align::Center, Align::Center)
                .child(Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new(),
                    move || Box::new(dots.clone()),
                ));
            area = area.child(Child::new(
                ChildKey::index(key),
                dots_request(style.height, style.dot_size),
                move || Box::new(overlay.clone()),
            ));
        }

        Surface::new()
            .request(LayoutRequest::new().width_pct(100.0).height(style.height))
            .background(surface)
            .radius(style.border_radius)
            .border(style.border_width, border_color)
            .child(Child::new(
                ChildKey::index(0),
                LayoutRequest::new().fill(),
                move || Box::new(area.clone()),
            ))
    }
}

impl Default for CupertinoCarousel {
    fn default() -> Self {
        Self::new()
    }
}

/// The index of the slide that is showing.
fn current_index(flags: &[SharedFlag]) -> usize {
    flags.iter().position(|flag| read_flag(flag)).unwrap_or(0)
}

/// Make one slide the showing one, and the others not.
fn set_index(flags: &[SharedFlag], index: usize) {
    for (position, flag) in flags.iter().enumerate() {
        write_flag(flag, position == index);
    }
}

/// One indicator dot: its size, its colour, and whether it is the current one.
///
/// A kit-local widget because the dot's *colour* follows the current slide,
/// and a child's decoration is fixed when it is built: the engine has no
/// widget that repaints itself when a shared flag moves.
#[derive(Clone)]
struct CupertinoCarouselDot {
    flag: SharedFlag,
    size: f32,
    radius: f32,
    active: PaintColor,
    inactive: PaintColor,
    /// Whether the dot was the current one as of the last update.
    reported: bool,
}

impl Widget for CupertinoCarouselDot {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let current = read_flag(&self.flag);
        if current != self.reported {
            self.reported = current;
            // The dot's colour is the only thing that moved.
            cx.mark_dirty(id);
        }
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().width(self.size).height(self.size))
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let color = if read_flag(&self.flag) {
            self.active
        } else {
            self.inactive
        };
        let bounds = cx.bounds();
        cx.fill_rounded_rect(bounds, self.radius, color);
    }

    fn cursor(&self) -> CursorStyle {
        // v1's dots were clickable and said so with the cursor.
        CursorStyle::Pointer
    }
}

impl WidgetAny for CupertinoCarouselDot {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

/// The arrow's own box: v1's absolute arrow, pinned to the chosen edge.
fn arrow_request(left: bool, width: f32) -> LayoutRequest {
    let mut request = LayoutRequest::new().width(width).height_pct(100.0);
    request.position = Some(Position::Absolute);
    request.inset = Some(Inset {
        left: if left { Some(0.0) } else { None },
        top: Some(0.0),
        right: if left { None } else { Some(0.0) },
        bottom: None,
    });
    request
}

/// The dots' overlay: v1's absolute strip along the bottom of the area.
fn dots_request(height: f32, dot_size: f32) -> LayoutRequest {
    let mut request = LayoutRequest::new().height(dot_size * 2.0);
    request.position = Some(Position::Absolute);
    request.inset = Some(Inset {
        left: Some(0.0),
        top: None,
        right: Some(0.0),
        // v1 wrote `inset_bottom_pct(2.0)`: two per cent of the area's
        // height, which the kit knows.
        bottom: Some(height * DOTS_BOTTOM_RATIO),
    });
    request
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
