//! Contract compilation and primitive widget smoke tests.

use floem_core::{
    Axis, Child, ChildKey, Color, CreateContext, Element, ElementId, ElementKind, Event,
    EventContext, LayoutRequest, PaintContext, Point, Rect, SemanticsContext, SemanticsHint, Size,
    UpdateContext, Widget, WidgetAny,
};
use floem_types::{Modifiers, PointerEvent};
use std::any::Any;
use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct TestCreateCx {
    next: u32,
}

impl CreateContext for TestCreateCx {
    fn alloc_child(&mut self) -> ElementId {
        let index = self.next;
        self.next += 1;
        ElementId::new(index, 1)
    }
}

struct TestUpdateCx {
    dirty: RefCell<Vec<ElementId>>,
    children_dirty: RefCell<Vec<ElementId>>,
}

impl TestUpdateCx {
    fn new() -> Self {
        Self {
            dirty: RefCell::new(Vec::new()),
            children_dirty: RefCell::new(Vec::new()),
        }
    }
}

impl UpdateContext for TestUpdateCx {
    fn mark_dirty(&mut self, id: ElementId) {
        self.dirty.borrow_mut().push(id);
    }

    fn mark_children_dirty(&mut self, id: ElementId) {
        self.children_dirty.borrow_mut().push(id);
    }
}

struct TestEventCx {
    target: ElementId,
    bounds: Rect,
    event: Event,
    focus_requests: Vec<ElementId>,
}

impl TestEventCx {
    fn pointer(target: ElementId, event: PointerEvent) -> Self {
        Self {
            target,
            bounds: Rect::ZERO,
            event: Event::Pointer(event),
            focus_requests: Vec::new(),
        }
    }
}

impl EventContext for TestEventCx {
    fn target(&self) -> ElementId {
        self.target
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn cursor_position(&self) -> Option<Point> {
        match &self.event {
            Event::Pointer(p) => Some(p.position),
            _ => None,
        }
    }

    fn event(&self) -> Event {
        self.event.clone()
    }

    fn request_focus(&mut self, id: ElementId) {
        self.focus_requests.push(id);
    }
}

struct TestPaintCx {
    fills: Vec<Rect>,
    colors: Vec<Color>,
    bounds: Rect,
}

impl PaintContext for TestPaintCx {
    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.fills.push(rect);
        self.colors.push(color);
    }
}

struct TestSemanticsCx {
    label: Option<String>,
    bounds: Rect,
}

impl SemanticsContext for TestSemanticsCx {
    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_label(&mut self, label: &str) {
        self.label = Some(label.to_string());
    }

    fn set_bounds(&mut self, _bounds: Rect) {}

    fn size_hint(&self) -> Size {
        Size::new(10.0, 10.0)
    }
}

/// Minimal colored rectangle primitive used to prove the Widget contract compiles.
struct ColorRect {
    color: Color,
    size: Size,
}

impl Widget for ColorRect {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        let _ = self.color;
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        cx.mark_dirty(id);
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let origin = cx.bounds().origin();
        cx.fill_rect(Rect::from_origin_size(origin, self.size), self.color);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        cx.set_label("color-rect");
        let bounds = cx.bounds();
        cx.set_bounds(bounds);
    }
}

#[test]
fn color_rect_widget_lifecycle() {
    let mut widget = ColorRect {
        color: Color::rgb(255, 0, 0),
        size: Size::new(20.0, 10.0),
    };
    let mut create = TestCreateCx { next: 0 };
    let id = widget.create(&mut create);
    assert!(!id.is_invalid());

    let element = Element::new(id, ElementKind::Primitive);
    assert_eq!(element.id, id);

    let mut update = TestUpdateCx::new();
    widget.update(&mut update, id);
    assert_eq!(update.dirty.into_inner(), vec![id]);

    let paint_bounds = Rect::from_origin_size(Point::ZERO, Size::new(100.0, 100.0));
    let mut paint = TestPaintCx {
        fills: Vec::new(),
        colors: Vec::new(),
        bounds: paint_bounds,
    };
    widget.paint(&mut paint, id);
    assert_eq!(paint.fills.len(), 1);
    assert_eq!(paint.fills[0].width(), 20.0);
    assert_eq!(paint.colors, vec![Color::rgb(255, 0, 0)]);

    let mut sem = TestSemanticsCx {
        label: None,
        bounds: paint_bounds,
    };
    widget.semantics(&mut sem, id);
    assert_eq!(sem.label.as_deref(), Some("color-rect"));

    let mut event = TestEventCx::pointer(id, PointerEvent::primary_down(Point::ZERO));
    assert!(!widget.event(&mut event, id));

    let pressed = PointerEvent::primary_down(Point::ZERO).with_modifiers(Modifiers::SHIFT);
    let mut event = TestEventCx::pointer(id, pressed);
    assert_eq!(event.cursor_position(), Some(Point::ZERO));
    event.request_focus(id);
    assert_eq!(event.focus_requests, vec![id]);
    match event.event() {
        Event::Pointer(p) => assert!(p.modifiers.contains(Modifiers::SHIFT)),
        other => panic!("expected pointer event, got {other:?}"),
    }
}
// --- composition -----------------------------------------------------------

