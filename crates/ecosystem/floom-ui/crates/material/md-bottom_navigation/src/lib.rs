//! Material bottom navigation, ported to the Floom 2.0 widget API.
//!
//! A bottom navigation is a bar across the bottom of the window with a row of
//! destinations in it: an icon above a label, one of them chosen.
//!
//! ## What the port uses
//!
//! The engine's vocabulary: a row of `Pressable` destinations, each a vertical
//! `Stack` of an optional icon and a label, inside a bar that paints its own
//! fill and the rule along its top edge.
//!
//! The *choice* is shared state rather than a signal: an `Arc<AtomicUsize>`,
//! the shape the engine's own `Tabs` and `NavigationBar` use, because a value
//! that changes after mount cannot be held in a signal and read during render.
//!
//! ## What this kit answers
//!
//! * the bar's fill: `surface`; the rule along its top edge: `outline_variant`;
//! * the chosen destination's ink: `primary`; the other destinations':
//!   `on_surface_variant`;
//! * the bar's height, the destination's minimum width, the icon's size and the
//!   label's size: the tokens;
//! * the gap between the icon and the label: the token's `gap` -- v1 used that
//!   number *inside* a destination and nowhere else;
//! * the rule's thickness: **1 point**, a v1 literal.
//!
//! ## The engine gap
//!
//! **The engine has no bottom navigation.** The shape is shared by all three
//! kits, so it belongs in `floom-ui-composites` beside the `SelectGroup` -- and
//! it cannot be put there from here, so each kit holds its own copy and this
//! paragraph is the record of that. The engine should grow one.
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
//!   a map; the choice is an index, so the kit holds an `Arc<AtomicUsize>` --
//!   the shape the engine's own `Tabs` and `NavigationBar` use.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **`cursor(CursorStyle::Pointer)`.** 2.0 asks the node *under* the pointer
//!   for its shape, and the node under the pointer is the destination's label
//!   or its icon rather than the destination, so the hand is not carried over.
//!   The destination is still a `Pressable`, so it is still activatable by
//!   keyboard, which the pointer was not.
//! * **`width_full()` on each destination.** v1's Material destinations asked
//!   for the whole width of the bar *each*, which in a row of five would
//!   overflow it; the port grows them equally instead, the way v1's Cupertino
//!   and Fluent destinations did.
//! * **The icons were `AnyView`s.** 2.0's slots are replayed `Fn`s and a
//!   widget is mounted by its parent rather than returned, so the caller hands
//!   over a builder per icon instead of a built view.
//!
//! ## The tokens v1's style did not report
//!
//! * **`gap`.** v1's paint used the token's gap between a destination's icon
//!   and its label, and v1's style did not say so. The port reports it.
//! * **`divider_width`.** v1 wrote `.border_top(1.0)` -- a literal, not a
//!   token. The port reports it, because a style that does not say what the
//!   widget draws is not the widget's output.
//!
//! ## The token that never showed
//!
//! v1 read the icon's size into a variable it never used (`let _icon_size =
//! ...`): Material's destinations were not sized, so the icon was whatever the
//! caller built. The style still reports the number, because v1's style did,
//! and the port does not size the icon either.
//!
//! ## The one thing v1's style did not have
//!
//! `design_system`. The other two kits' styles name theirs and this one's did
//! not; the port gives all three the same field, so a caller can read which
//! design system a style came from.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{
    Align, Axis, Child, ChildKey, CreateContext, ElementId, LayoutRequest, PaintContext,
    SemanticsContext, UpdateContext, Widget, WidgetAny,
};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Rect, Size};
use floom_widgets::{Pressable, Stack, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::bottom_navigation::BottomNavigationProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

/// A builder for one destination's icon.
///
/// v1 took `Vec<AnyView>`: a built view per destination. 2.0's slots are
/// replayed `Fn`s and a widget is mounted by its parent, so the caller hands
/// over a builder rather than a widget.
pub type IconBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

/// A callback in the shape 2.0's slots need.
///
/// `Send + Sync` is required, not incidental: 2.0 rebuilds a widget inside a
/// `Child` closure the host may hand between threads, so the callback is
/// shared as an `Arc`.
type Callback = Arc<dyn Fn(usize) + Send + Sync + 'static>;

/// The rule along the bar's top edge. A v1 literal.
const DIVIDER_WIDTH: f32 = 1.0;

/// The key the destination's label is mounted under.
///
/// Keyed rather than positional, because the label is re-mounted when the
/// choice moves: a `Text` cannot change its ink in place, so the label is
/// rebuilt with the colour the choice calls for.
const LABEL_KEY: u64 = 0xBA0_0000_0000_0000;

/// The bottom navigation's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialBottomNavigationStyle {
    pub design_system: &'static str,
    /// The bar's fill.
    pub bar_color: Color,
    /// The chosen destination's ink.
    pub selected_color: Color,
    /// The other destinations' ink.
    pub unselected_color: Color,
    /// The rule along the bar's top edge.
    pub divider_color: Color,
    /// The rule's thickness. A v1 literal.
    pub divider_width: f32,
    pub height: f32,
    pub item_min_width: f32,
    /// The icon's size. v1's Material destinations were not sized, so this never
    /// showed.
    pub icon_size: f32,
    pub label_font_size: f32,
    /// The gap between a destination's icon and its label. v1's style did not
    /// report it.
    pub gap: f32,
}

