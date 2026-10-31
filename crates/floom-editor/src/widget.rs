//! A widget that mounts a [`TextDocument`] into the widget tree.
//!
//! It bridges [`floom-editor`]'s model (transactions, selection, undo) with
//! Floom 2.0's [`Widget`] contract: it owns a document and an undo manager,
//! paints its text line by line, and turns key / pointer events into document
//! mutations.
//!
//! ## What it does today
//!
//! - **Paint**: lays the document out with [`TextShaper`] at the widget's box
//!   width and paints each line, the selection highlight, and the caret.
//! - **Keyboard**: printable characters insert at the selection; `Backspace`
//!   and `Delete` remove; `Enter` inserts a newline; `Mod+Z` / `Mod+Y` undo
//!   and redo.
//! - **Pointer**: a primary click focuses the editor and moves the caret to the
//!   end of the document. Per-glyph hit testing is a follow-up.
//!
//! ## What it deliberately does not do
//!
//! - IME preedit is accepted at the document level but not yet rendered as a
//!   separate composition layer.
//! - Multi-caret, rectangular selection, and rich styles are out of scope.
//!
//! [`TextShaper`]: floom_text::TextShaper

use std::any::Any;
use std::sync::Mutex;

use crate::{Selection, TextDocument, Transaction, UndoManager};
use floom_core::{
    CreateContext, ElementId, EventContext, FontWeight, LayoutRequest, PaintContext, PaintStyle,
    Point, Rect, SemanticsContext, SemanticsHint, SemanticsRole, UpdateContext, Widget, WidgetAny,
};
use floom_text::{LaidOutText, TextAlign, TextLayoutOptions, TextShaper};
use floom_types::{Color, Event, FontFamily, Key, KeyPhase, Modifiers, PointerPhase};
use floom_widgets::primitives::Revision;

const EDITOR_FONT_SIZE: f64 = 16.0;
const EDITOR_LINE_PITCH: f64 = 22.0;
const EDITOR_PADDING: f64 = 10.0;
const CARET_WIDTH: f64 = 2.0;
const SELECTION_ALPHA: u8 = 76; // ~0.30

/// Selection highlight colour, derived from the Floom blue base with alpha so
/// the text underneath stays legible.
const SELECTION_FILL: Color = Color::rgba(0x2B, 0x6C, 0xB0, SELECTION_ALPHA);
/// Caret colour.
const CARET_FILL: Color = Color::BLACK;

/// A mounted text editor. Owns its document and undo manager; callers drive it
/// through the builders below and the host drives it through [`Widget`].
pub struct EditorWidget {
    document: TextDocument,
    undo: UndoManager,
    // `paint` and `layout_request` are `&self`, but shaping needs a mutable
    // `TextShaper` (it owns the Parley font/layout contexts). A `Mutex` (not a
    // `RefCell`) gives interior mutability that is `Sync`, so the widget can be
    // mounted through the cross-thread `Fn` child builders the host uses.
    shaper: Mutex<TextShaper>,
    font_size: f64,
    color: Color,
    revision: Revision,
}

impl Default for EditorWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorWidget {
    /// An empty editor.
    pub fn new() -> Self {
        Self {
            document: TextDocument::new(),
            undo: UndoManager::new(),
            shaper: Mutex::new(TextShaper::new()),
            font_size: EDITOR_FONT_SIZE,
            color: Color::BLACK,
            revision: Revision::default(),
        }
    }

    /// An editor pre-filled with `text`.
    pub fn from_text(text: &str) -> Self {
        let mut w = Self::new();
        w.document = TextDocument::from_text(text);
        w
    }

    /// The document's current text.
    pub fn text(&self) -> String {
        self.document.text()
    }

    /// The document's current selection.
    pub fn selection(&self) -> Selection {
        self.document.selection()
    }

    /// Whether the editor has an undoable edit.
    pub fn can_undo(&self) -> bool {
        self.undo.can_undo()
    }

    /// Whether the editor has a redoable edit.
    pub fn can_redo(&self) -> bool {
        self.undo.can_redo()
    }

    /// Undo the last commit.
    pub fn undo(&mut self) {
        if self.undo.undo(&mut self.document) {
            self.revision.bump();
        }
    }

    /// Redo the last undo.
    pub fn redo(&mut self) {
        if self.undo.redo(&mut self.document) {
            self.revision.bump();
        }
    }

    /// Replace the whole document contents.
    pub fn set_text(&mut self, text: &str) {
        self.document = TextDocument::from_text(text);
        self.undo.clear();
        self.revision.bump();
    }

