//! Material tabs, ported to the Floom 2.0 widget API.
//!
//! A tab bar: a row of labels, one of them active, with an indicator line
//! under the active one. Clicking a label makes it the active one.
//!
//! ## What the port uses
//!
//! The engine's vocabulary -- a `Pressable` per tab for the click, a vertical
//! `Stack` for the label and the indicator, a `Text` for the label -- plus one
//! widget of the kit's own: the **bar**, which owns the selection and
//! re-declares itself when the selection moves.
//!
//! ## The engine's `Tabs` composite, and why this kit does not use it
//!
//! `floom_ui_composites::Tabs` is the engine's tab bar, and its own
//! documentation is honest about what it is: "which indicator, how tall the
//! strip and what the inactive label looks like are three kits' answers to the
//! same question". But it does not take those answers. It draws a row of
//! engine buttons, prefixes the active label with "▶ ", uses a height of 24
//! and a gap of 4, and every tab carries a panel to show.
//!
//! v1's Fluent tabs are none of those things: an indicator line under the
//! active label, the theme's inks, the token's height of 48 and padding of
//! 16, and no panel at all. Using the composite would replace this kit with
//! something v1 never drew, so the port does not use it.
//!
//! What the composite *is*, though, is the shape this kit wants, and the
//! port's bar is that shape specialised to this kit's answer: the bar owns the
//! selection, re-declares itself when the selection moves, and bakes the
//! active label's appearance into the widget it builds. The piece that is
//! missing from the engine -- a bar that takes a kit's answer -- is the gap
//! this port names, and until it exists each kit carries its own copy.
//!
//! ## What this kit answers
//!
//! * the bar's height, the indicator's thickness and the space at each side of
//!   a label: the tokens;
//! * the active label's ink: `primary`; the inactive label's:
//!   `on_surface_variant`; the indicator's: `primary`;
//! * the label's size: v1 wrote none, so the label was drawn at v1's default
//!   of 14;
//! * the active label's weight: semi-bold; the inactive label's: medium --
//!   v1 literals;
//! * the bar's fill: none, because v1's Fluent bar was left unpainted.
//!
//! ## The bar is taller than the other kits'
//!
//! v1's Fluent bar is the token's height *plus* the indicator's thickness --
//! 48 + 3 -- which is exactly the height of the active tab: a 48-tall label
//! and a 3-tall indicator, so it fits. Material's and Cupertino's bars are 48
//! tall, so their active tab does not fit. The port keeps each kit's own
//! number.
//!
//! ## The tab is rebuilt when the selection moves
//!
//! 2.0 mounts a tree once, so a label's ink cannot change after the mount
//! unless the widget that draws it is rebuilt. The bar therefore keys each
//! tab by the selection, and a tab is rebuilt -- its label's ink and its
//! indicator baked in -- when the selection moves. The cost is that the tab
//! loses the keyboard focus it had, which v1 did not: v1's bar was built once
//! and only its *styles* were re-evaluated. The engine's own `Tabs` composite
//! makes the same trade, for the same reason.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The hover fill.** v1 painted `on_surface` at 8/255 over the *inactive*
//!   tab while the pointer was over it. 2.0's `Pressable` paints nothing of
//!   its own and the engine has no state-layer widget for a text label, so the
//!   fill is not carried. The tab is still clickable, and by keyboard too,
//!   which the pointer was not.
//! * **The pointer cursor.** v1 asked for `CursorStyle::Pointer` on the tab.
//!   2.0's `Pressable` does not answer `Widget::cursor`, so the pointer is not
//!   carried either.
//! * **The callback in the props.** v1 stored the change callback in
//!   `TabsProps::on_change`, which is a `Box<dyn Fn(usize)>`. 2.0's slots are
//!   shared between threads, so the callback the tree uses is an
//!   `Arc<dyn Fn(usize) + Send + Sync>` held beside the props, and the props'
//!   own slot stays empty.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{
    Child, ChildKey, CreateContext, ElementId, LayoutRequest, PaintContext, UpdateContext, Widget,
};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, FontWeight};
use floom_widgets::{Align, Axis, Container, Pressable, Revision, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::tabs::TabsProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

/// The label's size. A v1 literal.
const LABEL_FONT_SIZE: f32 = 14.0;

/// The key the tabs are mounted under. The selection is part of it, so a tab is
/// rebuilt when the selection moves.
const TAB_KEY: u64 = 0x7A00_0000;

/// The tabs' appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentTabsStyle {
    pub design_system: &'static str,
    /// The active label's ink.
    pub active_color: Color,
    /// The inactive label's ink.
    pub inactive_color: Color,
    /// The indicator line's colour.
    pub indicator_color: Color,
    /// The bar's height.
    pub height: f32,
    /// The indicator line's thickness.
    pub indicator_height: f32,
    /// The space at each side of a label.
    pub padding: f32,
    /// The label's size. v1 wrote 14.
    pub label_font_size: f32,
    /// The active label's weight. v1 wrote medium.
    pub active_label_font_weight: FontWeight,
    /// The inactive label's weight. v1 wrote medium.
    pub inactive_label_font_weight: FontWeight,
    /// The bar's fill. `None` for the kits whose bar v1 left unpainted.
    pub bar_color: Option<Color>,
}

