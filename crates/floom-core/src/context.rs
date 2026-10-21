//! Context objects passed into widget lifecycle methods.

use floom_types::{
    Brush, Color, ElementId, Event, Filter, FontWeight, Point, Rect, ResourceId, Shadow, Size,
    StyledText,
};

/// Context available while creating an element from a widget.
pub trait CreateContext {
    fn alloc_child(&mut self) -> ElementId;
}

/// Context available during reactive/property updates.
///
/// Carries the same overlay door as [`EventContext`] because a composite can be
/// driven by state rather than by an event: a `Dialog` sees `present` flip in
/// [`crate::Widget::update`], where there is no pointer or key to react to, and
/// still has to be able to ask the host to mount its surface.
pub trait UpdateContext {
    fn mark_dirty(&mut self, id: ElementId);
    /// The children declared by `id` changed; rebuild that subtree.
    ///
    /// Requires [`crate::Widget::children`] to be consulted again, so it is
    /// strictly more expensive than a repaint. See [`crate::compose`].
    fn mark_children_dirty(&mut self, id: ElementId);
    /// The *layout* `id` asked for changed; re-run layout before painting.
    ///
    /// A third request, distinct from the other two, and the one that is easiest
    /// to leave out. A widget whose `layout_request` moves — a padding, an inset,
    /// an intrinsic size — and that only calls [`Self::mark_dirty`] produces a
    /// repaint of a box that is still the old size: the ink is redrawn into the
    /// geometry the *previous* frame computed, because
    /// `WidgetHost::frame` runs layout only when layout is dirty and a repaint
    /// request does not make it dirty.
    ///
    /// Not defaulted, unlike the overlay and native doors below, because there is
    /// no honest fallback: a host that ignored this would silently mis-lay-out
    /// every widget that used it. Every `UpdateContext` has to answer it.
    fn mark_layout_dirty(&mut self, id: ElementId);

    /// The union of `id`'s mounted descendants' boxes, relative to `id`'s own
    /// box, or `None` if the host cannot say.
    ///
    /// This exists because a widget cannot otherwise know how big its own
    /// content is. Its `layout_request` states what it *wants*; the size of what
    /// it ends up containing is computed by the layout engine and lives in the
    /// host, which is borrowed while the widget's `update` runs. A scrolling
    /// container is the case that hurts: it must clamp its offset to
    /// `content - viewport`, and both halves are host-side numbers.
    ///
    /// Measured on the *unscrolled* layout, so the answer does not change when
    /// the container scrolls. A size that moved with the offset would be
    /// self-referential — scrolling would shrink the content, which would clamp
    /// the offset, which would shrink it further.
    ///
    /// Relative rather than absolute so a caller does not have to know where its
    /// own box sits on screen to compare it with its own bounds.
    ///
    /// Defaults to `None`, because a host that does not lay out (a headless
    /// recorder, a testkit that mounts without framing) has no honest answer,
    /// and a zero would be indistinguishable from "genuinely empty".
    fn content_size(&mut self, id: ElementId) -> Option<Size> {
        let _ = id;
        None
    }

    /// Layout box of the node being updated, in viewport coordinates.
    ///
    /// This is the *previous* frame's box — a widget is updated before layout
    /// runs — which is exactly right for anchoring an overlay to a control that
    /// already exists (`Select`'s list has nowhere to go otherwise). Defaults to
    /// an empty rect for update contexts that carry no geometry.
    fn bounds(&self) -> Rect {
        Rect::ZERO
    }

    /// The host viewport, for a composite whose overlay spans the whole window
    /// (a modal dialog's scrim, a bottom sheet's column).
    ///
    /// Defaults to an empty rect, which a caller should read as "no viewport
    /// knowledge" and not as a zero-sized window.
    fn viewport(&self) -> Rect {
        Rect::ZERO
    }

    /// Asks the host to place `request.content` above the whole tree.
    ///
    /// The state-driven twin of [`EventContext::open_overlay`]; the same rules
    /// apply. Returns `false` when the host has no overlay support, and a
    /// composite must have a fallback for that — otherwise flipping `present`
    /// on a headless host would mount nothing at all.
    fn open_overlay(&mut self, request: crate::OverlayRequest) -> bool {
        let _ = request;
        false
    }

    /// Asks the host to close the overlay identified by `handle`.
    ///
    /// Returns whether the handle named an open overlay; `false` covers both
    /// "no such overlay" and "host has no overlays", which a caller treats the
    /// same way — the reference it held is dead either way.
    fn close_overlay(&mut self, handle: crate::OverlayHandle) -> bool {
        let _ = handle;
        false
    }