    /// Applies a transaction, records its inverse for undo, and bumps the
    /// revision so the host repaints.
    fn commit(&mut self, tx: Transaction) {
        let inverse = self.document.apply(&tx);
        self.undo.push(inverse);
        self.revision.bump();
    }

    /// Inserts a single character at the selection, replacing it if there is
    /// one.
    fn insert_char(&mut self, ch: char) {
        let sel = self.document.selection();
        self.commit(Transaction::replace(
            sel.start(),
            sel.end(),
            &ch.to_string(),
        ));
    }

    /// Inserts a newline at the selection.
    fn insert_newline(&mut self) {
        let sel = self.document.selection();
        self.commit(Transaction::replace(sel.start(), sel.end(), "\n"));
    }

    /// Deletes the selection, or the character before the caret.
    fn backspace(&mut self) {
        self.document.delete_selection_or_backward();
        self.revision.bump();
    }

    /// Deletes the selection, or the character after the caret.
    fn delete_forward(&mut self) {
        let sel = self.document.selection();
        if !sel.is_empty() {
            self.commit(Transaction::replace(sel.start(), sel.end(), ""));
        } else if sel.end() < self.document.len() {
            self.commit(Transaction::delete(sel.end(), sel.end() + 1));
        }
    }

    /// Shapes the document at the given box width and returns the laid-out
    /// result. The width is the *content* width (box minus padding).
    fn laid_out(&self, content_width: f64) -> LaidOutText {
        self.shaper.lock().unwrap().shape(
            &self.document.text(),
            &TextLayoutOptions {
                max_width: content_width,
                font_size: self.font_size as f32,
                align: TextAlign::Start,
                weight: FontWeight::Normal,
                family: FontFamily::SansSerif,
            },
        )
    }

    /// The line index that contains the given byte offset, or the last line.
    fn line_for_offset(&self, laid: &LaidOutText, offset: usize) -> usize {
        laid.lines
            .iter()
            .position(|l| offset < l.text_range.end)
            .unwrap_or_else(|| laid.lines.len().saturating_sub(1))
    }
}

impl Widget for EditorWidget {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// The editor wants a content-derived height so the host can size the box.
    /// Width is unbounded (the mount pins it); height is the laid-out text plus
    /// padding.
    fn layout_request(&self) -> Option<LayoutRequest> {
        let text = self.document.text();
        if text.is_empty() {
            return Some(
                LayoutRequest::new().height((EDITOR_FONT_SIZE + 2.0 * EDITOR_PADDING) as f32),
            );
        }
        // Measure at a wide column to learn the natural line count, then pitch.
        let laid = self.laid_out(600.0);
        let line_count = laid.lines.len().max(1) as f64;
        Some(
            LayoutRequest::new()
                .height((line_count * EDITOR_LINE_PITCH + 2.0 * EDITOR_PADDING) as f32),
        )
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }

        let content_width = (bounds.width() - 2.0 * EDITOR_PADDING).max(0.0);
        let laid = self.laid_out(content_width);
        if laid.lines.is_empty() {
            return;
        }

        let origin = Point::new(bounds.x0 + EDITOR_PADDING, bounds.y0 + EDITOR_PADDING);
        let sel = self.document.selection();
        let sel_start = sel.start().min(sel.end());
        let sel_end = sel.start().max(sel.end());
        let has_sel = sel_start != sel_end;

        let mut y = origin.y;
        for line in &laid.lines {
            let line_top = y + line.min_coord as f64;
            let line_bottom = y + line.max_coord as f64;

            // Selection highlight behind this line's intersected range.
            if has_sel {
                let line_start = line.text_range.start;
                let line_end = line.text_range.end;
                let inter_start = sel_start.max(line_start);
                let inter_end = sel_end.min(line_end);
                if inter_start < inter_end {
                    cx.fill_rect(
                        Rect {
                            x0: origin.x,
                            y0: line_top,
                            x1: origin.x + line.width as f64,
                            y1: line_bottom,
                        },
                        SELECTION_FILL,
                    );
                }
            }

            // Paint the line's text.
            let line_str = &self.document.text()[line.text_range.clone()];
            cx.fill_text_in(
                Point::new(origin.x, y + line.baseline as f64),
                line_str,
                self.font_size as f32,
                FontWeight::Normal,
                self.color,
                &FontFamily::SansSerif,
            );

            y += EDITOR_LINE_PITCH;
        }