/// Minimal child widget.
struct Row {
    label: String,
}

impl Widget for Row {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {}
}

impl WidgetAny for Row {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<SemanticsHint> {
        Some(SemanticsHint::decorative())
    }
}

/// A composite whose child list grows over time.
struct DynamicList {
    rows: Vec<String>,
    /// Bumped every time a child slot is actually constructed.
    builds: Arc<AtomicUsize>,
    structure_changed: bool,
}

impl DynamicList {
    fn new(rows: &[&str]) -> Self {
        Self {
            rows: rows.iter().map(|row| row.to_string()).collect(),
            builds: Arc::new(AtomicUsize::new(0)),
            structure_changed: true,
        }
    }
}

impl Widget for DynamicList {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        if self.structure_changed {
            self.structure_changed = false;
            cx.mark_children_dirty(id);
        }
    }

    fn children(&self) -> Vec<Child> {
        self.rows
            .iter()
            .map(|row| {
                let label = row.clone();
                let builds = self.builds.clone();
                Child::new(row.as_str(), LayoutRequest::new().height(24.0), move || {
                    builds.fetch_add(1, Ordering::Relaxed);
                    Box::new(Row {
                        label: label.clone(),
                    })
                })
            })
            .collect()
    }
}

#[test]
fn leaf_widgets_declare_no_children() {
    let widget = ColorRect {
        color: Color::rgb(0, 0, 0),
        size: Size::new(1.0, 1.0),
    };
    assert!(widget.children().is_empty());
}

#[test]
fn layout_request_is_engine_neutral() {
    let request = LayoutRequest::panel(Axis::Vertical, 8.0);
    assert_eq!(request.flex_grow, Some(1.0));
    assert_eq!(request.axis, Some(Axis::Vertical));
    assert_eq!(request.gap, Some(8.0));
    assert_eq!(Axis::Vertical.cross(), Axis::Horizontal);

    // "Unspecified" stays distinguishable from "zero": that distinction is why
    // every field is optional, and losing it would make the default opaque.
    let bare = LayoutRequest::new();
    assert_eq!(bare.gap, None);
    assert_eq!(bare.axis, None);
    assert_eq!(bare.grow(0.0).flex_grow, Some(0.0));
}

#[test]
fn child_keys_decide_whether_identity_survives_a_reorder() {
    // Positional keys: identity follows the slot.
    assert_eq!(ChildKey::index(3), ChildKey::from(3usize));
    assert_ne!(ChildKey::index(3), ChildKey::index(4));

    // Named keys: identity follows the item, so inserting earlier does not
    // invalidate it. Stable across calls, unlike a random seed.
    assert_eq!(ChildKey::from("alpha"), ChildKey::from("alpha"));
    assert_ne!(ChildKey::from("alpha"), ChildKey::from("beta"));
    assert_ne!(
        ChildKey::from("alpha"),
        ChildKey::index(0),
        "named and positional keys must not share a range"
    );
}

#[test]
fn mark_children_dirty_is_distinct_from_a_repaint() {
    let mut widget = DynamicList::new(&["one"]);
    let id = ElementId::new(42, 1);
    let mut cx = TestUpdateCx::new();
    widget.update(&mut cx, id);

    assert!(
        cx.dirty.borrow().is_empty(),
        "a structure change must not masquerade as a repaint"
    );
    assert_eq!(cx.children_dirty.borrow().as_slice(), &[id]);

    // A second update without a structure change asks for nothing.
    let mut cx = TestUpdateCx::new();
    widget.update(&mut cx, id);
    assert!(cx.children_dirty.borrow().is_empty());
}

#[test]
fn child_keys_are_stable_across_requests() {
    let widget = DynamicList::new(&["one", "two"]);
    let first: Vec<ChildKey> = widget.children().iter().map(|child| child.key).collect();
    let second: Vec<ChildKey> = widget.children().iter().map(|child| child.key).collect();
    assert_eq!(first, second, "the same rows must yield the same keys");
    assert_eq!(first.len(), 2);
}

#[test]
fn child_builders_are_deferred() {
    let widget = DynamicList::new(&["one", "two"]);
    let children = widget.children();
    assert_eq!(
        widget.builds.load(Ordering::Relaxed),
        0,
        "declaring a child builds nothing"
    );

    let _first = children[0].build();
    assert_eq!(widget.builds.load(Ordering::Relaxed), 1);

    // Re-declaring is free; only building costs.
    let again = widget.children();
    assert_eq!(again.len(), 2);
    assert_eq!(
        widget.builds.load(Ordering::Relaxed),
        1,
        "re-declaration must not rebuild"
    );
}