    /// Asks the host to put a native view in this node's box.
    ///
    /// The state-driven twin of the overlay door: a widget states what it wants
    /// every `update()` and the host decides, because the host is what owns the
    /// participant's lifetime. A native view has to outlive the frame that
    /// created it, be re-bound when its slot is recycled for another row, and
    /// be destroyed exactly once — none of which a widget can arrange from
    /// inside its own `update`.
    ///
    /// Re-stating the same request is free; a host that has already granted it
    /// returns the same grant. Changing [`NativeRequest::key`] is how a
    /// virtualized row asks for a different slot.
    ///
    /// [`NativeGrant::Refused`] means the host has no provider, or the provider
    /// offers no mode this participant accepts. A caller must decide what to
    /// draw instead: silently showing nothing is how a "supported" feature
    /// becomes an empty rectangle on one platform.
    fn request_native(&mut self, request: &crate::NativeRequest) -> crate::NativeGrant {
        let _ = request;
        crate::NativeGrant::Refused
    }

    /// The texture this node's participant has been frozen into, if any.
    ///
    /// A participant cannot be *animated* by the OS that owns it: there is no
    /// "move this `NSView` along a curve" in any windowing API, and the ones
    /// that come closest (re-placing it every frame) still leave it unable to
    /// fade, scale or rotate. So a transition freezes the live view into a
    /// texture and the framework paints the texture instead — which is exactly
    /// what makes a participant follow a page transition at all.
    ///
    /// `Some` means "stop expecting the live view: it is hidden, and this image
    /// is what belongs in your box right now". A widget draws it like any other
    /// image; a widget that ignores this shows a hole where the participant was
    /// for the length of the transition, which is failure mode D11 names.
    ///
    /// Defaults to `None`, so every existing caller keeps drawing nothing.
    fn native_snapshot(&self) -> Option<crate::NativeSnapshot> {
        None
    }
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

    /// Asks the host to scroll this widget's subtree by `offset`.
    ///
    /// The request is recorded and applied once `event` returns, because the
    /// widget is borrowed out of the host while its `event` runs and the host is
    /// what owns the tree. Defaults to a no-op for hosts without scrolling.
    ///
    /// A widget requests rather than writes, and the offset applies to `target`
    /// — the widget being dispatched — and its descendants. A scroll container
    /// never scrolls its ancestors or its siblings, which is what keeps two
    /// containers independent.
    fn request_scroll(&mut self, offset: crate::ScrollOffset) {
        let _ = offset;
    }

    /// The same answer [`UpdateContext::content_size`] gives, on the path a
    /// wheel arrives by.
    ///
    /// A scrolling container needs the content's extent to clamp against, and a
    /// wheel event arrives here rather than in `update`. The host has already
    /// measured it for [`Widget::needs_content_size`] widgets, so the answer is
    /// the same number — this is the same measurement, read at the moment it is
    /// needed rather than cached and re-read.
    ///
    /// Defaults to `None`, for the same reason [`UpdateContext::content_size`]
    /// does: a host that does not lay out has no figure, and zero would be
    /// indistinguishable from empty.
    fn content_size(&mut self, id: ElementId) -> Option<Size> {
        let _ = id;
        None
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

    /// Reports the on-screen rectangle the IME composition caret occupies, so
    /// the runner can anchor the OS input-method popup above the right glyph.
    ///
    /// `Some(rect)` while a composition is open; `None` once it ends (or if the
    /// widget is not a text input). Defaults to a no-op for hosts that do not
    /// surface composition geometry — the runner only reads it on IME events
    /// and falls back to no caret-area handshake.
    fn set_ime_cursor_area(&mut self, area: Option<Rect>) {
        let _ = area;
    }
}

/// Paint recording context — emits geometry only; no GPU types.
pub trait PaintContext {
    /// Layout bounds of the element being painted, in logical pixels.
    fn bounds(&self) -> Rect;

    /// Solid, axis-aligned rectangle.
    fn fill_rect(&mut self, rect: Rect, color: Color);