/// Material's tabs.
pub struct FluentTabs {
    props: TabsProps,
    theme: RwSignal<MultiTheme>,
    /// The tab on show. Shared with the tree the kit builds.
    selection: Arc<AtomicUsize>,
    /// The change callback, in the shape 2.0's slots need.
    ///
    /// `Send + Sync` is required, not incidental: 2.0 rebuilds a widget inside a
    /// `Child` closure the host may hand between threads, so the callback is
    /// shared as an `Arc`. v1's callback was a `Box<dyn Fn(usize)>`, which
    /// cannot cross that boundary.
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
}

impl FluentTabs {
    pub fn new() -> Self {
        Self {
            props: TabsProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            selection: Arc::new(AtomicUsize::new(0)),
            on_change: None,
        }
    }

    /// Append a tab.
    pub fn tab(mut self, tab: impl Into<String>) -> Self {
        self.props.tabs.push(tab.into());
        self
    }

    /// Replace the tabs.
    pub fn tabs(mut self, tabs: Vec<String>) -> Self {
        self.props.tabs = tabs;
        self
    }

    /// The tab that is active before the bar is built.
    pub fn selected(mut self, selected: usize) -> Self {
        self.props.selected = selected;
        self.selection.store(selected, Ordering::SeqCst);
        self
    }

    /// Called with the index of the tab that was clicked.
    pub fn on_change(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// The shared props.
    ///
    /// `on_change` is deliberately absent from what it reports: the props are
    /// the v1 carrier of a `Box<dyn Fn(usize)>` and the kit holds that here.
    pub fn props(&self) -> &TabsProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// Which tab is active, as the shared state reports it.
    pub fn active(&self) -> Option<usize> {
        let count = self.props.tabs.len();
        if count == 0 {
            return None;
        }
        Some(self.selection.load(Ordering::SeqCst).min(count - 1))
    }

    pub fn resolved_style(&self) -> FluentTabsStyle {
        let theme = self.theme.get();
        let ComponentTokens::Tabs(tokens) = theme.component(ComponentId::Tabs) else {
            unreachable!("ComponentId::Tabs must resolve to TabsTokens")
        };
        FluentTabsStyle {
            design_system: "fluent",
            active_color: theme.colors().primary,
            inactive_color: theme.colors().on_surface_variant,
            indicator_color: theme.colors().primary,
            height: tokens.height,
            indicator_height: tokens.indicator_height,
            padding: tokens.padding,
            label_font_size: LABEL_FONT_SIZE,
            // v1 wrote `FontWeight::SemiBold` for the active label ...
            active_label_font_weight: FontWeight::SemiBold,
            // ... and `FontWeight::Medium` for the inactive one.
            inactive_label_font_weight: FontWeight::Medium,
            // v1's Fluent bar was left unpainted.
            bar_color: None,
        }
    }

    /// The bar: a row of tabs, with the active one marked.
    ///
    /// The kit is borrowed rather than consumed, so a caller can read
    /// [`Self::active`] after the bar has been built -- and the tree and the
    /// kit share the same selection, which is what makes the click move the
    /// highlight.
    pub fn view(&self) -> FluentTabsBar {
        let style = self.resolved_style();
        let bar_color = style.bar_color;
        FluentTabsBar {
            tabs: self.props.tabs.clone(),
            style,
            selection: self.selection.clone(),
            // Impossible, so the first frame always reconciles.
            declared: usize::MAX,
            on_change: self.on_change.clone(),
                        // v1's Fluent bar was left unpainted.
            decoration: match bar_color {
                Some(color) => Container::new().background(to_paint(color)),
                None => Container::new(),
            },
            revision: Revision::default(),
        }
    }
}

impl Default for FluentTabs {
    fn default() -> Self {
        Self::new()
    }
}

/// The bar: the tabs, and the one that is active.
///
/// The shape the engine's `Tabs` composite has, specialised to this kit's
/// answer: the bar owns the selection, re-declares itself when the selection
/// moves, and bakes the active label's appearance into the widget it builds.
pub struct FluentTabsBar {
    /// The tab labels, in display order.
    tabs: Vec<String>,
    /// The kit's answer.
    style: FluentTabsStyle,
    /// The tab on show. Shared with the kit and with the tabs' clicks.
    selection: Arc<AtomicUsize>,
    /// The selection the children were last declared for.
    declared: usize,
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
    /// The bar's own fill. v1's Fluent bar was left unpainted.
    decoration: Container,
    revision: Revision,
}

impl FluentTabsBar {
    /// The tab on show.
    pub fn selected(&self) -> usize {
        self.current()
    }

    /// The tab on show, clamped to the tabs there are.
    fn current(&self) -> usize {
        let count = self.tabs.len();
        if count == 0 {
            return 0;
        }
        self.selection.load(Ordering::SeqCst).min(count - 1)
    }
}

impl Widget for FluentTabsBar {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    /// The selection moved: the tabs have to be declared again, because the
    /// label's ink and the indicator are baked into the tab that is built.
    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
        let current = self.current();
        if current != self.declared {
            self.declared = current;
            cx.mark_children_dirty(id);
        }
    }

