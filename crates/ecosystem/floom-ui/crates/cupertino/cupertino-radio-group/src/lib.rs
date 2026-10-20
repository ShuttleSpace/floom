//! Cupertino radio group, ported to the Floom 2.0 widget API.
//!
//! A radio group is a column of options, each a ring with a dot in it when it
//! is chosen, and the label beside it.
//!
//! ## What the engine grew
//!
//! **A state-aware row.** The engine's `SelectGroup` is a column of rows that
//! share one choice, and `row_with_state` gives a row's content the answer to
//! "is *this* row chosen?" -- which is what a radio needs, because the ring's
//! colour and the dot inside it both depend on it. The engine also grew a
//! `disabled` flag on the group, because a whole group of options can be
//! turned off and the gate belongs on the row's own `Pressable`, where it
//! covers the keyboard as well as the pointer.
//!
//! ## What this kit answers
//!
//! * the ring's size, the row's height and the gap between rows: the tokens;
//! * the ring's outline, the dot inside it, and the label's ink and weight:
//!   this kit's own.
//!
//! ## What the three kits disagree about
//!
//! | | Material | Cupertino | Fluent |
//! | --- | --- | --- | --- |
//! | the ring's outline | `primary` when chosen | `outline` always | `primary` when chosen |
//! | the dot's size | half the ring | 0.45 of the ring | half the ring |
//! | the ring's radius | 9999, a literal | the theme's `full` | the theme's `full` |
//! | the label's ink | the default | the default | `primary` when chosen |
//! | the label's weight | the default | the default | semi-bold when chosen |
//! | the gap in a row | 8, a literal | the token's gap | the token's gap |
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **`cursor(CursorStyle::Pointer)`.** 2.0's paint context has no cursor
//!   vocabulary, so the pointer is not carried over; the row is still a
//!   `Pressable`, so it is still activatable by keyboard, which the pointer was
//!   not.
//! * **The label's default ink.** v1's Material and Cupertino labels took the
//!   framework's default ink rather than a theme colour, so the port's do too.

use std::sync::Arc;

use floom_core::LayoutRequest;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::{ContentPlacement, SelectGroup, SelectGroupStyle, SelectionMode};
use floom_widgets::{Align, Axis, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::radio_group::RadioGroupProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;


/// The ring's outline thickness. A v1 literal, in all three kits.
const RING_BORDER: f32 = 2.0;

/// A callback in the shape 2.0's slots need.
type Callback = Arc<dyn Fn(usize) + Send + Sync + 'static>;

/// The radio group's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoRadioGroupStyle {
    pub design_system: &'static str,
    /// v1's style named a `selected_color` and a `label_color`; neither was
    /// used by v1's paint, and both are kept here because the style is the
    /// record of what v1 said.
    pub selected_color: Color,
    /// The ring's outline: this kit's ring is the same colour whether the
    /// option is chosen or not.
    pub ring_color: Color,
    /// The label's ink. v1 never used it.
    pub label_color: Color,
    /// The ring's corner radius.
    pub radius: f32,
    pub radio_size: f32,
    pub item_height: f32,
    pub gap: f32,
}

/// Cupertino's radio group.
pub struct CupertinoRadioGroup {
    props: RadioGroupProps,
    theme: RwSignal<MultiTheme>,
    /// The change callback in the shape 2.0's slots need.
    ///
    /// v1's props held a `Box<dyn Fn(usize)>`, which cannot cross a slot
    /// boundary: 2.0's slots are shared between threads.
    on_change: Option<Callback>,
}

impl CupertinoRadioGroup {
    pub fn new() -> Self {
        Self {
            props: RadioGroupProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            on_change: None,
        }
    }

    /// Replace the options.
    pub fn options(mut self, options: Vec<String>) -> Self {
        self.props.options = options;
        self
    }

    /// Append an option.
    pub fn option(mut self, option: impl Into<String>) -> Self {
        self.props.options.push(option.into());
        self
    }

    /// The option chosen before mount.
    pub fn selected(mut self, selected: usize) -> Self {
        self.props.selected = Some(selected);
        self
    }

