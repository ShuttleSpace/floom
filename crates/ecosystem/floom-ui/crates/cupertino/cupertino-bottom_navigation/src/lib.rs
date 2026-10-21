//! Cupertino bottom navigation, ported to the Floom 2.0 widget API.
//!
//! A bottom navigation is a bar across the bottom of the window with a row of
//! destinations in it: an icon above a label, one of them chosen.
//!
//! ## What the port uses
//!
//! The engine's `BottomNavigationBar`: a row of `Pressable` destinations, each
//! a vertical `Stack` of an optional icon and a label, inside a bar that
//! paints its own fill and the rule along its top edge.
//!
//! The *choice* is shared state rather than a signal: an `Arc<AtomicUsize>`,
//! the shape the engine's own `Tabs` and `NavigationBar` use, because a value
//! that changes after mount cannot be held in a signal and read during render.
//!
//! ## What this kit answers
//!
//! * the bar's fill: `surface` at 235; the rule along its top edge: `on_surface`
//!   at 30;
//! * the chosen destination's ink: `primary`; the other destinations':
//!   `on_surface_variant`;
//! * the fill under a destination the pointer is over: `on_surface` at 12 --
//!   and only while that destination is *not* the chosen one;
//! * the bar's height, the destination's minimum width, the icon's size and the
//!   label's size: the tokens;
//! * the gap between the icon and the label: the token's `gap`;
//! * the rule's thickness: **half a point**, a v1 literal;
//! * the gap between the icon and the label's top edge: **4 points**, a v1
//!   literal (`margin_top(4.0)`).
//!
//! ## The engine's bottom navigation
//!
//! The shape is shared by all three kits, so it belongs in `floom-ui-composites`
//! beside the `SelectGroup` -- and it is there now:
//! `floom_ui_composites::BottomNavigationBar` is a bar of destinations, one of
//! them chosen, and the three kits use it. What the kits did *not* agree on
//! -- whether the icon is sized, whether a chosen destination carries a rule,
//! whether a destination fills itself under the pointer, and whether it asks
//! for a minimum width -- are optional fields in the engine's style, because
//! they were v1's differences between the kits and not a theme's answers.
//!
//! Two things the engine *does* have were considered, and did not fit:
//!
//! * **The `SelectGroup`.** It is the engine's "one of several, one chosen",
//!   and its `row_with_state` gives a row's content the answer to "is this row
//!   chosen?" -- which is exactly the label's ink. But a group is a *stack of
//!   rows*: there is no bar behind them to paint, its rows do not grow to
//!   share a bar's width, and its choice lives inside the group, so a caller
//!   holding only the bar could not read it the way v1's `selected_signal()`
//!   let them. The kit holds the bar and the rows instead.
//! * **A shared integer.** The engine's shared types are a bool, a string and
//!   a map; the choice is an index, so the bar holds an `Arc<AtomicUsize>` --
//!   the shape the engine's own `Tabs` and `NavigationBar` use.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **`cursor(CursorStyle::Pointer)`.** 2.0 asks the node *under* the pointer
//!   for its shape, and the node under the pointer is the destination's label
//!   or its icon rather than the destination, so the hand is not carried over.
//!   The destination is still a `Pressable`, so it is still activatable by
//!   keyboard, which the pointer was not.
//! * **The icons were `AnyView`s.** 2.0's slots are replayed `Fn`s and a
//!   widget is mounted by its parent rather than returned, so the caller hands
//!   over a builder per icon instead of a built view.
//!
//! ## The tokens v1's style did not report
//!
//! * **`gap`.** v1's paint used the token's gap between a destination's icon
//!   and its label, and v1's style did not say so. The port reports it.
//! * **`divider_width`.** v1 wrote `.border_top(0.5)` -- a literal, not a
//!   token. The port reports it, because a style that does not say what the
//!   widget draws is not the widget's output.
//!
//! ## The one thing v1's style did not have
//!
//! `design_system`. The other two kits' styles name theirs and this one's did
//! not; the port gives all three the same field, so a caller can read which
//! design system a style came from.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::BottomNavigationStyle;
pub use floom_ui_composites::BottomNavigationBar;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::bottom_navigation::BottomNavigationProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

/// A builder for one destination's icon.
///
/// v1 took `Vec<AnyView>`: a built view per destination. 2.0's slots are
/// replayed `Fn`s and a widget is mounted by its parent, so the caller hands
/// over a builder rather than a widget.
pub use floom_ui_composites::IconBuilder;

/// A callback in the shape 2.0's slots need.
///
/// `Send + Sync` is required, not incidental: 2.0 rebuilds a widget inside a
/// `Child` closure the host may hand between threads, so the callback is
/// shared as an `Arc`.
type Callback = Arc<dyn Fn(usize) + Send + Sync + 'static>;

/// The rule along the bar's top edge. A v1 literal.
const DIVIDER_WIDTH: f32 = 0.5;

/// The gap between a destination's icon and the top of its label. A v1
/// literal: v1 wrote `margin_top(4.0)` on the label.
const LABEL_MARGIN_TOP: f32 = 4.0;

