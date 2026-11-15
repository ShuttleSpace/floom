//! Cupertino timeline, ported to the Floom 2.0 widget API.
//!

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
pub struct CupertinoTimelineStyle {
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

/// Cupertino's timeline.
pub struct CupertinoTimeline {
    props: TimelineProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoTimeline {
    pub fn new() -> Self {
        Self {
            props: TimelineProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
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

    pub fn resolved_style(&self) -> CupertinoTimelineStyle {
        let theme = self.theme.get();
        let ComponentTokens::Timeline(tokens) = theme.component(ComponentId::Timeline) else {
            unreachable!("ComponentId::Timeline must resolve to TimelineTokens")
        };
        CupertinoTimelineStyle {
            design_system: "cupertino",
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

impl Default for CupertinoTimeline {
    fn default() -> Self {
        Self::new()
    }
}
