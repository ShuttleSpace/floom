//! A semantics-only box: a region the evaluation harness addresses by name.
//!
//! [`Labeled`] used to be a `floom-widgets` primitive and then moved up a layer
//! into `floom-ui-composites`, a crate that lives in the excluded satellite
//! workspace at `crates/ecosystem/`. The accuracy loop cannot follow it there:
//! `crates/floom/tests/feature_graph.rs` and
//! `scripts/check-architecture-deps.sh` both forbid the root workspace from
//! reaching into that satellite, and the loop is precisely a root-workspace
//! consumer.
//!
//! So it is declared here instead. The whole widget is a role, a label, one
//! optional child slot, a layout request, and no paint — there is nothing left
//! to re-derive from the kit version, and keeping a local copy is what stops the
//! gallery's labels from silently disappearing.
//!
//! The kit's `Labeled` stays the real widget an app should use; this one exists so
//! the reference screens can keep publishing the labels
//! `tests/gallery.rs` gates on.

use std::any::Any;

use floom_accessibility::SemanticsRole;
use floom_core::{
    Child, CreateContext, ElementId, LayoutRequest, SemanticsContext, SemanticsHint as CoreHint,
    UpdateContext, Widget, WidgetAny,
};
use floom_widgets::primitives::{SemanticsHint, emit_semantics};
use floom_widgets::{to_core_role, to_core_state};

/// A labelled, single-child, paint-free group.
///
/// The engine composes styled regions out of `Container` leaves stacked under
/// their content, so such a region has no widget of its own to publish a name.
/// This one contributes a semantics node whose bounds are the region's bounds
/// and holds one child slot — which is all the evaluation harness needs to find
/// "the card" or "the Save button" in the published tree.
#[derive(Clone)]
pub struct Labeled {
    role: SemanticsRole,
    label: String,
    request: LayoutRequest,
    child: Option<Child>,
}

impl std::fmt::Debug for Labeled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Labeled")
            .field("role", &self.role)
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

impl Labeled {
    /// A labelled group with default sizing.
    pub fn new(role: SemanticsRole, label: impl Into<String>) -> Self {
        Self {
            role,
            label: label.into(),
            request: LayoutRequest::new(),
            child: None,
        }
    }

    /// The layout this box asks for itself (width/height/fill/stack).
    pub fn request(mut self, request: LayoutRequest) -> Self {
        self.request = request;
        self
    }

    /// The single child, with the slot's own layout and a builder.
    pub fn child(
        mut self,
        layout: LayoutRequest,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.child = Some(Child::new("labeled-child", layout, build));
        self
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::labeled(self.role, self.label.clone())
    }
}

impl Widget for Labeled {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {}

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(self.request)
    }

    fn children(&self) -> Vec<Child> {
        match &self.child {
            Some(child) => vec![child.clone()],
            None => Vec::new(),
        }
    }

    fn paint(&self, _cx: &mut dyn floom_core::PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}

impl WidgetAny for Labeled {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<CoreHint> {
        let hint = Labeled::semantics_hint(self);
        Some(CoreHint {
            role: to_core_role(hint.role),
            label: hint.label,
            state: to_core_state(hint.state),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use floom_core::{ChildKey, Length};

    #[test]
    fn a_labelled_box_declares_its_child_and_request() {
        let box_ = Labeled::new(SemanticsRole::Group, "Card")
            .request(LayoutRequest::new().height(120.0))
            .child(LayoutRequest::new().fill(), || {
                Box::new(floom_widgets::Text::new("hello"))
            });
        let children = box_.children();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].key, ChildKey::from("labeled-child"));
        // The child slot was declared `fill()`: grow along the parent axis.
        assert_eq!(children[0].layout.flex_grow, Some(1.0));
        let request = box_.layout_request().expect("the request was set");
        assert_eq!(request.height, Some(Length::Px(120.0)));
    }

    #[test]
    fn the_label_reaches_the_semantics_hint() {
        let hint = Labeled::new(SemanticsRole::Button, "Save").semantics_hint();
        assert_eq!(hint.role, SemanticsRole::Button);
        assert_eq!(hint.label.as_deref(), Some("Save"));
    }
}
