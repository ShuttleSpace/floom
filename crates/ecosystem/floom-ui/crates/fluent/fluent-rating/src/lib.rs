//! Fluent rating, ported to the Floom 2.0 widget API.
//!
//! A rating is a row of stars: the first `value` of them are filled, the rest
//! are empty, and tapping a star sets the value to that star's number.
//!
//! ## What the port uses
//!
//! The engine's vocabulary: a horizontal `Stack` of stars, each a `Pressable`
//! around a slot that holds one of two glyphs -- the same `star` path in two
//! inks. Which glyph is mounted is the engine's `Conditional`, following a
//! shared flag, so the row follows whoever writes the flag the way v1's
//! signal did.
//!
//! ## What this kit answers
//!
//! * the star's size and the gap between the stars: the tokens;
//! * the filled star's ink: `primary`;
//! * the empty star's ink: `outline_variant`;
//! * the glyph: the `star` path from this kit's own icon table.
//!
//! ## The value, and how it is held
//!
//! v1 held the value in an `RwSignal<f32>` that `view()` created, so nothing
//! outside could read it. 2.0 cannot hold it in a signal at all: a slot
//! closure has to be `Send + Sync` and an `RwSignal` is `Rc`-based. The port
//! holds **one shared flag per star** -- "is this star filled?" -- which is
//! the state the paint actually needs. The filled stars are a prefix, so the
//! number v1 held is the length of that prefix, and a click rewrites the
//! flags rather than a number.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The pointer cursor.** v1 gave every star
//!   `cursor(CursorStyle::Pointer)`, and `CursorStyle::Default` when the
//!   rating was read-only. 2.0's `Pressable` does not answer
//!   `Widget::cursor`, so the pointer is not carried over.
//! * **The callback in the shared props.** `RatingProps::on_change` is a
//!   `Box<dyn Fn(f32)>`, which cannot cross a `Send + Sync` slot boundary.
//!   The kit keeps an `Arc<dyn Fn(f32) + Send + Sync>` of its own and the
//!   props' slot stays empty.
//!
//! ## What the port adds
//!
//! * **A name for every star.** 2.0's widgets publish semantics and v1's
//!   did not; the port gives each star a name ("Rate 3 of 5"), which is an
//!   addition rather than a translation.
//! * **The keyboard is gated too.** v1's read-only rating guarded the click
//!   with `if !readonly`, which covered the pointer but not the keyboard.
//!   2.0 puts the gate on the `Pressable`, so both are covered.
//!
//! ## The accessors
//!
//! v1's Fluent rating already had `props()`, `theme()` and `with_theme()`.
//! The port gives Material the same three, so the three kits are one shape.

use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::text_input::{SharedFlag, shared_flag, write_flag};
use floom_widgets::{Align, Axis, Conditional, Pressable, Stack, VectorIcon};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::rating::RatingProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_widget_fluent_icon::{VIEW_BOX, fluent_icon_path};

const DESIGN_SYSTEM: &str = "fluent";

/// The glyph the rating draws. This kit's own icon table, not a hard-coded
/// path: the icon set is the design system's, and the three kits' tables are
/// separate crates.
const STAR: &str = "star";

/// A callback in the shape 2.0's slots need.
type Callback = Arc<dyn Fn(f32) + Send + Sync + 'static>;

/// One star's state: the two flags that choose between its two glyphs.
///
/// The filled flag is "is this star filled?", which is the state v1's paint
/// read out of the value signal. The empty flag is its complement, and it
/// exists because the engine's `Conditional` mounts a child while a flag is
/// true -- so showing the *other* glyph needs a flag of its own.
#[derive(Debug, Clone)]
struct StarState {
    filled: SharedFlag,
    empty: SharedFlag,
}

/// The rating's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentRatingStyle {
    pub design_system: &'static str,
    /// The filled star's ink: the theme's `primary`.
    pub active_color: Color,
    /// The empty star's ink.
    pub inactive_color: Color,
    pub star_size: f32,
    pub gap: f32,
}

/// Fluent's rating.
pub struct FluentRating {
    props: RatingProps,
    theme: RwSignal<MultiTheme>,
    /// One entry per star: the flags that say which glyph it shows.
    stars: Vec<StarState>,
    /// The change callback in the shape 2.0's slots need.
    on_change: Option<Callback>,
}

impl FluentRating {
    pub fn new() -> Self {
        let mut kit = Self {
            props: RatingProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            stars: Vec::new(),
            on_change: None,
        };
        kit.reset_stars();
        kit
    }

    /// The rating's value. Clamped into `0..=max` when the stars are rebuilt,
    /// exactly as v1 clamped it when it built the view.
    pub fn value(mut self, value: f32) -> Self {
        self.props.value = value;
        self.reset_stars();
        self
    }

    /// How many stars the rating has. v1's `max.max(1)`: a rating with no
    /// stars still showed one.
    pub fn max(mut self, max: usize) -> Self {
        self.props.max = max;
        self.reset_stars();
        self
    }

    /// A read-only rating does not answer a tap.
    pub fn readonly(mut self, readonly: bool) -> Self {
        self.props.readonly = readonly;
        self
    }

