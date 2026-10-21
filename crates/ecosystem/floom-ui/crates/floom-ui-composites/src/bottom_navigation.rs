//! `BottomNavigationBar` -- a bar of destinations, one of them chosen.
//!
//! A row of destinations: an icon above a label, the label's ink following
//! the choice, and a rule along the bar's top edge. The three floem-ui bottom
//! navigation kits each used to carry their own copy of this bar and of the
//! destination inside it; they share the engine's one now.
//!
//! ## What the kits still answer
//!
//! The bar's fill, the chosen and unchosen inks, the rule's colour and
//! thickness, the bar's height, the smallest a destination may be, the icon's
//! size, the label's size, the gap between the icon and the label. All of
//! those are design-system answers; the *shape* -- a bar of destinations with
//! one chosen -- is not.
//!
//! ## The three kits did not agree on everything
//!
//! v1's kits differed in ways that are not a theme's answer, so they are
//! three fields here rather than one:
//!
//! * **the icon's size.** Cupertino and Fluent held the icon in a box of a
//!   given size; Material did not, and left the icon to size itself. So the
//!   engine's `icon_size` is an `Option`.
//! * **the rule along a chosen destination's top edge.** Fluent drew one;
//!   the other two did not. So the engine's `indicator` is an `Option`.
//! * **the fill under a destination while the pointer is over it.** Only
//!   Cupertino had one, and only when the destination was *not* the chosen
//!   one. So the engine's `hover_color` is an `Option`.
//!
//! ## The choice travels in a shared counter
//!
//! The chosen destination changes after mount -- a tap moves it -- so it
//! cannot live in a signal, which is `Rc`-based and cannot cross a slot
//! boundary. It is an atomic counter the kit, the bar and the destinations'
//! clicks all share.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{
    Child, ChildKey, Color, CreateContext, ElementId, Event, EventContext, Insets, LayoutRequest,
    PaintContext, PointerPhase, Rect, SemanticsContext, Size, UpdateContext, Widget, WidgetAny,
};
use floom_widgets::{Align, Axis, Pressable, Stack, Text};

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

/// The key the destination's label is mounted under.
///
/// Keyed rather than positional, because the label is re-mounted when the
/// choice moves: a `Text` cannot change its ink in place, so the label is
/// rebuilt with the colour the choice calls for.
const LABEL_KEY: u64 = 0xBA0_0000_0000_0000;

/// The bottom navigation's appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BottomNavigationStyle {
    /// The bar's fill.
    pub bar_color: Color,
    /// The chosen destination's ink.
    pub selected_color: Color,
    /// The other destinations' ink.
    pub unselected_color: Color,
    /// The rule along the bar's top edge.
    pub divider_color: Color,
    /// The rule's thickness.
    pub divider_width: f32,
    /// The bar's height.
    pub height: f32,
    /// The smallest a destination may be. `None` for a kit whose destinations
    /// asked for no minimum width.
    pub item_min_width: Option<f32>,
    /// The box the icon is held in, when the kit sizes it. `None` for a kit
    /// that left the icon to size itself.
    pub icon_size: Option<f32>,
    /// The label's size.
    pub label_font_size: f32,
    /// The gap between a destination's icon and its label.
    pub gap: f32,
    /// The space above the label.
    pub label_margin_top: f32,
    /// The padding above and below a destination's content.
    pub destination_padding_vert: f32,
    /// The rule along a chosen destination's top edge: its thickness and its
    /// colour. `None` for a kit that drew no such rule.
    pub indicator: Option<(f32, Color)>,
    /// The fill under a destination while the pointer is over it. `None` for a
    /// kit that had no hover fill.
    pub hover_color: Option<Color>,
}

impl Default for BottomNavigationStyle {
    fn default() -> Self {
        Self {
            bar_color: Color::rgb(0xFF, 0xFF, 0xFF),
            selected_color: Color::rgb(0x67, 0x50, 0xA4),
            unselected_color: Color::rgb(0x49, 0x45, 0x55),
            divider_color: Color::rgb(0xE7, 0xE0, 0xEC),
            divider_width: 1.0,
            height: 80.0,
            item_min_width: Some(64.0),
            icon_size: None,
            label_font_size: 12.0,
            gap: 4.0,
            label_margin_top: 0.0,
            destination_padding_vert: 0.0,
            indicator: None,
            hover_color: None,
        }
    }
}

