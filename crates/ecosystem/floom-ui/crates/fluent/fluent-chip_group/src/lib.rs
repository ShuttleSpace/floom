//! Fluent chip group, ported to the Floom 2.0 widget API.
//!
//! A chip group is a *row* of chips, any number of which can be chosen.
//!
//! ## What the engine grew for this
//!
//! v1's chip group and v1's item group look alike and are not: a chip group is
//! a horizontal row of pills whose label is centred, whose ink follows the
//! choice, and whose chosen chip can be tapped off. The engine's
//! [`SelectGroup`] was a vertical list of radio-ish rows, so four answers had
//! to become the caller's rather than the engine's:
//!
//! * **The direction.** `SelectGroupStyle::axis` -- a row of chips runs the
//!   other way from a list of items.
//! * **A row's own state.** v1 coloured a chosen chip's **ink** as well as its
//!   fill, and 2.0's `Surface` paints a fill while only the child knows it is
//!   text. `row_with_state` is the door: the content is built knowing whether
//!   *its* row is chosen.
//! * **Where the content sits.** `SelectGroupStyle::content_placement` -- v1
//!   centred the label in the chip (`items_center`), and a list of items does
//!   not.
//! * **The rules.** `toggles_off_in_single_mode` and
//!   `mandatory_in_multiple_mode` -- v1's chip group read `mandatory` only in
//!   the single-mode branch, and a chip can be tapped off where a radio cannot.
//!
//! ## What the port reproduces
//!
//! * the chip: 32 tall, 12 of padding at the sides, a 16 radius, 13pt text;
//! * the palette: `surface_variant` with `on_surface` ink, and
//!   `secondary_container` with `on_secondary_container` when chosen;
//! * the gap between chips, from the token;
//! * the click rules: several at once when `multiple`, one at a time
//!   otherwise -- and the chosen one comes off when it is tapped again, unless
//!   it is `mandatory`.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **`items_center()` on the stack.** The chips were centred against each
//!   other in the stack's cross axis. Every chip is 32 tall, so centring them
//!   against each other never showed; 2.0's stack stretches its children
//!   instead, and a child with a height of its own is not stretched.
//! * **`gap(6.0)` inside the chip.** v1's chip was a container with one child,
//!   so the gap between its children never had anything to separate.
//! * **`justify_center()`.** The chip is as wide as its label plus the padding,
//!   so the label already fills the content box.
//! * **`cursor(CursorStyle::Pointer)`.** 2.0's paint context has no cursor
//!   vocabulary, so the pointer is not carried over; the chip is still a
//!   `Pressable`, so it is still activatable by keyboard, which the pointer was
//!   not.
//!
//! ## The one difference that is left
//!
//! v1 reported the selection after **every** click, including a click that
//! changed nothing -- tapping the chosen chip of a `mandatory` group reported
//! the same indices again. The engine reports a *change*, so that click
//! reports nothing. The difference shows only in that one case.
//!
//! ## The numbers that are not tokens
//!
//! v1 wrote the chip's height (32), its padding (12), its radius (16) and its
//! text size (13) as literals, and read the token only for the gap between
//! chips. They are literals here too.

use std::sync::Arc;

use floom_core::Axis;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::{ContentPlacement, SelectGroup, SelectGroupStyle, SelectionMode};
use floom_widgets::Text;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::chip_group::ChipGroupProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;


/// A callback in the shape 2.0's slots need.
type Callback = Arc<dyn Fn(Vec<usize>) + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentChipGroupStyle {
    pub design_system: &'static str,
    /// The chip's fill, unchosen.
    pub background: Color,
    /// The chip's ink, unchosen.
    pub foreground: Color,
    /// The chip's fill, chosen.
    pub selected_background: Color,
    /// The chip's ink, chosen.
    pub selected_foreground: Color,
    /// The gap between chips, from the token.
    pub gap: f32,
    /// The chip's height. A v1 literal.
    pub height: f32,
    /// The chip's padding at the sides. A v1 literal.
    pub padding_horizontal: f32,
    /// The chip's corner radius. A v1 literal.
    pub border_radius: f32,
    /// The chip's text size. A v1 literal.
    pub font_size: f32,
}

