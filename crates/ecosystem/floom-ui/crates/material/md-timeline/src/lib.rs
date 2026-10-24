//! Material timeline, ported to the Floom 2.0 widget API.
//!
//! A timeline is a column of entries, each a dot with a line running down from
//! it and a title and a description beside it.
//!
//! ## What the port uses
//!
//! The engine's vocabulary: a vertical `Stack` of rows, each row a horizontal
//! `Stack` of a dot, a line, and the text. There is nothing here that a widget
//! could own and the vocabulary does not already say.
//!
//! ## What this kit answers
//!
//! * the dot's size and the line's thickness: the tokens;
//! * the dot's colour: `primary`; the line's: `outline_variant`;
//! * the padding around the column, and the gap between the dot column and
//!   the text: the token's `padding` -- v1 used the same number for both.
//!
//! ## The token that never showed
//!
//! v1's `item_gap` -- the vertical gap between entries -- was written on a
//! container that held *one* child, so it had nothing to separate. The entries
//! are stacked with no gap between them here too, and the style still reports
//! the number, because v1's style did.
//!
//! ## The numbers v1 wrote as literals
//!
//! The title's size (15) and weight (semi-bold), the description's size (13),
//! the gap between them (2) and the dot's radius (9999, a circle).
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The accessors.** v1's Material timeline had no `props()`, `theme()` or
//!   `with_theme()` -- the other two kits had all three. The port gives all
//!   three kits the same three, because a kit that cannot be given a theme
//!   cannot be tested against one.

use floom_core::LayoutRequest;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::{FontWeight, Insets};
use floom_widgets::{Align, Axis, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::timeline::{TimelineItem, TimelineProps};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

/// The title's size. A v1 literal.
const TITLE_FONT_SIZE: f32 = 15.0;
/// The description's size. A v1 literal.
const DESCRIPTION_FONT_SIZE: f32 = 13.0;
/// The gap between the title and the description. A v1 literal.
const TITLE_GAP: f32 = 2.0;
/// The dot's radius: 9999 is a circle whatever the size.
const DOT_RADIUS: f32 = 9999.0;

/// The timeline's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialTimelineStyle {
    pub design_system: &'static str,
    /// The dot's colour.
    pub dot_color: Color,
    /// The line's colour.
    pub line_color: Color,
    pub dot_size: f32,
    pub line_width: f32,
    /// The vertical gap between entries. v1 wrote it on a container with one
    /// child, so it never showed.
    pub item_gap: f32,
    pub padding: f32,
}

/// Material's timeline.
pub struct MaterialTimeline {
    props: TimelineProps,
    theme: RwSignal<MultiTheme>,
}

impl MaterialTimeline {
    pub fn new() -> Self {
        Self {
            props: TimelineProps::default(),
            theme: theme_signal(),
        }
    }

    /// Append an entry.
    pub fn item(mut self, title: impl Into<String>, description: impl Into<String>) -> Self {
        self.props.items.push(TimelineItem {
            title: title.into(),
            description: description.into(),
        });
        self
    }

    /// Replace the entries.
    pub fn items(mut self, items: Vec<TimelineItem>) -> Self {
        self.props.items = items;
        self
    }

    pub fn props(&self) -> &TimelineProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MaterialTimelineStyle {
        let theme = self.theme.get();
        let ComponentTokens::Timeline(tokens) = theme.component(ComponentId::Timeline) else {
            unreachable!("ComponentId::Timeline must resolve to TimelineTokens")
        };
        MaterialTimelineStyle {
            design_system: "material",
            dot_color: theme.colors().primary,
            line_color: theme.colors().outline_variant,
            dot_size: tokens.dot_size,
            line_width: tokens.line_width,
            item_gap: tokens.item_gap,
            padding: tokens.padding,
        }
    }

    /// The timeline: the column of entries.
    pub fn view(self) -> Stack {
        let style = self.resolved_style();
        let count = self.props.items.len();
        let mut column = Stack::new(Axis::Vertical)
            // v1's container wrote `gap(item_gap)`, and it held one child: the
            // entries are stacked with no gap between them.
            .request(
                LayoutRequest::new()
                    .padding(Insets {
                        left: f64::from(style.padding),
                        right: f64::from(style.padding),
                        top: f64::from(style.padding),
                        bottom: f64::from(style.padding),
                    })
                    // v1's `.items_start()`.
                    .cross_align(Align::Start),
            );

        for (index, item) in self.props.items.iter().enumerate() {
            let is_last = index + 1 == count;
            // The last entry has no line under it: v1 gave it a width of zero.
            let line_width = if is_last { 0.0 } else { style.line_width };
            let dot_size = style.dot_size;
            let dot_color = to_paint(style.dot_color);
            let line_color = to_paint(style.line_color);
            let padding = style.padding;
            let title = item.title.clone();
            let description = item.description.clone();
            let title_color = to_paint(self.theme.get().colors().on_surface);
            let description_color = to_paint(self.theme.get().colors().on_surface_variant);

            column = column.push(move || {
                // The dot, and the line running down from it.
                let dot = Surface::new()
                    .request(LayoutRequest::new().width(dot_size).height(dot_size))
                    .background(dot_color)
                    .radius(DOT_RADIUS);
                let line = Surface::new()
                    .request(LayoutRequest::new().width(line_width).grow(1.0))
                    .background(line_color);
                // The slot is a replayed `Fn`, so each box it holds gets a copy
                // of its own rather than a borrow of the one outside.
                let left = Stack::new(Axis::Vertical)
                    // v1's `.items_center()`.
                    .align(Align::Start, Align::Center)
                    .push({
                        let dot = dot.clone();
                        move || Box::new(dot.clone())
                    })
                    .push({
                        let line = line.clone();
                        move || Box::new(line.clone())
                    });

                // The title, and the description under it.
                let right = Stack::new(Axis::Vertical)
                    .gap(TITLE_GAP)
                    .push({
                        let title = title.clone();
                        move || {
                            Box::new(
                                Text::new(title.clone())
                                    .size(TITLE_FONT_SIZE)
                                    .weight(FontWeight::SemiBold)
                                    .color(title_color),
                            )
                        }
                    })
                    .push({
                        let description = description.clone();
                        move || {
                            Box::new(
                                Text::new(description.clone())
                                    .size(DESCRIPTION_FONT_SIZE)
                                    .color(description_color),
                            )
                        }
                    });

                Box::new(
                    Stack::new(Axis::Horizontal)
                        // v1's row used the padding as its gap.
                        .gap(padding)
                        // v1's `.items_start()`.
                        .align(Align::Start, Align::Start)
                        .push({
                            let left = left.clone();
                            move || Box::new(left.clone())
                        })
                        .push({
                            let right = right.clone();
                            move || Box::new(right.clone())
                        }),
                )
            });
        }

        column
    }
}

impl Default for MaterialTimeline {
    fn default() -> Self {
        Self::new()
    }
}
