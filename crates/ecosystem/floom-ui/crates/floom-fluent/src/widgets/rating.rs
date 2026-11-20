//! Fluent rating, ported to the Floom 2.0 widget API.

use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::text_input::{shared_flag, write_flag, SharedFlag};
use floom_widgets::{Align, Axis, Conditional, Pressable, Stack, VectorIcon};

use crate::widgets::icon::{fluent_icon_path, VIEW_BOX};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::rating::RatingProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

const DESIGN_SYSTEM: &str = "fluent";

const STAR: &str = "star";

type Callback = Arc<dyn Fn(f32) + Send + Sync + 'static>;

#[derive(Debug, Clone)]
struct StarState {
    filled: SharedFlag,
    empty: SharedFlag,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentRatingStyle {
    pub design_system: &'static str,
    pub active_color: Color,
    pub inactive_color: Color,
    pub star_size: f32,
    pub gap: f32,
}

pub struct FluentRating {
    props: RatingProps,
    theme: RwSignal<MultiTheme>,
    stars: Vec<StarState>,
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

    pub fn value(mut self, value: f32) -> Self {
        self.props.value = value;
        self.reset_stars();
        self
    }

    pub fn max(mut self, max: usize) -> Self {
        self.props.max = max;
        self.reset_stars();
        self
    }

    pub fn readonly(mut self, readonly: bool) -> Self {
        self.props.readonly = readonly;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(f32) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// that as an `Arc` of its own (see the field's note), and `RatingProps` is
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

    pub fn filled_flags(&self) -> Vec<SharedFlag> {
        self.stars.iter().map(|star| star.filled.clone()).collect()
    }

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
            active_color: theme.colors().primary,
            inactive_color: theme.colors().outline_variant,
            star_size: tokens.star_size,
            gap: tokens.gap,
        }
    }

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
            .gap(style.gap)
            .align(Align::Start, Align::Center);

        for index in 0..max {
            let state = self.stars[index].clone();
            let all = self.stars.clone();
            let on_change = on_change.clone();
            let active = active;
            let inactive = inactive;
            let star_size = star_size;
            let path = path;
            let value = index as f32 + 1.0;

            row = row.push(move || {
                let all = all.clone();
                let on_change = on_change.clone();
                let filled_glyph = VectorIcon::from_path_data(path, VIEW_BOX)
                    .size(star_size)
                    .color(active);
                let empty_glyph = VectorIcon::from_path_data(path, VIEW_BOX)
                    .size(star_size)
                    .color(inactive);

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
