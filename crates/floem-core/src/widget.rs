//! Declarative widget description contract.

use crate::compose::{Child, LayoutRequest};
use crate::context::{CreateContext, EventContext, PaintContext, SemanticsContext, UpdateContext};
use floem_types::ElementId;

/// High-level UI description. Implementations must not depend on concrete renderers.
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
}