/// A bar of destinations, one of them chosen.
pub struct BottomNavigationBar {
    /// The destinations' labels, in display order.
    labels: Vec<String>,
    /// The icon each destination shows, when it has one.
    icons: Vec<IconBuilder>,
    /// The destination on show. Shared with the kit and with the taps.
    selection: Arc<AtomicUsize>,
    /// The kit's answer.
    style: BottomNavigationStyle,
    /// Called with the chosen destination's index whenever the choice moves.
    on_change: Option<Callback>,
}

impl BottomNavigationBar {
    pub fn new() -> Self {
        Self {
            labels: Vec::new(),
            icons: Vec::new(),
            selection: Arc::new(AtomicUsize::new(0)),
            style: BottomNavigationStyle::default(),
            on_change: None,
        }
    }

    /// The bar's appearance.
    pub fn style(mut self, style: BottomNavigationStyle) -> Self {
        self.style = style;
        self
    }

    /// The destinations' labels, in display order.
    pub fn labels(mut self, labels: Vec<String>) -> Self {
        self.labels = labels;
        self
    }

    /// The icon each destination shows, when it has one.
    pub fn icons(mut self, icons: Vec<IconBuilder>) -> Self {
        self.icons = icons;
        self
    }

    /// The counter the chosen destination lives in.
    pub fn shared_selection(mut self, selection: Arc<AtomicUsize>) -> Self {
        self.selection = selection;
        self
    }

