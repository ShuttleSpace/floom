//! Declarative widget description contract.

use crate::context::{
    CreateContext, EventContext, PaintContext, SemanticsContext, UpdateContext,
};
use floem_types::ElementId;

/// High-level UI description. Implementations must not depend on concrete renderers.
pub trait Widget {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId;
    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId);
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
