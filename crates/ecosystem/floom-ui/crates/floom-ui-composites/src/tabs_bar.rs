//! `TabsBar` -- a row of tabs, one of them chosen.
//!
//! A row of labels, one of them chosen, with the chosen one's ink and weight
//! following the choice. The three floem-ui tabs kits each used to carry their
//! own copy of this; they share the engine's one now.
//!
//! ## What the kits still answer
//!
//! The bar's fill, the active and inactive inks, the two label weights, the
//! indicator's colour and thickness, the label's size, the padding at each
//! side of a label, and the bar's height. All of those are design-system
//! answers; the *shape* -- a row of tabs with one chosen -- is not.
//!
//! ## The choice travels in a shared counter
//!
//! The chosen tab changes after mount -- a click moves it -- so it cannot live
//! in a signal, which is `Rc`-based and cannot cross a slot boundary. It is an
//! atomic counter the kit, the bar and the tabs' clicks all share.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{
    Child, ChildKey, Color, CreateContext, ElementId, LayoutRequest, PaintContext,
    UpdateContext, Widget,
};
use floom_types::FontWeight;
use floom_widgets::primitives::Revision;
use floom_widgets::{Align, Axis, Container, Pressable, Stack, Surface, Text};

/// The key the tab children are keyed by.
const TAB_KEY: u64 = 0x7A00_0000;

/// The bar's appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabsBarStyle {
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
    /// The label's size.
    pub label_font_size: f32,
    /// The active label's weight.
    pub active_label_font_weight: FontWeight,
    /// The inactive label's weight.
    pub inactive_label_font_weight: FontWeight,
    /// The bar's fill. `None` for a kit whose bar v1 left unpainted.
    pub bar_color: Option<Color>,
}

impl Default for TabsBarStyle {
    fn default() -> Self {
        Self {
            active_color: Color::rgb(0x67, 0x50, 0xA4),
            inactive_color: Color::rgb(0x49, 0x45, 0x55),
            indicator_color: Color::rgb(0x67, 0x50, 0xA4),
            height: 48.0,
            indicator_height: 2.0,
            padding: 16.0,
            label_font_size: 14.0,
            active_label_font_weight: FontWeight::Medium,
            inactive_label_font_weight: FontWeight::Medium,
            bar_color: None,
        }
    }
}


impl TabsBar {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            style: TabsBarStyle::default(),
            selection: Arc::new(AtomicUsize::new(0)),
            declared: usize::MAX,
            on_change: None,
            decoration: Container::new(),
            revision: Revision::default(),
        }
    }

    /// The bar's appearance.
    pub fn style(mut self, style: TabsBarStyle) -> Self {
        // The bar's fill is drawn by a decoration box, so it follows the style
        // rather than being set a second time.
        self.decoration = match style.bar_color {
            Some(color) => Container::new().background(color),
            None => Container::new(),
        };
        self.style = style;
        self
    }

    /// The tab labels, in display order.
    pub fn tabs(mut self, tabs: Vec<String>) -> Self {
        self.tabs = tabs;
        self
    }

    /// The counter the chosen tab lives in.
    pub fn selection(mut self, selection: Arc<AtomicUsize>) -> Self {
        self.selection = selection;
        self
    }

    /// Called with the chosen tab's index whenever the choice moves.
    pub fn on_change(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }
}

pub struct TabsBar {
    /// The tab labels, in display order.
    tabs: Vec<String>,
    /// The kit's answer.
    style: TabsBarStyle,
    /// The tab on show. Shared with the kit and with the tabs' clicks.
    selection: Arc<AtomicUsize>,
    /// The selection the children were last declared for.
    declared: usize,
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
    /// The bar's own fill. v1's Material bar was left unpainted.
    decoration: Container,
    revision: Revision,
}

impl TabsBar {
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

impl Widget for TabsBar {
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
        // v1's bar: `list(rows)` with `flex_row()` and the bar's height.
        Some(LayoutRequest::new().height(self.style.height))
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
    style: TabsBarStyle,
    selection: Arc<AtomicUsize>,
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
) -> Pressable {
    let ink = if active {
        style.active_color
    } else {
        style.inactive_color
    };
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
            style.indicator_color,
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
    ink: Color,
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
fn indicator_box(color: Color, height: f32) -> Child {
    Child::new(
        ChildKey::index(0),
        // v1's `height(indicator_height)` and `width_pct(100.0)`.
        LayoutRequest::new().height(height).width_pct(100.0),
        move || Box::new(Surface::new().background(color)),
    )
}

impl floom_core::WidgetAny for TabsBar {
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