/// Material's bottom navigation.
pub struct MaterialBottomNavigation {
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

impl MaterialBottomNavigation {
    pub fn new() -> Self {
        let props = BottomNavigationProps::default();
        // The props are moved into the kit below, so the selection they carry
        // is read first.
        let selected = props.selected;
        Self {
            props,
            icons: Vec::new(),
            theme: theme_signal(),
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

    pub fn resolved_style(&self) -> MaterialBottomNavigationStyle {
        let theme = self.theme.get();
        let ComponentTokens::BottomNavigation(tokens) = theme.component(ComponentId::BottomNavigation)
        else {
            unreachable!("ComponentId::BottomNavigation must resolve to BottomNavigationTokens")
        };
        MaterialBottomNavigationStyle {
            design_system: "material",
            bar_color: theme.colors().surface,
            selected_color: theme.colors().primary,
            unselected_color: theme.colors().on_surface_variant,
            divider_color: theme.colors().outline_variant,
            divider_width: DIVIDER_WIDTH,
            height: tokens.height,
            item_min_width: tokens.item_min_width,
            icon_size: tokens.icon_size,
            label_font_size: tokens.label_font_size,
            gap: tokens.gap,
        }
    }

    /// The bar: the destinations, the shared choice, and the bar's own box.
    pub fn view(self) -> MaterialBottomNavigationBar {
        let style = self.resolved_style();
        MaterialBottomNavigationBar {
            labels: self.props.labels,
            icons: self.icons,
            selection: self.selection,
            style,
            on_change: self.on_change,
        }
    }
}

impl Default for MaterialBottomNavigation {
    fn default() -> Self {
        Self::new()
    }
}

/// The bar: a row of destinations and the box they sit in.
///
/// The engine has no bottom navigation, so the kit holds one -- see the module
/// documentation.
pub struct MaterialBottomNavigationBar {
    labels: Vec<String>,
    icons: Vec<IconBuilder>,
    selection: Arc<AtomicUsize>,
    style: MaterialBottomNavigationStyle,
    on_change: Option<Callback>,
}

impl MaterialBottomNavigationBar {
    /// The chosen destination.
    pub fn selected(&self) -> usize {
        self.selection.load(Ordering::SeqCst)
    }

    /// Choose a destination, as if it had been tapped.
    ///
    /// The change callback is *not* run: v1's callback reported what a person
    /// did, and a caller that chooses a destination already knows.
    pub fn set_selected(&mut self, index: usize) {
        self.selection.store(index, Ordering::SeqCst);
    }

    /// The shared choice, so a caller can drive it from outside.
    pub fn selection(&self) -> Arc<AtomicUsize> {
        self.selection.clone()
    }
}

impl Widget for MaterialBottomNavigationBar {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    /// Deliberately empty.
    ///
    /// The bar's own box does not move when the choice does; the destination
    /// that answers to the choice is the one that has to notice, and it does.
    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {}

    fn children(&self) -> Vec<Child> {
        let mut out = Vec::with_capacity(self.labels.len());
        for (index, label) in self.labels.iter().enumerate() {
            let icon = self.icons.get(index).cloned();
            let selection = self.selection.clone();
            let on_change = self.on_change.clone();
            let style = self.style;
            let label = label.clone();
            out.push(Child::new(
                ChildKey::index(index),
                // v1's Cupertino destination grew to fill the bar; v1's Material
                // one asked for the whole width, which would have overflowed it.
                LayoutRequest::new()
                    .min_width(style.item_min_width)
                    .grow(1.0),
                move || {
                    // The slot is a replayed `Fn`, so everything it hands to a
                    // nested closure is copied out of it first.
                    let label = label.clone();
                    let icon = icon.clone();
                    let selection = selection.clone();
                    let on_change = on_change.clone();
                    let content = MaterialDestination {
                        index,
                        label: label.clone(),
                        font_size: style.label_font_size,
                        inner_gap: style.gap,
                        icon: icon.clone(),
                        // v1's Material destinations did not size the icon.
                        icon_size: None,
                        selection: selection.clone(),
                        selected_ink: to_paint(style.selected_color),
                        unselected_ink: to_paint(style.unselected_color),
                        indicator: None,
                        last_selected: usize::MAX,
                    };
                    Box::new(
                        Pressable::new(Child::new(
                            ChildKey::index(0),
                            LayoutRequest::new().fill(),
                            move || Box::new(content.clone()),
                        ))
                        .label(label.clone())
                        .on_click({
                            let selection = selection.clone();
                            move || {
                                selection.store(index, Ordering::SeqCst);
                                if let Some(callback) = &on_change {
                                    callback(index);
                                }
                            }
                        }),
                    )
                },
            ));
        }
        out
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(
            LayoutRequest::new()
                // v1's bar: `width_full()`.
                .width_pct(100.0)
                .height(self.style.height)
                .stack(Axis::Horizontal, 0.0)
                // v1's bar: `items_center().justify_center()`.
                .main_align(Align::Center)
                .cross_align(Align::Stretch),
        )
    }

    /// The bar's fill, and the rule along its top edge.
    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        cx.fill_rect(bounds, to_paint(self.style.bar_color));
        // v1's bar: `.border_top(1.0).border_color(outline_variant)`.
        cx.fill_rect(
            Rect::from_origin_size(
                bounds.origin(),
                Size::new(bounds.width(), f64::from(self.style.divider_width)),
            ),
            to_paint(self.style.divider_color),
        );
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        floom_widgets::emit_semantics(floom_widgets::SemanticsHint::decorative(), cx);
    }
}

/// One destination: the icon above the label, and the label's ink following
/// the choice.
#[derive(Clone)]
struct MaterialDestination {
    index: usize,
    label: String,
    font_size: f32,
    /// The gap between the icon and the label.
    inner_gap: f32,
    icon: Option<IconBuilder>,
    /// The box the icon is held in, when the kit sizes it.
    icon_size: Option<f32>,
    selection: Arc<AtomicUsize>,
    selected_ink: PaintColor,
    unselected_ink: PaintColor,
    /// The rule along this destination's top edge, when the kit draws one.
    indicator: Option<(f32, PaintColor)>,
    /// The choice this destination last acted on, so a change is a change
    /// rather than a fresh read every frame.
    last_selected: usize,
}

impl floom_core::WidgetAny for MaterialDestination {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// The destination names itself through the `Pressable` around it, so this
    /// node adds nothing of its own.
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

impl Widget for MaterialDestination {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let now = self.selection.load(Ordering::SeqCst);
        if now == self.last_selected {
            return;
        }
        self.last_selected = now;
        // The label's ink follows the choice, and a `Text` cannot change its
        // colour in place -- so the label is re-keyed and rebuilt. The
        // destination itself is not re-keyed, so the keyboard focus a click
        // asked for survives the change.
        cx.mark_children_dirty(id);
    }

