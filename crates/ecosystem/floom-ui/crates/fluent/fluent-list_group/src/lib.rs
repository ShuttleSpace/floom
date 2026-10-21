//! Fluent list group, ported to the Floom 2.0 widget API.
//!
//! A list group is a titled, collapsible group of rows. v1 built it as a
//! clickable header over a stack of rows that appeared and disappeared with an
//! `RwSignal<bool>`.
//!
//! ## What the port uses
//!
//! The header-plus-body shape, the chevron that changes with the state, and the
//! body that is mounted only while the group is open -- that is the engine's
//! [`Accordion`], and one list group is one accordion with a single panel. The
//! engine owns the open/close state, so the kit does not need a signal of its
//! own, and the toggle callback is the accordion's `on_change`.
//!
//! ## Where the padding lives
//!
//! v1 padded the *items* -- each row took `indent` or `content_padding` of
//! horizontal padding -- and the stack around them took the vertical padding.
//! The accordion's own `content_padding` would put that inset on the body box
//! on *all four* sides, and v1 had no horizontal inset there, so the
//! accordion's is left at zero and the kit's own stack carries the vertical
//! one.
//!
//! ## What v1 drew that this port does not
//!
//! * **The outline around the header row.** v1 put a 1px outline on the
//!   *header* -- `border(1.0).border_color(with_alpha(outline, 30))` -- so the
//!   outline was the header row's own box. 2.0's accordion draws the outline
//!   around the *panel*: the header and, while it is open, the rows. The box
//!   that would have to carry the header's outline is a leaf, and a leaf
//!   cannot hold the row it would be outlining; the engine says as much, and
//!   outlining the panel is what Material 3's outlined expansion panel does.
//!   The width and the colour are v1's; the shape is the engine's.
//! * **`cursor(CursorStyle::Pointer)` on the header.** 2.0's paint context has
//!   no cursor vocabulary, so the pointer is not carried over; the header is
//!   still a `Pressable`, so it is still activatable by keyboard, which the
//!   pointer was not.
//!
//! ## The chevron
//!
//! The indicator is this design system's own glyph: `expand_more` and
//! `chevron_right` are different drawings in each icon table, and a lookup that
//! reached for another system's table would draw a foreign chevron.

use std::sync::Arc;

use floom_core::LayoutRequest;
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Insets};
use floom_ui_composites::{Accordion, AccordionStyle};
use floom_widgets::{Align, Axis, Stack, Text, VectorIcon};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, ListGroupTokens};
use floom_core_theme::props::list_group::ListGroupProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_fluent_icon::FluentIcon;

/// The header title's size, in logical pixels.
///
/// v1 drew the title with a plain `Label`, which takes the default text size;
/// the engine's own label size is 14, and the port names it rather than
/// leaving it implicit.
const LABEL_FONT_SIZE: f32 = 14.0;

/// A callback in the shape 2.0's slots need.
type Callback = Arc<dyn Fn(bool) + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentListGroupStyle {
    pub title_color: Color,
    /// The outline colour the header's border is drawn in.
    pub outline: Color,
    pub item_color: Color,
    pub indicator_color: Color,
    pub item_font_size: f32,
    pub header_height: f32,
    pub content_padding: f32,
    pub indent: f32,
    pub gap: f32,
    pub icon_size: f32,
    pub expanded: bool,
    pub inset: bool,
}

/// Fluent's list group.
pub struct FluentListGroup {
    props: ListGroupProps,
    items: Vec<String>,
    theme: RwSignal<MultiTheme>,
    /// The toggle callback in the shape 2.0's slots need. v1's props held a
    /// `Box<dyn Fn(bool)>`, which is not shareable.
    on_toggle: Option<Callback>,
}

impl FluentListGroup {
    pub fn new() -> Self {
        Self {
            props: ListGroupProps::default(),
            items: Vec::new(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            on_toggle: None,
        }
    }

    /// Add a row to the group.
    pub fn item(mut self, label: impl Into<String>) -> Self {
        self.items.push(label.into());
        self
    }

    /// The group header's text.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = Some(title.into());
        self
    }