/// Fluent's chip group.
pub struct FluentChipGroup {
    props: ChipGroupProps,
    labels: Vec<String>,
    theme: RwSignal<MultiTheme>,
    /// The change callback in the shape 2.0's slots need.
    ///
    /// `Send + Sync` is required, not incidental: 2.0 rebuilds a widget inside a
    /// `Child` closure the host may hand between threads, so the callback is
    /// shared as an `Arc`. v1's callback was a `Box<dyn Fn(Vec<usize>)>` in the
    /// shared props, which cannot cross that boundary.
    on_change: Option<Callback>,
}

impl FluentChipGroup {
    pub fn new() -> Self {
        Self {
            props: ChipGroupProps::default(),
            labels: Vec::new(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            on_change: None,
        }
    }

    /// Append a chip.
    pub fn chip(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self
    }

    /// Several chips may be chosen at once.
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.props.multiple = multiple;
        self
    }

    /// At least one chip stays chosen.
    ///
    /// v1 read this only in the single-mode branch, so in multiple mode it did
    /// nothing. The engine is told the same thing: see
    /// [`Self::select_group_style`].
    pub fn mandatory(mut self, mandatory: bool) -> Self {
        self.props.mandatory = mandatory;
        self
    }

    /// Called with the chosen chips whenever the choice changes.
    pub fn on_change(mut self, callback: impl Fn(Vec<usize>) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// The shared props, for a caller that reads the design-system-neutral shape.
    ///
    /// `on_change` is deliberately absent from what it reports: the kit holds
    /// that as an `Arc` of its own (see the field's note), and `ChipGroupProps`
    /// is the v1 carrier of a `Box<dyn Fn(..)>` that 2.0 cannot use.
    pub fn props(&self) -> &ChipGroupProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentChipGroupStyle {
        let theme = self.theme.get();
        let ComponentTokens::ChipGroup(tokens) = theme.component(ComponentId::ChipGroup) else {
            unreachable!("ComponentId::ChipGroup must resolve to ChipGroupTokens")
        };
        FluentChipGroupStyle {
            design_system: "fluent",
            background: theme.colors().surface_variant,
            foreground: theme.colors().on_surface,
            selected_background: theme.colors().secondary_container,
            selected_foreground: theme.colors().on_secondary_container,
            gap: tokens.gap,
            height: 32.0,
            padding_horizontal: 12.0,
            border_radius: 16.0,
            font_size: 13.0,
        }
    }

    /// The engine's style for this group: the chip's shape and v1's rules.
    pub fn select_group_style(&self) -> SelectGroupStyle {
        let style = self.resolved_style();
        SelectGroupStyle {
            gap: style.gap,
            // A chip group is a *row* of chips.
            axis: Axis::Horizontal,
            // v1's chip had a height of its own: 32.
            height: Some(style.height),
            pad: style.padding_horizontal,
            // v1 centred the label in the chip.
            content_placement: ContentPlacement::Centred,
            // v1's chip came off when it was tapped again.
            toggles_off_in_single_mode: true,
            // v1 read `mandatory` only in the single-mode branch, so in multiple
            // mode it did nothing.
            mandatory_in_multiple_mode: false,
            radius: style.border_radius,
            // v1's chip had no border.
            border_width: 0.0,
            background: to_paint(style.background),
            foreground: to_paint(style.foreground),
            border_color: to_paint(style.foreground),
            selected_background: to_paint(style.selected_background),
            selected_foreground: to_paint(style.selected_foreground),
            selected_border_color: to_paint(style.selected_foreground),
        }
    }

    /// The group: a row of chips, the shared choice, and the click rules.
    pub fn view(self) -> SelectGroup {
        let style = self.resolved_style();
        let mut group = SelectGroup::new()
            .mode(if self.props.multiple {
                SelectionMode::Multiple
            } else {
                SelectionMode::Single
            })
            .mandatory(self.props.mandatory)
            .style(self.select_group_style());

        let ink = to_paint(style.foreground);
        let selected_ink = to_paint(style.selected_foreground);
        let font_size = style.font_size;
        for label in self.labels {
            let text = label.clone();
            group = group.row_with_state(label, move |selected| {
                // The content is built with the row's own state, so the ink
                // follows the choice.
                let color = if selected { selected_ink } else { ink };
                Box::new(Text::new(text.clone()).size(font_size).color(color))
            });
        }

        if let Some(on_change) = self.on_change.clone() {
            group = group.on_change(move |indices| on_change(indices));
        }

        group
    }
}

impl Default for FluentChipGroup {
    fn default() -> Self {
        Self::new()
    }
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
