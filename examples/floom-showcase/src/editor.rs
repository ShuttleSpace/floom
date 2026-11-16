// SPDX-License-Identifier: MIT
//! The shared editor widget that outlives a single page mount.
//!
//! Delegates every `Widget` method to a unified `TextEditor` held in `AppState`,
//! so text and selection survive navigating away from `/editor` and back.

use floom::floom_core::{
    CreateContext, ElementId, EventContext, PaintContext, PaintStyle, SemanticsContext,
    SemanticsHint, UpdateContext, Widget, WidgetAny,
};
use floom_editor::TextEditor;

/// Delegates to a `TextEditor` that outlives the page's widget values.
pub struct SharedEditor {
    inner: TextEditor,
}

impl SharedEditor {
    pub fn new(inner: TextEditor) -> Self {
        Self { inner }
    }
}

impl Widget for SharedEditor {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        self.inner.create(cx)
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.inner.update(cx, id);
    }

    fn children(&self) -> Vec<floom::floom_core::Child> {
        self.inner.children()
    }

    fn layout_request(&self) -> Option<floom::floom_core::LayoutRequest> {
        self.inner.layout_request()
    }

    fn paint_style(&self) -> Option<PaintStyle> {
        self.inner.paint_style()
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        self.inner.event(cx, id)
    }

    fn set_focused(&mut self, focused: bool) {
        self.inner.set_focused(focused);
    }

    fn paint(&self, cx: &mut dyn PaintContext, id: ElementId) {
        self.inner.paint(cx, id);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, id: ElementId) {
        self.inner.semantics(cx, id);
    }
}

impl WidgetAny for SharedEditor {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn semantics_hint(&self) -> Option<SemanticsHint> {
        self.inner.semantics_hint()
    }
}