/// The bottom navigation's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoBottomNavigationStyle {
    pub design_system: &'static str,
    /// The bar's fill.
    pub background: Color,
    /// The chosen destination's ink.
    pub active_color: Color,
    /// The other destinations' ink.
    pub inactive_color: Color,
    /// The rule along the bar's top edge.
    pub divider_color: Color,
    /// The rule's thickness. A v1 literal.
    pub divider_width: f32,
    pub height: f32,
    pub item_min_width: f32,
    pub icon_size: f32,
    pub label_font_size: f32,
    /// The gap between a destination's icon and its label. v1's style did not
    /// report it.
    pub gap: f32,
    /// The fill under a destination the pointer is over. v1's style did not
    /// report it.
    pub hover_color: Color,
}

/// Cupertino's bottom navigation.
pub struct CupertinoBottomNavigation {
    props: BottomNavigationProps,
    /// The icons, one per destination, in the same order as the labels.
    icons: Vec<IconBuilder>,
    theme: RwSignal<MultiTheme>,
    /// The chosen destination.
    selection: Arc<AtomicUsize>,
    /// The change callback in the shape 2.0's slots need.
    ///
    /// `BottomNavigationProps::on_change` stays empty: it is a
    /// `Box<dyn Fn(usize)>`, which cannot cross a slot boundary.
    on_change: Option<Callback>,
}

impl CupertinoBottomNavigation {
    pub fn new() -> Self {
        let props = BottomNavigationProps::default();
        // The props are moved into the kit below, so the selection they carry
        // is read first.
        let selected = props.selected;
        Self {
            props,
            icons: Vec::new(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            selection: Arc::new(AtomicUsize::new(selected)),
            on_change: None,
        }
    }

    /// The destinations' labels, in order.
    pub fn labels(mut self, labels: Vec<String>) -> Self {
        self.props.labels = labels;
        self
    }

    /// The destination chosen before mount.
    pub fn selected(mut self, selected: usize) -> Self {
        self.props.selected = selected;
        self.selection.store(selected, Ordering::SeqCst);
        self
    }

    /// Called with the chosen destination's index whenever the choice changes.
    pub fn on_change(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// The icons, one per destination, in the same order as the labels.
    pub fn icons(mut self, icons: Vec<IconBuilder>) -> Self {
        self.icons = icons;
        self
    }

    pub fn props(&self) -> &BottomNavigationProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// The shared choice, so a caller can read it and drive it.
    ///
    /// v1's `selected_signal()` was an `RwSignal<usize>`; 2.0's shared state is
    /// an `Arc<AtomicUsize>`, the shape the engine's own `Tabs` uses.
    pub fn selection(&self) -> Arc<AtomicUsize> {
        self.selection.clone()
    }

    pub fn resolved_style(&self) -> CupertinoBottomNavigationStyle {
        let theme = self.theme.get();
        let ComponentTokens::BottomNavigation(tokens) = theme.component(ComponentId::BottomNavigation)
        else {
            unreachable!("ComponentId::BottomNavigation must resolve to BottomNavigationTokens")
        };
        CupertinoBottomNavigationStyle {
            design_system: "cupertino",
            // v1's bar: `with_alpha(surface, 235)`.
            background: with_alpha(theme.colors().surface, 235),
            active_color: theme.colors().primary,
            inactive_color: theme.colors().on_surface_variant,
            // v1's bar: `with_alpha(on_surface, 30)`.
            divider_color: with_alpha(theme.colors().on_surface, 30),
            divider_width: DIVIDER_WIDTH,
            height: tokens.height,
            item_min_width: tokens.item_min_width,
            icon_size: tokens.icon_size,
            label_font_size: tokens.label_font_size,
            gap: tokens.gap,
            // v1's hovered destination: `with_alpha(on_surface, 12)`.
            hover_color: with_alpha(theme.colors().on_surface, 12),
        }
    }

    /// The bar: the destinations, the shared choice, and the bar's own box.
    pub fn view(self) -> BottomNavigationBar {
        let style = self.resolved_style();
        let mut bar = BottomNavigationBar::new()
            .style(BottomNavigationStyle {
                bar_color: to_paint(style.background),
                selected_color: to_paint(style.active_color),
                unselected_color: to_paint(style.inactive_color),
                divider_color: to_paint(style.divider_color),
                divider_width: style.divider_width,
                height: style.height,
                item_min_width: Some(style.item_min_width),
                icon_size: Some(style.icon_size),
                label_font_size: style.label_font_size,
                gap: style.gap,
                label_margin_top: LABEL_MARGIN_TOP,
                destination_padding_vert: 0.0,
                indicator: None,
                hover_color: Some(to_paint(style.hover_color)),
            })
            .labels(self.props.labels)
            .icons(self.icons)
            .shared_selection(self.selection);
        if let Some(on_change) = self.on_change {
            bar = bar.on_change(move |index| on_change(index));
        }
        bar
    }
}

impl Default for CupertinoBottomNavigation {
    fn default() -> Self {
        Self::new()
    }
}

/// The bar: a row of destinations and the box they sit in.
///
/// The engine has no bottom navigation, so the kit holds one -- see the module
/// documentation.

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
