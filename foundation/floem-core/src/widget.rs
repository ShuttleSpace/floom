//! Declarative widget description contract.

use crate::compose::{Child, LayoutRequest};
use crate::context::{CreateContext, EventContext, PaintContext, SemanticsContext, UpdateContext};
use floem_types::ElementId;

/// High-level UI description. Implementations must not depend on concrete renderers.
///
/// The `on_unimplemented` diagnostic is the Phase 29.2 migration hook: the
/// most common cross-version mistake is handing a v1 `floem::view::View`
/// value to a Floem 2.0 mount point. The unsatisfied bound then reports the
/// migration path at the exact line that needs it, instead of a bare
/// "trait bound not satisfied".
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a Floem 2.0 `Widget`",
    label = "not a Floem 2.0 `Widget`",
    note = "v1 `floem::view::View` values are compatibility APIs and cannot be mounted into a Floem 2.0 host",
    note = "rebuild the subtree from `floem2::prelude` widgets, or keep the application on the v1 `floem` crate (preserved for the current major version)"
)]
pub trait Widget {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId;
    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId);
    /// Children to mount underneath this widget's own element.
    ///
    /// Returning an empty list (the default) makes a leaf widget. Composites
    /// rebuild this list on demand and the tree owner diffs it by
    /// [`Child::key`], so only genuinely new slots construct a widget.
    fn children(&self) -> Vec<Child> {
        Vec::new()
    }
    /// Layout intent for *this* node's own box — chiefly how it arranges its
    /// children (flex direction, gap). `None` (the default) means "leaf / no
    /// container behaviour", and the tree owner falls back to whatever the
    /// parent supplied for sizing.
    ///
    /// This is the counterpart to [`Widget::children`]: that says *what* this
    /// widget contains, this says *how* it lays those children out. Keeping the
    /// two apart lets a parent size a child while the child stays in charge of
    /// its own internal flow.
    fn layout_request(&self) -> Option<LayoutRequest> {
        None
    }
    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        let _ = (cx, id);
        false
    }
    fn paint(&self, cx: &mut dyn PaintContext, id: ElementId) {
        let _ = (cx, id);
    }
    fn semantics(&self, cx: &mut dyn SemanticsContext, id: ElementId) {
        let _ = (cx, id);
    }
    /// An overlay this widget asked for is now open, under `handle`.
    ///
    /// The opener needs the handle to close its own overlay — a `Select` whose
    /// header is clicked again, a dialog whose owner is torn down — and it
    /// cannot get it from [`EventContext::open_overlay`], because the overlay
    /// only opens once the event that requested it has finished dispatching.
    /// The content does not need this call: it is built by the request's
    /// content factory, which is handed the handle directly.
    ///
    /// [`EventContext::open_overlay`]: crate::EventContext::open_overlay
    fn overlay_opened(
        &mut self,
        cx: &mut dyn UpdateContext,
        id: ElementId,
        handle: crate::OverlayHandle,
    ) {
        let _ = (cx, id, handle);
    }
    /// An overlay this widget opened has closed; `content` is its content,
    /// handed back so the opener can read whatever it accumulated.
    ///
    /// This is how an overlay reports a result. The content — a popup list, a
    /// dialog's form — owns the outcome because it is the thing the user acted
    /// on, and the opener owns the consequence because it owns the state being
    /// changed. The host pairs them back up: it remembers which widget asked
    /// for each overlay, and when that overlay closes it returns the content
    /// box to the asker.
    ///
    /// Always called, whether the overlay was closed by the content itself, by
    /// the opener, or because the host took it down — a widget that cached a
    /// handle must clear it here or it will believe a stale popup is still up.
    fn overlay_closed(
        &mut self,
        cx: &mut dyn UpdateContext,
        id: ElementId,
        handle: crate::OverlayHandle,
        content: Box<dyn crate::WidgetAny>,
    ) {
        let _ = (cx, id, handle, content);
    }
}
