//! `Carousel` -- a box with one slide on show, arrows and dots.
//!
//! Only the current slide is mounted; the arrows move between the slides and
//! the dots say which one is on show. The three floem-ui carousel kits each
//! used to carry their own copy of this; they share the engine's one now.
//!
//! ## What the kits still answer
//!
//! The box's fill and radius, the arrow glyphs' ink, the dots' ink, the
//! outline's colour, width and alpha, the box's height, the dots' size and
//! gap, the inactive dots' opacity, the arrow glyph's size and width, and
//! the token's slide gap, which v1 never read.
//!
//! ## The slide on show travels in shared flags
//!
//! One flag per slide: exactly one of them is set, and that one is the slide
//! on screen. A flag changes after mount -- a click moves it -- so it cannot
//! live in a signal, which is `Rc`-based and cannot cross a slot boundary.

use std::sync::Arc;

use floom_core::{
    Child, ChildKey, Color, CreateContext, ElementId, Inset, LayoutRequest, PaintContext,
    Insets, Position, UpdateContext, Widget, WidgetAny,
};
use floom_types::CursorStyle;
use floom_widgets::{
    Align, Axis, Conditional, Pressable, SharedFlag, Stack, Surface, Text, read_flag, write_flag,
};

use crate::color_picker::with_alpha;

/// The dots' distance from the bottom edge, as a fraction of the box's height.
/// A v1 literal.
const DOTS_BOTTOM_RATIO: f32 = 0.02;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarouselStyle {
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

impl Default for CarouselStyle {
    fn default() -> Self {
        Self {
            surface: Color::rgb(0, 0, 0),
            on_surface: Color::rgb(0, 0, 0),
            on_surface_variant: Color::rgb(0, 0, 0),
            primary: Color::rgb(0, 0, 0),
            border_radius: 0.0,
            height: 0.0,
            dot_size: 0.0,
            arrow_size: 0.0,
            outline: Color::rgb(0, 0, 0),
            border_width: 0.0,
            border_alpha: 0,
            dot_gap: 0.0,
            dot_inactive_opacity: 0.0,
            arrow_font_size: 0.0,
            arrow_width: 0.0,
            slide_gap: 0.0,
        }
    }
}

pub fn current_index(flags: &[SharedFlag]) -> usize {
    flags.iter().position(|flag| read_flag(flag)).unwrap_or(0)
}

/// Make one slide the showing one, and the others not.
pub fn set_index(flags: &[SharedFlag], index: usize) {
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
struct CarouselDot {
    flag: SharedFlag,
    size: f32,
    radius: f32,
    active: Color,
    inactive: Color,
    /// Whether the dot was the current one as of the last update.
    reported: bool,
}

impl Widget for CarouselDot {
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

impl WidgetAny for CarouselDot {
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

/// The carousel: the box, the slides, the arrows and the dots.
pub struct Carousel {
    /// The kit's answer.
    style: CarouselStyle,
    /// The slides, in order.
    slides: Vec<Child>,
    /// One flag per slide: exactly one of them is set, and that one is the
    /// slide on screen.
    flags: Vec<SharedFlag>,
    /// Called with the slide's index whenever the choice moves.
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
    /// Whether the arrows are shown.
    show_arrows: bool,
    /// Whether the dots are shown.
    show_dots: bool,
}

impl Carousel {
    pub fn new() -> Self {
        Self {
            style: CarouselStyle::default(),
            slides: Vec::new(),
            flags: Vec::new(),
            on_change: None,
            show_arrows: false,
            show_dots: false,
        }
    }

    /// The carousel's appearance.
    pub fn style(mut self, style: CarouselStyle) -> Self {
        self.style = style;
        self
    }

    /// The slides, in order.
    pub fn slides(mut self, slides: Vec<Child>) -> Self {
        self.slides = slides;
        self
    }

    /// One flag per slide: exactly one of them is set, and that one is the
    /// slide on screen.
    pub fn flags(mut self, flags: Vec<SharedFlag>) -> Self {
        self.flags = flags;
        self
    }

    /// Called with the slide's index whenever the choice moves.
    pub fn on_change(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// Whether the arrows are shown.
    pub fn show_arrows(mut self, show: bool) -> Self {
        self.show_arrows = show;
        self
    }

    /// Whether the dots are shown.
    pub fn show_dots(mut self, show: bool) -> Self {
        self.show_dots = show;
        self
    }

    /// The carousel, ready to mount.
    pub fn build(self) -> Surface {

        let style = self.style;
        let count = self.slides.len();
        let flags = self.flags.clone();
        let on_change = self.on_change.clone();

        let surface = style.surface;
        let on_surface = style.on_surface;
        let primary = style.primary;
        let border_color = with_alpha(style.outline, style.border_alpha);
        let arrow_font_size = style.arrow_font_size;

        let mut area = Stack::new(Axis::Vertical);
        let mut key = 0usize;

        // The slides: only the current one is mounted.
        for (index, slide) in self.slides.clone().into_iter().enumerate() {
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
        if self.show_arrows && count > 1 {
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
        if self.show_dots && count > 0 {
            let mut dots = Stack::new(Axis::Horizontal)
                // v1's `justify_center().items_center().gap(0.0)`.
                .align(Align::Center, Align::Center)
                .gap(0.0);
            for index in 0..count {
                let dot = Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new(),
                    {
                        let dot = CarouselDot {
                            flag: flags[index].clone(),
                            size: style.dot_size,
                            // v1's `border_radius(dot_size / 2.0)`: a circle.
                            radius: style.dot_size / 2.0,
                            active: primary,
                            inactive: with_alpha(
                                style.on_surface,
                                (style.dot_inactive_opacity * 255.0) as u8,
                            ),
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
