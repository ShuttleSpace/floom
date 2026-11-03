
use floom_accessibility::SemanticsRole;
use floom_core::{
    Child, CreateContext, ElementId, LayoutRequest, SemanticsContext, UpdateContext, Widget,
    WidgetAny,
};

/// A labelled single-child group.
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

    pub fn semantics_hint(&self) -> floom_widgets::primitives::SemanticsHint {
        floom_widgets::primitives::SemanticsHint::labeled(self.role, self.label.clone())
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
        floom_widgets::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use floom_core::{Axis, ChildKey, Length};

    #[test]
    fn a_labeled_box_declares_its_child_and_request() {
        let box_ = Labeled::new(SemanticsRole::Group, "Card")
            .request(LayoutRequest::new().height(120.0))
            .child(LayoutRequest::new().fill(), || {
                Box::new(floom_widgets::primitives::Text::new("hello"))
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
        let box_ = Labeled::new(SemanticsRole::Button, "Save");
        let hint = box_.semantics_hint();
        assert_eq!(hint.role, SemanticsRole::Button);
        assert_eq!(hint.label.as_deref(), Some("Save"));
        // The axis import is part of the child-layout vocabulary a caller
        // uses; reference it so the test fails loudly if the re-export moves.
        let _ = Axis::Vertical;
    }
}
