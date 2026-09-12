//! Contract compilation and primitive widget smoke tests.

use floem_core::{
    Color, CreateContext, Element, ElementId, ElementKind, EventContext, PaintContext, Point,
    Rect, SemanticsContext, Size, UpdateContext, Widget,
};
use std::cell::RefCell;

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
}

impl UpdateContext for TestUpdateCx {
    fn mark_dirty(&mut self, id: ElementId) {
        self.dirty.borrow_mut().push(id);
    }
}

struct TestEventCx {
    target: ElementId,
}

impl EventContext for TestEventCx {
    fn target(&self) -> ElementId {
        self.target
    }

    fn cursor_position(&self) -> Option<Point> {
        None
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

    let mut update = TestUpdateCx {
        dirty: RefCell::new(Vec::new()),
    };
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

    let mut event = TestEventCx { target: id };
    assert!(!widget.event(&mut event, id));
}