        // Caret: a thin vertical bar at the selection head.
        if sel.is_caret() {
            let head_line = self.line_for_offset(&laid, sel.end());
            if let Some(line) = laid.lines.get(head_line) {
                let head_in_line = sel.end().saturating_sub(line.text_range.start);
                let char_width = self.font_size * 0.6; // rough em-width estimate
                let x = origin.x + (head_in_line as f64 * char_width).min(line.width as f64);
                let line_top = y - EDITOR_LINE_PITCH + line.min_coord as f64;
                let line_bottom = y - EDITOR_LINE_PITCH + line.max_coord as f64;
                cx.fill_rect(
                    Rect {
                        x0: x,
                        y0: line_top,
                        x1: x + CARET_WIDTH,
                        y1: line_bottom,
                    },
                    CARET_FILL,
                );
            }
        }
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        match cx.event() {
            Event::Pointer(pointer) => {
                if pointer.phase == PointerPhase::Down && pointer.is_primary_click() {
                    let len = self.document.len();
                    self.document
                        .set_selection(Selection::caret(len.min(self.document.len())));
                    self.revision.bump();
                    cx.request_focus(id);
                    true
                } else {
                    false
                }
            }
            Event::Key(key) => {
                if key.phase != KeyPhase::Down && key.phase != KeyPhase::Repeat {
                    return false;
                }

                let is_mod = key.modifiers.contains(Modifiers::CONTROL)
                    || key.modifiers.contains(Modifiers::META);

                // Undo / redo.
                if is_mod {
                    match key.key {
                        Key::Char('z') => {
                            self.undo();
                            return true;
                        }
                        Key::Char('y') => {
                            self.redo();
                            return true;
                        }
                        _ => {}
                    }
                }

                // Text input.
                if key.is_text_input() {
                    if let Key::Char(ch) = key.key {
                        self.insert_char(ch);
                        return true;
                    }
                }

                match key.key {
                    Key::Named(floom_types::NamedKey::Enter) => {
                        self.insert_newline();
                        true
                    }
                    Key::Named(floom_types::NamedKey::Backspace) => {
                        self.backspace();
                        true
                    }
                    Key::Named(floom_types::NamedKey::Delete) => {
                        self.delete_forward();
                        true
                    }
                    _ => false,
                }
            }
            Event::Scroll(_) => false,
            Event::Ime(_) => false,
        }
    }

    fn paint_style(&self) -> Option<PaintStyle> {
        Some(PaintStyle {
            foreground: Some(self.color),
            font_size: Some(self.font_size as f32),
            ..PaintStyle::default()
        })
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        floom_widgets::primitives::emit_semantics(
            floom_widgets::primitives::SemanticsHint::labeled(
                floom_accessibility::SemanticsRole::TextField,
                self.document.text(),
            ),
            cx,
        );
    }
}

impl WidgetAny for EditorWidget {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<SemanticsHint> {
        Some(SemanticsHint::labeled(
            SemanticsRole::TextField,
            self.document.text(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Selection;

    #[test]
    fn insert_and_undo_round_trip() {
        let mut w = EditorWidget::from_text("hello");
        w.insert_char('!');
        assert_eq!(w.text(), "hello!");
        w.undo();
        assert_eq!(w.text(), "hello");
        w.redo();
        assert_eq!(w.text(), "hello!");
    }

    #[test]
    fn backspace_removes_selection_or_previous_char() {
        let mut w = EditorWidget::from_text("abc");
        w.document.set_selection(Selection::range(1, 3));
        w.backspace();
        assert_eq!(w.text(), "a");

        let mut w = EditorWidget::from_text("abc");
        w.document.set_selection(Selection::caret(1));
        w.backspace();
        // Caret sits between 'a' and 'b'; Backspace removes the char before it.
        assert_eq!(w.text(), "bc");
    }

    /// The host mounts children through `Fn() -> Box<dyn WidgetAny> + Send + Sync`
    /// builders, so a page that keeps one editor across rebuilds has to put it
    /// behind `Arc<Mutex<_>>`. That only works if the widget itself is `Send`.
    #[test]
    fn editor_widget_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<EditorWidget>();
    }

    #[test]
    fn layout_request_grows_with_lines() {
        let w = EditorWidget::from_text("short");
        let small = w.layout_request().unwrap().height.unwrap().px().unwrap() as f64;

        let w = EditorWidget::from_text("a\nb\nc\nd\ne");
        let tall = w.layout_request().unwrap().height.unwrap().px().unwrap() as f64;
        assert!(tall > small, "more lines means more height");
    }
}