    /// Called with the new value when a star is tapped.
    pub fn on_change(mut self, callback: impl Fn(f32) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// The shared props.
    ///
    /// `on_change` is deliberately absent from what it reports: the kit holds
    /// that as an `Arc` of its own (see the field's note), and `RatingProps` is
    /// the v1 carrier of a `Box<dyn Fn(..)>` that 2.0 cannot use.
    pub fn props(&self) -> &RatingProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// The per-star flags, for a caller that wants to read or drive the state.
    ///
    /// The value v1 held in a signal is these flags: the filled stars are a
    /// prefix, so the value is the number of flags that are true.
    pub fn filled_flags(&self) -> Vec<SharedFlag> {
        self.stars.iter().map(|star| star.filled.clone()).collect()
    }

    /// Rebuilds the per-star flags from the current value and maximum.
    ///
    /// The flags *are* the value: a star is filled when its index is below the
    /// value, exactly as v1's paint decided. Rebuilding them keeps the two in
    /// step whatever order the builder methods were called in.
    fn reset_stars(&mut self) {
        let max = self.props.max.max(1);
        let value = self.props.value.clamp(0.0, max as f32);
        self.stars = (0..max)
            .map(|index| {
                let filled = (index as f32) < value;
                StarState {
                    filled: shared_flag(filled),
                    empty: shared_flag(!filled),
                }
            })
            .collect();
    }

    pub fn resolved_style(&self) -> FluentRatingStyle {
        let theme = self.theme.get();
        let ComponentTokens::Rating(tokens) = theme.component(ComponentId::Rating) else {
            unreachable!("ComponentId::Rating must resolve to RatingTokens")
        };
        FluentRatingStyle {
            design_system: DESIGN_SYSTEM,
            // v1: the accent colour, not a literal.
            active_color: theme.colors().primary,
            inactive_color: theme.colors().outline_variant,
            star_size: tokens.star_size,
            gap: tokens.gap,
        }
    }

    /// The row of stars.
    pub fn view(self) -> Stack {
        let style = self.resolved_style();
        let readonly = self.props.readonly;
        let on_change = self.on_change.clone();
        let active = to_paint(style.active_color);
        let inactive = to_paint(style.inactive_color);
        let star_size = style.star_size;
        let path = fluent_icon_path(STAR);
        let max = self.stars.len();

        let mut row = Stack::new(Axis::Horizontal)
            // v1's `.flex_row().gap(gap)`.
            .gap(style.gap)
            // v1's `.items_center()`.
            .align(Align::Start, Align::Center);

        for index in 0..max {
            let state = self.stars[index].clone();
            // Every star's flags: a click rewrites them all, because the
            // filled stars are a prefix.
            let all = self.stars.clone();
            let on_change = on_change.clone();
            let active = active;
            let inactive = inactive;
            let star_size = star_size;
            let path = path;
            let value = index as f32 + 1.0;

            row = row.push(move || {
                // The click closure is built inside the slot, so the values it
                // takes are copied here: a slot is replayed, and a closure
                // cannot move its own captures out.
                let all = all.clone();
                let on_change = on_change.clone();
                // The two glyphs: the star filled, and the star empty. They are
                // the same path in two inks.
                let filled_glyph = VectorIcon::from_path_data(path, VIEW_BOX)
                    .size(star_size)
                    .color(active);
                let empty_glyph = VectorIcon::from_path_data(path, VIEW_BOX)
                    .size(star_size)
                    .color(inactive);

                // The slot: the engine's `Conditional` mounts the filled glyph
                // while the star is filled, and the empty one otherwise.
                let slot = Stack::new(Axis::Vertical)
                    .push({
                        let filled_glyph = filled_glyph.clone();
                        let flag = state.filled.clone();
                        move || {
                            Box::new(
                                Conditional::new(false)
                                    .when_flag(flag.clone())
                                    .show(Child::new(
                                        ChildKey::index(0),
                                        LayoutRequest::new(),
                                        {
                                            let filled_glyph = filled_glyph.clone();
                                            move || Box::new(filled_glyph.clone())
                                        },
                                    )),
                            )
                        }
                    })
                    .push({
                        let empty_glyph = empty_glyph.clone();
                        let flag = state.empty.clone();
                        move || {
                            Box::new(
                                Conditional::new(false)
                                    .when_flag(flag.clone())
                                    .show(Child::new(
                                        ChildKey::index(0),
                                        LayoutRequest::new(),
                                        {
                                            let empty_glyph = empty_glyph.clone();
                                            move || Box::new(empty_glyph.clone())
                                        },
                                    )),
                            )
                        }
                    });

                Box::new(
                    Pressable::new(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new(),
                        move || Box::new(slot.clone()),
                    ))
                    .disabled(readonly)
                    .on_click(move || {
                        // v1 fired the callback only when the rating was not
                        // read-only.
                        if readonly {
                            return;
                        }
                        for (other, star) in all.iter().enumerate() {
                            write_flag(&star.filled, other <= index);
                            write_flag(&star.empty, other > index);
                        }
                        if let Some(cb) = &on_change {
                            cb(value);
                        }
                    })
                    .label(format!("Rate {value} of {max}")),
                )
            });
        }

        row
    }
}

impl Default for FluentRating {
    fn default() -> Self {
        Self::new()
    }
}