    /// Called with the chosen destination's index whenever the choice moves.
    pub fn on_change(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

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

/// One destination's pressable: the destination's own box, and the click that
/// chooses it.
///
/// The destination is built inside the pressable's slot, because a slot is a
/// replayed `Fn` and the destination has to be rebuilt when it is mounted
/// again.
fn pressable_for(
    index: usize,
    label: String,
    icon: Option<IconBuilder>,
    style: BottomNavigationStyle,
    selection: Arc<AtomicUsize>,
    on_change: Option<Callback>,
) -> Pressable {
    let content = Destination {
        index,
        label: label.clone(),
        font_size: style.label_font_size,
        inner_gap: style.gap,
        icon: icon.clone(),
        icon_size: style.icon_size,
        selection: selection.clone(),
        selected_ink: style.selected_color,
        unselected_ink: style.unselected_color,
        indicator: style.indicator,
        label_margin_top: style.label_margin_top,
        destination_padding_vert: style.destination_padding_vert,
        last_selected: usize::MAX,
    };
    Pressable::new(Child::new(
        ChildKey::index(0),
        LayoutRequest::new().fill(),
        move || Box::new(content.clone()),
    ))
    .label(label.clone())
    .on_click(move || {
        selection.store(index, Ordering::SeqCst);
        if let Some(callback) = &on_change {
            callback(index);
        }
    })
}

impl Widget for BottomNavigationBar {
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
            let mut request = LayoutRequest::new().grow(1.0);
            if let Some(min_width) = style.item_min_width {
                request = request.min_width(min_width);
            }
            out.push(Child::new(
                ChildKey::index(index),
                // v1's destination grew to fill the bar. Whether it asked for
                // a minimum width is a kit's answer: v1's Cupertino and
                // Material ones did, and v1's Fluent one asked for none.
                request,
                move || {
                    // The slot is a replayed `Fn`, so everything it hands to a
                    // nested closure is copied out of it first.
                    let label = label.clone();
                    let icon = icon.clone();
                    let selection = selection.clone();
                    let on_change = on_change.clone();
                    match style.hover_color {
                        Some(hover_color) => {
                            // The destination's own box, above the pressable:
                            // the pointer has to reach it for the hover fill to
                            // appear.
                            let pressable_selection = selection.clone();
                            let pressable = Child::new(
                                ChildKey::index(0),
                                LayoutRequest::new().fill(),
                                move || {
                                    Box::new(pressable_for(
                                        index,
                                        label.clone(),
                                        icon.clone(),
                                        style,
                                        pressable_selection.clone(),
                                        on_change.clone(),
                                    ))
                                },
                            );
                            Box::new(HoverHost {
                                index,
                                selection: selection.clone(),
                                hover_color,
                                hovered: false,
                                last_hovered: false,
                                last_chosen: false,
                                pressable,
                            })
                        }
                        None => Box::new(pressable_for(
                            index,
                            label.clone(),
                            icon.clone(),
                            style,
                            selection.clone(),
                            on_change.clone(),
                        )),
                    }
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
        cx.fill_rect(bounds, self.style.bar_color);
        // v1's bar: `.border_top(1.0).border_color(outline_variant)`.
        cx.fill_rect(
            Rect::from_origin_size(
                bounds.origin(),
                Size::new(bounds.width(), f64::from(self.style.divider_width)),
            ),
            self.style.divider_color,
        );
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        floom_widgets::emit_semantics(floom_widgets::SemanticsHint::decorative(), cx);
    }
}

/// One destination's box: the fill a kit put under it while the pointer was
/// over it, and the pressable that chooses it.
///
/// The box is *above* the pressable on purpose. 2.0's `Pressable` tracks the
/// pointer but paints no fill, and the pointer never reaches a widget *inside*
/// a pressable -- the pressable consumes the pointer phases on the way down.
#[derive(Clone)]
struct HoverHost {
    index: usize,
    selection: Arc<AtomicUsize>,
    /// The fill under the pointer.
    hover_color: Color,
    /// Whether the pointer is over this destination right now.
    hovered: bool,
    /// What `hovered` was when this widget last acted, so a change is a change
    /// rather than a fresh read every frame.
    last_hovered: bool,
    /// Whether this destination was the chosen one when this widget last
    /// acted: v1's hover fill does not appear on the chosen destination, so
    /// the choice moving has to invalidate the fill too.
    last_chosen: bool,
    pressable: Child,
}

impl floom_core::WidgetAny for HoverHost {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// The host is a fill under the destination; the destination's own
    /// `Pressable` names it.
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

impl Widget for HoverHost {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let hovered = self.hovered;
        let chosen = self.selection.load(Ordering::SeqCst) == self.index;
        if hovered == self.last_hovered && chosen == self.last_chosen {
            return;
        }
        self.last_hovered = hovered;
        self.last_chosen = chosen;
        // The fill is this box's own decoration, so this box is what has to
        // invalidate it. A repaint and nothing else: the box does not move.
        cx.mark_dirty(id);
    }

    fn children(&self) -> Vec<Child> {
        vec![self.pressable.clone()]
    }

    /// v1's `.hover(..)`: the destination is filled while the pointer is over
    /// it -- unless it is the chosen one, which keeps its own appearance.
    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        if !self.hovered {
            return;
        }
        if self.selection.load(Ordering::SeqCst) == self.index {
            return;
        }
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        cx.fill_rect(bounds, self.hover_color);
    }

    /// The pointer arriving and leaving.
    ///
    /// The event is not consumed: v1's destination let the click through, and
    /// the pressable below is the one that answers it.
    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        let Event::Pointer(pointer) = cx.event() else {
            return false;
        };
        match pointer.phase {
            PointerPhase::Enter | PointerPhase::Move => self.hovered = true,
            PointerPhase::Leave | PointerPhase::Cancel => self.hovered = false,
            _ => {}
        }
        false
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        floom_widgets::emit_semantics(floom_widgets::SemanticsHint::decorative(), cx);
    }
}

/// One destination: the icon above the label, and the label's ink following
/// the choice.
#[derive(Clone)]
struct Destination {
    index: usize,
    label: String,
    font_size: f32,
    /// The gap between the icon and the label.
    inner_gap: f32,
    icon: Option<IconBuilder>,
    /// The box the icon is held in, when the kit sizes it.
    icon_size: Option<f32>,
    selection: Arc<AtomicUsize>,
    selected_ink: Color,
    unselected_ink: Color,
    /// The rule along this destination's top edge, when the kit draws one.
    indicator: Option<(f32, Color)>,
    /// The space above the label.
    label_margin_top: f32,
    /// The padding above and below the destination's content.
    destination_padding_vert: f32,
    /// The choice this destination last acted on, so a change is a change
    /// rather than a fresh read every frame.
    last_selected: usize,
}

impl floom_core::WidgetAny for Destination {
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

impl Widget for Destination {
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
        let margin_top = self.label_margin_top;
        out.push(Child::new(
            ChildKey::keyed(LABEL_KEY ^ (selected as u64)),
            if margin_top > 0.0 {
                LayoutRequest::new().margin(Insets {
                    left: 0.0,
                    right: 0.0,
                    top: f64::from(margin_top),
                    bottom: 0.0,
                })
            } else {
                LayoutRequest::new()
            },
            move || Box::new(Text::new(text.clone()).size(font_size).color(ink)),
        ));
        out
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        let mut request = LayoutRequest::new()
            .stack(Axis::Vertical, self.inner_gap)
            // v1's destination: `items_center().justify_center()`.
            .main_align(Align::Center)
            .cross_align(Align::Center);
        if self.destination_padding_vert > 0.0 {
            // v1's destination: `padding_vert(8.0)`.
            request = request.padding_vert(f64::from(self.destination_padding_vert));
        }
        Some(request)
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