    /// Whether the group starts open.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.props.expanded = expanded;
        self
    }

    /// Whether the rows are indented (sub-group styling).
    pub fn inset(mut self, inset: bool) -> Self {
        self.props.inset = inset;
        self
    }

    /// Called with the new expanded state whenever the header is toggled.
    pub fn on_toggle(mut self, callback: impl Fn(bool) + Send + Sync + 'static) -> Self {
        // Stored twice, from one closure: a v1 `Box<dyn Fn(bool)>` for the props
        // and an `Arc<dyn Fn(bool) + Send + Sync>` for the engine's slot.
        let shared: Callback = Arc::new(callback);
        self.props.on_toggle = Some(Box::new({
            let shared = shared.clone();
            move |open| shared(open)
        }));
        self.on_toggle = Some(shared);
        self
    }

    pub fn props(&self) -> &ListGroupProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentListGroupStyle {
        let theme = self.theme.get();
        let ComponentTokens::ListGroup(tokens) = theme.component(ComponentId::ListGroup) else {
            unreachable!("ComponentId::ListGroup must resolve to ListGroupTokens")
        };
        FluentListGroupStyle {
            // v1's header title was a plain `Label`, which takes the default ink.
            title_color: theme.colors().on_surface,
            // v1's header border: `border_color(with_alpha(outline, 30))`.
            outline: theme.colors().outline,
            item_color: theme.colors().on_surface,
            // v1's indicator: `color(t.colors().on_surface_variant)`.
            indicator_color: theme.colors().on_surface_variant,
            item_font_size: theme.typography().body.medium.font_size,
            header_height: tokens.header_height,
            content_padding: tokens.content_padding,
            indent: tokens.indent,
            gap: tokens.gap,
            icon_size: tokens.icon_size,
            expanded: self.props.expanded,
            inset: self.props.inset,
        }
    }

    /// The two indicator glyphs, at the token's size and the muted ink.
    ///
    /// The collapsed one and the expanded one, in that order. Public because a
    /// caller that draws its own header wants the same glyphs.
    pub fn indicator(&self) -> (VectorIcon, VectorIcon) {
        let style = self.resolved_style();
        let color = to_paint(style.indicator_color);
        (
            FluentIcon::new("chevron_right")
                .resolve()
                .size(style.icon_size)
                .color(color),
            FluentIcon::new("expand_more")
                .resolve()
                .size(style.icon_size)
                .color(color),
        )
    }

    /// The group: a header that toggles, and the rows it reveals.
    pub fn view(self) -> Accordion {
        let style = self.resolved_style();
        let title = self.props.title.clone().unwrap_or_default();
        let items = self.items.clone();
        let inset = self.props.inset;
        let on_toggle = self.on_toggle.clone();
        let (collapsed, expanded_icon) = self.indicator();

        let item_color = to_paint(style.item_color);
        let item_font_size = style.item_font_size;
        let item_height = style.header_height;
        let row_gap = style.gap;
        let vertical_padding = style.content_padding;
        let row_padding = if inset { style.indent } else { style.content_padding };

        let mut accordion = Accordion::new()
            .style(AccordionStyle {
                header_height: style.header_height,
                // v1's header: `padding_horiz(content_padding)`.
                header_padding: style.content_padding,
                // The padding belongs to the rows, as it did in v1.
                content_padding: 0.0,
                content_indent: 0.0,
                gap: 0.0,
                title_color: to_paint(style.title_color),
                title_size: LABEL_FONT_SIZE,
                // v1's header: `border(1.0).border_color(with_alpha(outline, 30))`.
                border_width: 1.0,
                border_color: to_paint(with_alpha(style.outline, 30)),
                radius: 0.0,
            })
            .panel(title, move |open| {
                let icon = if open { expanded_icon.clone() } else { collapsed.clone() };
                Box::new(icon)
            }, move || {
                // v1's items: a vertical stack, each row padded, with the stack
                // itself padded vertically.
                let mut stack = Stack::new(Axis::Vertical)
                    .gap(row_gap)
                    // v1: `padding_vert(content_padding)` on the stack that
                    // holds the items. The accordion's own `content_padding`
                    // would inset the body box on *all four* sides, and v1 had
                    // no horizontal inset there -- the rows carried their own --
                    // so the vertical inset belongs here.
                    .request(
                        LayoutRequest::new().padding_vert(f64::from(vertical_padding)),
                    );
                for label in items.iter() {
                    let label = label.clone();
                    let color = item_color;
                    stack = stack.push(move || {
                        // The slot is a replayed `Fn`, so the label is cloned
                        // again for the inner slot that draws it.
                        let label = label.clone();
                        Box::new(
                            Stack::new(Axis::Horizontal)
                                .request(
                                    LayoutRequest::new()
                                        .height(item_height)
                                        .padding_horiz(f64::from(row_padding))
                                        .cross_align(Align::Center),
                                )
                                .push(move || {
                                    // The slot is a replayed `Fn`, so it borrows
                                    // the label rather than moving it out.
                                    Box::new(Text::new(label.clone()).size(item_font_size).color(color))
                                }),
                        )
                    });
                }
                Box::new(stack)
            });

        // v1 seeded the expanded flag from the props, and the engine's
        // accordion starts closed.
        accordion.set_open(0, self.props.expanded);

        if let Some(on_toggle) = on_toggle {
            accordion = accordion.on_change(move |_, open| on_toggle(open));
        }

        accordion
    }
}

impl Default for FluentListGroup {
    fn default() -> Self {
        Self::new()
    }
}