    /// A rectangle filled with a [`Brush`] — a gradient where the design asks
    /// for one, [`Brush::Solid`] in the common case.
    ///
    /// This is the widget-facing half of the gradient gap (A3): without it a
    /// control can draw nothing but flat colours no matter what the scene IR
    /// supports, exactly the state Compose was in before it added a `brush:`
    /// overload beside every `color:` one. The brush's own coordinates are
    /// absolute logical px, so a gradient does *not* rescale with whatever
    /// rect receives it — a progress bar's ramp stays anchored to its track
    /// while the fill grows along it.
    ///
    /// The default is honest rather than silent: a backend with no per-pixel
    /// brush paints the rect in one representative colour (the brush sampled
    /// at the rect's centre) instead of dropping the shape. That is a visible
    /// approximation, and a backend that can do better overrides it.
    fn fill_rect_brush(&mut self, rect: Rect, brush: &Brush) {
        self.fill_rect(rect, brush.sample(rect.center()));
    }

    /// Rectangle with uniformly rounded corners.
    ///
    /// Backends without a rounded-rect primitive may fall back to `fill_rect`;
    /// the default implementation does exactly that.
    fn fill_rounded_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        let _ = radius;
        self.fill_rect(rect, color);
    }

    /// Rounded rectangle filled with a [`Brush`]. See
    /// [`PaintContext::fill_rect_brush`] for what the brush's own coordinates
    /// mean and why the default paints one representative colour.
    fn fill_rounded_rect_brush(&mut self, rect: Rect, radius: f32, brush: &Brush) {
        self.fill_rounded_rect(rect, radius, brush.sample(rect.center()));
    }

    /// Draws `shadow` behind `rect`: the rect's own silhouette, blurred and
    /// offset, composited *before* the caller draws anything on top of it.
    ///
    /// A separate op from the fill because a shadow belongs to a shape rather
    /// than being a shape: the caller decides the order (shadow first, then
    /// the shape it sits behind) and the backend decides how much of the blur
    /// it can actually do. A backend that cannot blur draws nothing here and
    /// is expected to say so in its own frame stats rather than stay quiet —
    /// the software backend counts what it drew, vello counts what it skipped.
    ///
    /// Defaults to nothing: a shadow is decoration, and a backend inventing one
    /// (a hard black rectangle, say) would be worse than the absence of it.
    fn draw_shadow(&mut self, rect: Rect, radius: f32, shadow: &Shadow) {
        let _ = (rect, radius, shadow);
    }

    /// Filters the pixels **already drawn** inside `rect` — the backdrop half
    /// of the filter vocabulary, and the frosted-glass case.
    ///
    /// The other half — filtering a *subtree* — is the host's job rather than
    /// an op: only the host knows where a subtree starts and ends in the draw
    /// order, so a widget asks for that with [`crate::Widget::filter`] and the
    /// host brackets it. This op is for the case where the input is whatever
    /// happens to be behind, which no bracket can express.
    ///
    /// Defaults to nothing. A backend that cannot reach the pixels it has
    /// already drawn must not paint a translucent panel and call it frost —
    /// the same reason [`Self::draw_shadow`] defaults to nothing rather than
    /// to a hard rectangle.
    fn backdrop_filter(&mut self, rect: Rect, filter: &Filter) {
        let _ = (rect, filter);
    }

    /// Runs a caller-supplied WGSL fragment shader over this element's box —
    /// the widget-facing half of the custom-shader gap (A5).
    ///
    /// The shader replaces this element's ink rather than decorating it: the
    /// box is the shader's whole output, so a caller does not also fill a rect
    /// behind it. What the shader sees is one normalised coordinate space over
    /// that box plus the caller's own scalars, which is why the request is a
    /// program and not a colour — see [`floom_types::Shader`] for the uniform
    /// contract it is compiled against.
    ///
    /// Defaults to nothing, and for the same reason [`Self::backdrop_filter`]
    /// does: a backend with no GPU program pipeline has no honest way to
    /// approximate an arbitrary fragment — drawing a flat grey would *look*
    /// like a shader that produced grey, which is worse than the absence.
    /// Backends that skip are expected to say so in their own frame stats
    /// (the software renderer counts `shaders_skipped`) rather than stay quiet.
    fn draw_shader(&mut self, shader: &floom_types::Shader) {
        let _ = shader;
    }

    /// A single run of text. `size` is the font size in logical pixels.
    ///
    /// Multi-paragraph / shaped text goes through `floom-text`; this op carries
    /// the unshaped string so renderer backends can intern a glyph run.
    ///
    /// `weight` is part of this signature rather than a separate op because a
    /// backend that keys its glyph cache on `(text, size)` will otherwise hand
    /// back the regular runs for a `Medium` request — same string, same size,
    /// silently the wrong weight, and nothing downstream can tell.
    fn fill_text(
        &mut self,
        origin: Point,
        text: &str,
        size: f32,
        weight: FontWeight,
        color: Color,
    ) {
        let _ = (origin, text, size, weight, color);
    }

