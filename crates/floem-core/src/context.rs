//! Context objects passed into widget lifecycle methods.

use floem_types::{ElementId, Point, Rect, Size};

/// Context available while creating an element from a widget.
pub trait CreateContext {
    fn alloc_child(&mut self) -> ElementId;
}

/// Context available during reactive/property updates.
pub trait UpdateContext {
    fn mark_dirty(&mut self, id: ElementId);
}

/// Pointer/keyboard routing context (payload-agnostic for now).
pub trait EventContext {
    fn target(&self) -> ElementId;
    fn cursor_position(&self) -> Option<Point>;
}

/// Paint recording context — emits geometry only; no GPU types.
pub trait PaintContext {
    fn fill_rect(&mut self, rect: Rect);
    fn bounds(&self) -> Rect;
}

/// Accessibility / semantics emission context.
pub trait SemanticsContext {
    fn set_label(&mut self, label: &str);
    fn set_bounds(&mut self, bounds: Rect);
    fn size_hint(&self) -> Size;
}