    fn children(&self) -> Vec<Child> {
        let index = self.current();
        let count = self.tabs.len();
        // The stride keeps the keys apart: the selection and the tab's place
        // in the row both go into the key.
        let stride = count as u64 + 1;
        let mut out = Vec::with_capacity(count);
        for (place, label) in self.tabs.iter().enumerate() {
            let active = place == index;
            let label = label.clone();
            let style = self.style;
            let selection = self.selection.clone();
            let on_change = self.on_change.clone();
            out.push(Child::new(
                // The selection is part of the key: a tab is rebuilt -- with
                // the ink and the indicator it now has -- when the selection
                // moves.
                ChildKey::keyed(TAB_KEY + (index as u64) * stride + place as u64),
                LayoutRequest::new(),
                move || {
                    Box::new(tab(
                        place,
                        label.clone(),
                        active,
                        style,
                        selection.clone(),
                        on_change.clone(),
                    ))
                },
            ));
        }
        out
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        // v1's bar: `list(rows)` with `flex_row()`, the full width of its
        // parent, and the token's height plus the indicator's.
        Some(
            LayoutRequest::new()
                .width_pct(100.0)
                .height(self.style.height + self.style.indicator_height),
        )
    }

    fn paint(&self, cx: &mut dyn PaintContext, id: ElementId) {
        self.decoration.paint(cx, id);
    }
}

/// One tab: the click target, the label, and the indicator under the active one.
fn tab(
    index: usize,
    label: String,
    active: bool,
    style: FluentTabsStyle,
    selection: Arc<AtomicUsize>,
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
) -> Pressable {
    let ink = to_paint(if active {
        style.active_color
    } else {
        style.inactive_color
    });
    let weight = if active {
        style.active_label_font_weight
    } else {
        style.inactive_label_font_weight
    };

    let mut body = Stack::new(Axis::Vertical)
        // v1's `.items_center()` on the tab's own stack.
        .align(Align::Start, Align::Center)
        .child(label_box(
            &label,
            style.label_font_size,
            weight,
            ink,
            style.height,
            style.padding,
        ));
    if active {
        // v1 hid the indicator on an inactive tab rather than drawing it
        // empty, and 2.0 has no `hide()`: the tab that is built is the one
        // that carries the indicator.
        body = body.child(indicator_box(
            to_paint(style.indicator_color),
            style.indicator_height,
        ));
    }

    Pressable::new(Child::new(
        ChildKey::index(0),
        LayoutRequest::new(),
        move || Box::new(body.clone()),
    ))
    // v1 published nothing to assistive technology. 2.0's `Pressable` does:
    // the port gives the tab its label as its accessible name and reports
    // which one is selected. The *role* stays the `Pressable`'s own -- a
    // button -- because naming the tab role means reaching for
    // `floom-accessibility`'s vocabulary, and no ported kit carries that
    // dependency.
    .label(label)
    .selected_from({
        let selection = selection.clone();
        move || selection.load(Ordering::SeqCst) == index
    })
    .on_click(move || {
        // v1: `selected.set(i)` and then the callback.
        selection.store(index, Ordering::SeqCst);
        if let Some(cb) = &on_change {
            cb(index);
        }
    })
}

/// The label's box: the text, centred in a box of the tab's height with the
/// token's padding at each side.
///
/// v1's label was a `Label` with `padding_horiz(padding)`, `height(height)`,
/// `items_center()` and `justify_center()`. 2.0's `Text` draws at the top-left
/// of its box, so the centring needs a box of its own: a horizontal stack
/// whose alignment centres the text on both axes.
fn label_box(
    label: &str,
    font_size: f32,
    font_weight: FontWeight,
    ink: PaintColor,
    height: f32,
    padding: f32,
) -> Child {
    let label = label.to_string();
    Child::new(
        ChildKey::index(0),
        // v1's `padding_horiz(padding)` and `height(height)`.
        LayoutRequest::new()
            .height(height)
            .padding_horiz(f64::from(padding)),
        move || {
            Box::new(
                Stack::new(Axis::Horizontal)
                    // v1's `items_center()` and `justify_center()`.
                    .align(Align::Center, Align::Center)
                    .push({
                        let label = label.clone();
                        move || {
                            Box::new(
                                Text::new(label.clone())
                                    .size(font_size)
                                    .weight(font_weight)
                                    .color(ink),
                            )
                        }
                    }),
            )
        },
    )
}

/// The indicator line under the active tab.
fn indicator_box(color: PaintColor, height: f32) -> Child {
    Child::new(
        ChildKey::index(0),
        // v1's `height(indicator_height)` and `width_pct(100.0)`.
        LayoutRequest::new().height(height).width_pct(100.0),
        move || Box::new(Surface::new().background(color)),
    )
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}

impl floom_core::WidgetAny for FluentTabsBar {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// The bar publishes nothing of its own: the tabs inside it are what
    /// speak.
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}