    /// Text described in *ranges* rather than as one string.
    ///
    /// The plain [`Self::fill_text`] op carries one string in one ink. An
    /// editor needs more than that: a line can be a keyword in one colour, a
    /// string literal in another, and the rest in a third, and the geometry
    /// the caret uses has to be the geometry of *that* text.
    ///
    /// So this op carries a [`StyledText`] — the text, the style it starts
    /// with, and the ranges that differ — and the host shapes it. The
    /// description is *data*, not a shaped result, so it crosses the layer
    /// boundary without the lower layer naming the higher layer's types; and
    /// because the editor lays out with the *same* description it paints
    /// with, the caret lands on the pixels.
    ///
    /// The host shapes the description exactly as given. It does not re-wrap
    /// the text to the element's box the way [`Self::fill_text`] does,
    /// because a caller that laid out against a width and a host that wrapped
    /// against a different one would put the caret and the pixels in
    /// different places.
    ///
    /// Defaults to a no-op, like [`Self::fill_text`]: a host that cannot shape
    /// styled text draws nothing rather than drawing the text in one colour
    /// and calling it highlighted.
    fn fill_styled_text(&mut self, origin: Point, styled: &StyledText) {
        let _ = (origin, styled);
    }

    /// Fills an arbitrary vector path.
    ///
    /// Paths are in absolute logical coordinates, unlike the rect ops, which
    /// are also absolute — the difference is only that a path is not tied to
    /// the element's box, because the interesting ones (a chart line, a ring,
    /// a chevron) are not rectangles at all.
    ///
    /// Defaults to a no-op, matching [`PaintContext::fill_text`]: a host with no
    /// path support draws nothing rather than guessing at a bounding box.
    fn fill_path(&mut self, path: &floom_types::Path, color: Color) {
        let _ = (path, color);
    }

    /// Strokes an arbitrary vector path: the outline drawn as a band of
    /// `stroke.width` centred on the path.
    ///
    /// Separate from [`PaintContext::fill_path`] because the two answer
    /// different questions — a fill is "what is inside this shape", a stroke is
    /// "trace this shape" — and a caller that wants both calls both, in that
    /// order. Defaults to a no-op.
    fn stroke_path(&mut self, path: &floom_types::Path, stroke: &floom_types::Stroke) {
        let _ = (path, stroke);
    }

    /// Fills an arbitrary vector path with a [`Brush`].
    ///
    /// The same trade as [`PaintContext::fill_rect_brush`], and the one that
    /// lets a self-drawn chart paint its area with a ramp rather than a flat
    /// colour. Defaults to the representative colour.
    fn fill_path_brush(&mut self, path: &floom_types::Path, brush: &Brush) {
        self.fill_path(path, brush.sample(path.bounds().center()));
    }

    /// An already-registered image resource drawn into `bounds`.
    fn draw_image(&mut self, bounds: Rect, image: ResourceId) {
        let _ = (bounds, image);
    }

    /// This frame's value from the animation running on this element.
    ///
    /// The other half of [`crate::Widget::animation`]: a widget whose animation
    /// drives no compositor channel is handed the sampled scalar here and draws
    /// its own motion with it. That is what a control animating *part* of itself
    /// needs — a progress bar's sweep, a caret's blink — where the alternative
    /// (translating the whole element) would move the thing the motion is
    /// supposed to happen inside.
    ///
    /// `None` covers both "nothing is animating this element" and "a host that
    /// has no animation at all", which a drawing routine should treat the same
    /// way: as the settled state, not as a reason to draw nothing.
    fn animation_value(&self) -> Option<f64> {
        None
    }
}

/// Accessibility / semantics emission context.
pub trait SemanticsContext {
    /// Bounds of the element semantics are being emitted for.
    fn bounds(&self) -> Rect;
    fn set_label(&mut self, label: &str);
    fn set_bounds(&mut self, bounds: Rect);
    fn size_hint(&self) -> Size;
    /// Declares what kind of thing this element is.
    ///
    /// Without this a drawn element can only say *what it is called*, which
    /// leaves a self-drawn canvas indistinguishable from decorative noise: the
    /// role is what tells a screen reader that there is an image here to
    /// describe at all. Compose's canvas and Flutter's `CustomPaint` both
    /// defaulted to invisible for exactly this reason, and both now require the
    /// author to attach semantics explicitly — so the default here is
    /// [`SemanticsRole::None`], i.e. publish nothing, and a widget that draws
    /// something worth describing says so.
    fn set_role(&mut self, role: crate::SemanticsRole) {
        let _ = role;
    }
}
