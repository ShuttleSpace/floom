//! Context objects passed into widget lifecycle methods.

use floem_types::{Color, ElementId, Event, Point, Rect, ResourceId, Size};

/// Context available while creating an element from a widget.
pub trait CreateContext {
    fn alloc_child(&mut self) -> ElementId;
}

/// Context available during reactive/property updates.
pub trait UpdateContext {
    fn mark_dirty(&mut self, id: ElementId);
    /// The children declared by `id` changed; rebuild that subtree.
    ///
    /// Requires [`crate::Widget::children`] to be consulted again, so it is
    /// strictly more expensive than a repaint. See [`crate::compose`].
    fn mark_children_dirty(&mut self, id: ElementId);
}

/// Pointer/keyboard routing context.
pub trait EventContext {
    fn target(&self) -> ElementId;
    /// Layout bounds of the element receiving the event.
    ///
    /// Required so controls can turn a pointer position into a value without
    /// caching their own geometry.
    fn bounds(&self) -> Rect;
    fn cursor_position(&self) -> Option<Point>;
    /// The event currently being dispatched.
    ///
    /// Required rather than optional: this context only exists *during*
    /// dispatch, so there is always a payload.
    fn event(&self) -> Event;
    /// Asks the host to move keyboard focus to `id`.
    ///
    /// Defaults to a no-op for hosts without a focus manager.
    fn request_focus(&mut self, id: ElementId) {
        let _ = id;
    }

    /// Asks the host to place `request.content` above the whole tree.
    ///
    /// Returns whether a host accepted it. A widget must check: hosts without
    /// overlay support (a unit-test context, a headless embedder that only
    /// wants the main tree) return `false`, and a control that ignored that
    /// would report a popup as open that nothing is going to paint.
    ///
    /// The overlay opens *after* the current event finishes dispatching, so the
    /// content is not mounted — and its handle is not yet valid for anything
    /// except passing back to [`EventContext::close_overlay`] — until the host
    /// has drained its command queue.
    fn open_overlay(&mut self, request: crate::OverlayRequest) -> bool {
        let _ = request;
        false
    }

    /// Asks the host to close the overlay identified by `handle`.
    ///
    /// Returns whether the handle named an open overlay. Defaults to `false`,
    /// so a widget can treat "not a host that knows this overlay" the same as
    /// "already closed" and drop its own reference either way.
    fn close_overlay(&mut self, handle: crate::OverlayHandle) -> bool {
        let _ = handle;
        false
    }
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
