//! Cupertino rating, ported to the Floom 2.0 widget API.
//!

use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::text_input::{shared_flag, write_flag, SharedFlag};
use floom_widgets::{Align, Axis, Conditional, Pressable, Stack, VectorIcon};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::rating::RatingProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use crate::widgets::icon::{cupertino_icon_path, VIEW_BOX};

const DESIGN_SYSTEM: &str = "cupertino";

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
pub struct CupertinoRatingStyle {
    pub design_system: &'static str,
    /// The filled star's ink. The system yellow, a v1 literal.
    pub active_color: Color,
    /// The empty star's ink.
    pub inactive_color: Color,
    pub star_size: f32,
    pub gap: f32,
}

/// Cupertino's rating.
pub struct CupertinoRating {
    props: RatingProps,
    theme: RwSignal<MultiTheme>,
    /// One entry per star: the flags that say which glyph it shows.
    stars: Vec<StarState>,
    /// The change callback in the shape 2.0's slots need.
    on_change: Option<Callback>,
}

impl CupertinoRating {
    pub fn new() -> Self {
        let mut kit = Self {
            props: RatingProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
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

    pub fn resolved_style(&self) -> CupertinoRatingStyle {
        let theme = self.theme.get();
        let ComponentTokens::Rating(tokens) = theme.component(ComponentId::Rating) else {
            unreachable!("ComponentId::Rating must resolve to RatingTokens")
        };
        CupertinoRatingStyle {
            design_system: DESIGN_SYSTEM,
            // v1's literal: the system yellow, not a theme colour.
            active_color: Color::rgb(255, 204, 0),
            inactive_color: theme.colors().outline,
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
        let path = cupertino_icon_path(STAR);
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
                let all = all.clone();
                let on_change = on_change.clone();
                // The two glyphs: the star filled, and the star empty. They are
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
                            Box::new(Conditional::new(false).when_flag(flag.clone()).show(
                                Child::new(ChildKey::index(0), LayoutRequest::new(), {
                                    let filled_glyph = filled_glyph.clone();
                                    move || Box::new(filled_glyph.clone())
                                }),
                            ))
                        }
                    })
                    .push({
                        let empty_glyph = empty_glyph.clone();
                        let flag = state.empty.clone();
                        move || {
                            Box::new(Conditional::new(false).when_flag(flag.clone()).show(
                                Child::new(ChildKey::index(0), LayoutRequest::new(), {
                                    let empty_glyph = empty_glyph.clone();
                                    move || Box::new(empty_glyph.clone())
                                }),
                            ))
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

impl Default for CupertinoRating {
    fn default() -> Self {
        Self::new()
    }
}