    fn children(&self) -> Vec<Child> {
        let mut out = Vec::with_capacity(2);
        if let Some(icon) = self.icon.clone() {
            let child = match self.icon_size {
                Some(size) => Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new().width(size).height(size),
                    move || {
                        // The slot is a replayed `Fn`, so the icon is copied out
                        // of it before it is handed to the inner closure.
                        let icon = icon.clone();
                        Box::new(
                            Stack::new(Axis::Horizontal)
                                .align(Align::Center, Align::Center)
                                .push(move || icon()),
                        )
                    },
                ),
                None => Child::new(ChildKey::index(0), LayoutRequest::new(), move || {
                    icon()
                }),
            };
            out.push(child);
        }
        let selected = self.selection.load(Ordering::SeqCst) == self.index;
        let ink = if selected {
            self.selected_ink
        } else {
            self.unselected_ink
        };
        let text = self.label.clone();
        let font_size = self.font_size;
        out.push(Child::new(
            ChildKey::keyed(LABEL_KEY ^ (selected as u64)),
            LayoutRequest::new(),
            move || Box::new(Text::new(text.clone()).size(font_size).color(ink)),
        ));
        out
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(
            LayoutRequest::new()
                .stack(Axis::Vertical, self.inner_gap)
                // v1's destination: `items_center().justify_center()`.
                .main_align(Align::Center)
                .cross_align(Align::Center),
        )
    }

    /// The rule along this destination's top edge, when the kit draws one.
    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let Some((width, color)) = self.indicator else {
            return;
        };
        if self.selection.load(Ordering::SeqCst) != self.index {
            return;
        }
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 {
            return;
        }
        cx.fill_rect(
            Rect::from_origin_size(
                bounds.origin(),
                Size::new(bounds.width(), f64::from(width)),
            ),
            color,
        );
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        floom_widgets::emit_semantics(floom_widgets::SemanticsHint::decorative(), cx);
    }
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
