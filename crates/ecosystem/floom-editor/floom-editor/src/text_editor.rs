//! The public builder: an editor over a piece of text.
//!
//! v1's `text_editor` returned a `View` and the caller styled it with a
//! `CustomStyle`. 2.0 has no view tree to hand back and no style system to
//! hang a property on, so the builder returns the widget itself and the
//! style is a plain struct the caller fills in — the same answer `floom-ui`
//! settled on.

use std::sync::{Arc, Mutex};

use crate::core::xi_rope::Rope;
use floom_core::{
    CreateContext, ElementId, EventContext, LayoutRequest, PaintContext, SemanticsContext,
    UpdateContext, Widget, WidgetAny,
};

use crate::document::{Document, TextDocument};
use crate::editor::{Editor, EditorState};
use crate::style::EditorStyle;
use crate::styling::{SimpleStyling, Styling};

/// A text editor.
///
/// Built by [`text_editor`]. The builder methods return the editor, so they
/// chain; the accessors hand out the state, so a caller can read the caret or
/// change the text from outside the widget.
#[derive(Clone)]
pub struct TextEditor {
    editor: Editor,
    state: Arc<Mutex<EditorState>>,
}

impl TextEditor {
    /// The editor's state.
    ///
    /// The handle is how a caller reads the caret, moves it, or replaces the
    /// document. It is `Send + Sync` — the same shape as `floom-widgets`'
    /// shared buffer — so it can be captured by a closure the caller builds
    /// elsewhere.
    pub fn state(&self) -> Arc<Mutex<EditorState>> {
        self.state.clone()
    }

    /// The document being edited.
    pub fn document(&self) -> Arc<dyn Document> {
        self.editor.document()
    }

    /// The document's current text.
    pub fn text(&self) -> String {
        self.document().text().to_string()
    }

    /// Replaces the document's text and repaints.
    ///
    /// The caret is left where it was, clamped to the new text's length by
    /// the editor's own geometry, which is what a caller replacing a document
    /// behind an editor expects.
    pub fn set_text(&self, text: impl Into<Rope>) {
        self.editor.document().set_text(text.into());
        self.bump_generation();
    }

    /// Undo the last edit. Reports whether there was one.
    pub fn undo(&self) -> bool {
        self.editor.undo()
    }

    /// Redo the last undone edit. Reports whether there was one.
    pub fn redo(&self) -> bool {
        self.editor.redo()
    }

    fn bump_generation(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.generation += 1;
    }

    /// Takes the keyboard as soon as the editor mounts. Off by default.
    pub fn autofocus(mut self, autofocus: bool) -> Self {
        self.editor.set_autofocus(autofocus);
        self
    }

    /// Sets the editor's appearance.
    pub fn style(mut self, style: EditorStyle) -> Self {
        self.editor.set_style(style);
        self
    }

    /// Sets how the text is set: the size, the face, and the highlighted
    /// ranges.
    pub fn styling(mut self, styling: Arc<dyn Styling>) -> Self {
        self.editor.set_styling(styling);
        self
    }
}

/// An editor over `text`.
///
/// The document is a [`TextDocument`], which is the editor core's rope buffer
/// with an undo history behind it.
pub fn text_editor(text: impl Into<Rope>) -> TextEditor {
    let doc: Arc<dyn Document> = Arc::new(TextDocument::new(text));
    let editor = Editor::new(doc, EditorStyle::default(), Arc::new(SimpleStyling::new()));
    let state = editor.state();
    TextEditor { editor, state }
}

impl Widget for TextEditor {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        self.editor.create(cx)
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.editor.update(cx, id);
    }

    fn children(&self) -> Vec<floom_core::Child> {
        self.editor.children()
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        self.editor.layout_request()
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        self.editor.event(cx, id)
    }

    fn paint(&self, cx: &mut dyn PaintContext, id: ElementId) {
        self.editor.paint(cx, id);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, id: ElementId) {
        self.editor.semantics(cx, id);
    }

    fn set_focused(&mut self, focused: bool) {
        self.editor.set_focused(focused);
    }
}

impl WidgetAny for TextEditor {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        Some(self.editor.accessibility_hint())
    }
}
