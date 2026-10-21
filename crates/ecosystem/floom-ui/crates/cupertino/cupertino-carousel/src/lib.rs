//! Cupertino carousel, ported to the Floom 2.0 widget API.
//!
//! A carousel shows one slide at a time, with arrows to step through them
//! and a row of dots to jump to one.
//!
//! ## What the port uses
//!
//! The engine's `Carousel`: the box, the slides, the arrows and the dots. The
//! three floem-ui carousel kits each used to carry their own copy of all of
//! it, down to the dot that repaints itself when the slide moves.
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

use floom_core::Child;
use floom_signal::{RwSignal, SignalGet};
use floom_ui_composites::{Carousel, CarouselStyle, current_index, set_index};
use floom_types::Color as PaintColor;
use floom_widgets::text_input::{SharedFlag, shared_flag, write_flag};
use floom_widgets::Surface;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::carousel::CarouselProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

/// The outline's alpha. A v1 literal.
const BORDER_ALPHA: u8 = 80;
/// The outline's width. A v1 literal.
const BORDER_WIDTH: f32 = 1.0;
/// The arrow's width. A v1 literal.
const ARROW_WIDTH: f32 = 40.0;
/// The arrow glyph's size, as a fraction of the arrow's size. A v1 literal.
const ARROW_FONT_RATIO: f32 = 0.6;
/// The dots' distance from the bottom, as a fraction of the area's height.
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
        let mut carousel = Carousel::new()
            .style(CarouselStyle {
                surface: to_paint(style.surface),
                on_surface: to_paint(style.on_surface),
                on_surface_variant: to_paint(style.on_surface_variant),
                primary: to_paint(style.primary),
                border_radius: style.border_radius,
                height: style.height,
                dot_size: style.dot_size,
                arrow_size: style.arrow_size,
                outline: to_paint(style.outline),
                border_width: style.border_width,
                border_alpha: style.border_alpha,
                dot_gap: style.dot_gap,
                dot_inactive_opacity: style.dot_inactive_opacity,
                arrow_font_size: style.arrow_font_size,
                arrow_width: style.arrow_width,
                slide_gap: style.slide_gap,
            })
            .slides(self.slides.clone())
            .flags(self.flags.clone())
            .show_arrows(self.props.show_arrows)
            .show_dots(self.props.show_dots);
        if let Some(on_change) = self.on_change.clone() {
            carousel = carousel.on_change(move |index| on_change(index));
        }
        carousel.build()
    }
}

impl Default for CupertinoCarousel {
    fn default() -> Self {
        Self::new()
    }
}

/// The index of the slide that is showing.

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
