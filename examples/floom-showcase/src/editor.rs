// SPDX-License-Identifier: MIT
//! The shared editor widget that outlives a single page mount.
//!
//! Delegates every `Widget` method to an `EditorWidget` owned by `AppState`,
//! so text and selection survive navigating away from `/editor` and back.

use std::sync::{Arc, Mutex};

use floom::floom_core::{
    CreateContext, ElementId, EventContext, PaintContext, PaintStyle, SemanticsContext,
    SemanticsHint, UpdateContext, Widget, WidgetAny,
};
use floom_editor::EditorWidget;

/// Delegates to an `EditorWidget` that outlives the page's widget values.
pub struct SharedEditor {
    inner: Arc<Mutex<EditorWidget>>,
}

impl SharedEditor {
    pub fn new(inner: Arc<Mutex<EditorWidget>>) -> Self {
        Self { inner }
    }
}

impl Widget for SharedEditor {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        self.inner.lock().unwrap().create(cx)
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.inner.lock().unwrap().update(cx, id);
    }

    fn layout_request(&self) -> Option<floom::floom_core::LayoutRequest> {
        self.inner.lock().unwrap().layout_request()
    }

    fn paint_style(&self) -> Option<PaintStyle> {
        self.inner.lock().unwrap().paint_style()
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        self.inner.lock().unwrap().event(cx, id)
    }

    fn set_focused(&mut self, focused: bool) {
        self.inner.lock().unwrap().set_focused(focused);
    }

    fn paint(&self, cx: &mut dyn PaintContext, id: ElementId) {
        self.inner.lock().unwrap().paint(cx, id);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, id: ElementId) {
        self.inner.lock().unwrap().semantics(cx, id);
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
        self.inner.lock().unwrap().semantics_hint()
    }
}