    /// Whether every option is turned off.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    /// Called with the chosen option's index whenever the choice changes.
    pub fn on_change(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        // Stored twice, from one closure: the props keep v1's `Box<dyn Fn(usize)>`
        // and the engine's slot gets the shareable one.
        let shared: Callback = Arc::new(callback);
        self.props.on_change = Some(Box::new({
            let shared = shared.clone();
            move |index| shared(index)
        }));
        self.on_change = Some(shared);
        self
    }

    pub fn props(&self) -> &RadioGroupProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoRadioGroupStyle {
        let theme = self.theme.get();
        let ComponentTokens::RadioGroup(tokens) = theme.component(ComponentId::RadioGroup) else {
            unreachable!("ComponentId::RadioGroup must resolve to RadioGroupTokens")
        };
        CupertinoRadioGroupStyle {
            design_system: "cupertino",
            selected_color: theme.colors().primary,
            ring_color: theme.colors().outline,
            label_color: theme.colors().on_surface,
            radius: theme.radius().full,
            radio_size: tokens.radio_size,
            item_height: tokens.item_height,
            gap: tokens.gap,
        }
    }

    /// The group: the options, and the one choice they share.
    pub fn view(self) -> SelectGroup {
        let style = self.resolved_style();
        let selected = self.props.selected;
        let on_change = self.on_change.clone();
        let mut group = SelectGroup::new()
            .mode(SelectionMode::Single)
            .disabled(self.props.disabled)
            .style(SelectGroupStyle {
                gap: style.gap,
                axis: Axis::Vertical,
                height: Some(style.item_height),
                // v1's row had no padding of its own: the ring and the label
                // carry their own spacing.
                pad: 0.0,
                // v1's row: `items_center()`.
                content_placement: ContentPlacement::Centred,
                // The row itself has no fill and no outline: the ring is the
                // outline.
                background: PaintColor::rgba(0, 0, 0, 0),
                foreground: PaintColor::rgba(0, 0, 0, 0),
                border_width: 0.0,
                border_color: PaintColor::rgba(0, 0, 0, 0),
                // The row has no corner of its own: the ring is the outline.
                radius: 0.0,
                selected_background: PaintColor::rgba(0, 0, 0, 0),
                selected_foreground: PaintColor::rgba(0, 0, 0, 0),
                selected_border_color: PaintColor::rgba(0, 0, 0, 0),
                // A radio keeps its choice: tapping the chosen option leaves it
                // chosen, which is the engine's single-mode rule.
                toggles_off_in_single_mode: false,
                mandatory_in_multiple_mode: false,
            });

        for label in self.props.options {
            // The style is `Copy` and the label is cloned: the row's content
            // builder is a replayed `Fn`, so it has to own what it reads.
            let style = style;
            let label = label.clone();
            group = group.row_with_state(label.clone(), move |chosen| {
                // The ring: an outline, with the dot inside it when this option
                // is the chosen one.
                let dot = Surface::new()
                    .request(
                        LayoutRequest::new()
                            .width(style.radio_size * 0.45)
                            .height(style.radio_size * 0.45),
                    )
                    .background(to_paint(style.selected_color))
                    .radius(style.radius);
                let ring = Surface::new()
                    .request(
                        LayoutRequest::new()
                            .width(style.radio_size)
                            .height(style.radio_size)
                            // v1's ring: `items_center().justify_center()`.
                            .main_align(Align::Center)
                            .cross_align(Align::Center),
                    )
                    .radius(style.radius)
                    .border(RING_BORDER, to_paint(style.ring_color))
                    .push({
                        let dot = dot.clone();
                        move || Box::new(dot.clone())
                    });

                let row = Stack::new(Axis::Horizontal)
                    // v1's row gap: the token's gap
                    .gap(style.gap)
                    .push({
                        let ring = ring.clone();
                        move || Box::new(ring.clone())
                    })
                    .push({
                        let label = label.clone();
                        move || Box::new(Text::new(label.clone()))
                    });

                Box::new(row)
            });
        }

        if let Some(on_change) = on_change {
            group = group.on_change(move |chosen| {
                if let Some(index) = chosen.last() {
                    on_change(*index);
                }
            });
        }

        // v1 seeded the choice from the props, and the engine's group starts
        // with nothing chosen.
        if let Some(index) = selected {
            group = group.selected([index]);
        }

        group
    }
}

impl Default for CupertinoRadioGroup {
    fn default() -> Self {
        Self::new()
    }
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
