//! Context objects passed into widget lifecycle methods.

use floem_types::{Color, ElementId, Point, Rect, ResourceId, Size};

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
    /// Layout bounds of the element being painted, in logical pixels.
    fn bounds(&self) -> Rect;

    /// Solid, axis-aligned rectangle.
    fn fill_rect(&mut self, rect: Rect, color: Color);

    /// Rectangle with uniformly rounded corners.
    ///
    /// Backends without a rounded-rect primitive may fall back to `fill_rect`;
    /// the default implementation does exactly that.
    fn fill_rounded_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        let _ = radius;
        self.fill_rect(rect, color);
    }

    /// A single run of text. `size` is the font size in logical pixels.
    ///
    /// Multi-paragraph / shaped text goes through `floem-text`; this op carries
    /// the unshaped string so renderer backends can intern a glyph run.
    fn fill_text(&mut self, origin: Point, text: &str, size: f32, color: Color) {
        let _ = (origin, text, size, color);
    }

    /// An already-registered image resource drawn into `bounds`.
    fn draw_image(&mut self, bounds: Rect, image: ResourceId) {
        let _ = (bounds, image);
    }
}

/// Accessibility / semantics emission context.
pub trait SemanticsContext {
    /// Bounds of the element semantics are being emitted for.
    fn bounds(&self) -> Rect;
    fn set_label(&mut self, label: &str);
    fn set_bounds(&mut self, bounds: Rect);
    fn size_hint(&self) -> Size;
}
