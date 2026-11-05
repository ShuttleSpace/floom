// SPDX-License-Identifier: MIT
//! A widget that mounts a [`TextDocument`] into the widget tree.
//!
//! It bridges [`floom-editor`]'s model (transactions, selection, undo) with
//! Floom 2.0's [`Widget`] contract: it owns a document and an undo manager,
//! paints its text line by line, and turns key / pointer events into document
//! mutations.
//!
//! ## What it does today
//!
//! - **Paint**: lays the document out with [`TextLayout`] at the widget's box
//!   width and paints each line, the selection highlight, and the caret.
//! - **Keyboard**: printable characters insert at the selection; `Backspace`
//!   and `Delete` remove; `Enter` inserts a newline; `Mod+Z` / `Mod+Y` undo
//!   and redo.
//! - **Pointer**: a primary click focuses the editor and moves the caret to the
//!   end of the document. Per-glyph hit testing is a follow-up.
//! - **IME**: a composition in progress is drawn at the caret in the accent ink
//!   and is not part of the document until the OS commits it (the commit
//!   arrives as character keys and replaces the preedit).
//!
//! ## What it deliberately does not do
//!
//! - Multi-caret, rectangular selection, and rich styles are out of scope.
//!
//! [`TextShaper`]: floom_text::TextLayout

use std::any::Any;

use crate::{Selection, TextDocument, Transaction, UndoManager};
use floom_core::{
    CreateContext, ElementId, EventContext, FontWeight, LayoutRequest, PaintContext, PaintStyle,
    Point, Rect, SemanticsContext, SemanticsHint, SemanticsRole, UpdateContext, Widget, WidgetAny,
};
use floom_text::{CaretAffinity, TextLayout};
use floom_types::{
    Color, Event, FontFamily, ImeUpdate, Key, KeyPhase, Modifiers, PointerPhase, StyledText,
    TextStyle, TextWrap,
};
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
/// The IME composition is drawn in the accent ink, so an in-progress
/// composition is visibly not part of the document yet.
const COMPOSITION_FILL: Color = Color::rgb(0x2B, 0x6C, 0xB0);

/// A mounted text editor. Owns its document and undo manager; callers drive it
/// through the builders below and the host drives it through [`Widget`].
pub struct EditorWidget {
    document: TextDocument,
    undo: UndoManager,
    font_size: f64,
    color: Color,
    revision: Revision,
    /// The IME composition in progress, drawn after the caret and *not* part of
    /// the document until a commit turns it into character keys.
    preedit: String,
    /// Caret position inside [`Self::preedit`], in bytes.
    preedit_caret: usize,
    /// Whether the page wants the keyboard here as soon as the editor mounts.
    autofocus: bool,
    /// The element the autofocus request was made for. The editor widget is
    /// shared across page rebuilds, so "have I focused yet" is per mount, not
    /// per widget — returning to the page has to take the keyboard again.
    auto_focused_for: Option<ElementId>,
    /// The element this widget was last updated as, so a remount can reset the
    /// focus flag the old element left behind.
    mounted_id: Option<ElementId>,
    /// Mirrors `Widget::set_focused`, so `update` knows when to report the IME
    /// caret area and `paint` can show that the editor has the keyboard.
    focused: bool,
    /// The IME caret area last reported, so an unchanged caret does not reshape
    /// the document every frame just to answer the same rectangle.
    ime_area: Option<Rect>,
    /// Set whenever the caret moved; the next `update` re-reports the area.
    ime_dirty: bool,
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
            font_size: EDITOR_FONT_SIZE,
            color: Color::BLACK,
            revision: Revision::default(),
            preedit: String::new(),
            preedit_caret: 0,
            autofocus: false,
            auto_focused_for: None,
            mounted_id: None,
            focused: false,
            ime_area: None,
            ime_dirty: false,
        }
    }

    /// An editor pre-filled with `text`.
    pub fn from_text(text: &str) -> Self {
        let mut w = Self::new();
        w.document = TextDocument::from_text(text);
        w
    }

    /// Take the keyboard as soon as the editor mounts.
    ///
    /// Off by default: an editor in a form is one of several fields, and the
    /// form decides which one starts focused. A page whose editor *is* the
    /// content — the showcase's `/editor` — turns it on, so typing works
    /// without a click first.
    pub fn autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = autofocus;
        self
    }

    /// The document's current text.
    pub fn text(&self) -> String {
        self.document.text()
    }

    /// The document's current selection.
    pub fn selection(&self) -> Selection {
        self.document.selection()
    }

    /// The IME composition in progress, empty when nothing is composing.
    ///
    /// Deliberately *not* part of [`Self::text`]: a composition is not in the
    /// document until the OS commits it.
    pub fn preedit(&self) -> &str {
        &self.preedit
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
            self.ime_dirty = true;
            self.revision.bump();
        }
    }

    /// Redo the last undo.
    pub fn redo(&mut self) {
        if self.undo.redo(&mut self.document) {
            self.ime_dirty = true;
            self.revision.bump();
        }
    }

    /// Replace the whole document contents.
    pub fn set_text(&mut self, text: &str) {
        self.document = TextDocument::from_text(text);
        self.undo.clear();
        self.preedit.clear();
        self.preedit_caret = 0;
        self.ime_dirty = true;
        self.revision.bump();
    }

    /// Applies a transaction, records its inverse for undo, and bumps the
    /// revision so the host repaints.
    fn commit(&mut self, tx: Transaction) {
        let inverse = self.document.apply(&tx);
        self.undo.push(inverse);
        self.ime_dirty = true;
        self.revision.bump();
    }

    /// Inserts a single character at the selection, replacing it if there is
    /// one.
    fn insert_char(&mut self, ch: char) {
        let sel = self.document.selection();
        self.commit(Transaction::replace(
            sel.start(),
            sel.end(),
            ch.to_string(),
        ));
    }

    /// Inserts a newline at the selection.
    fn insert_newline(&mut self) {
        let sel = self.document.selection();
        self.commit(Transaction::replace(sel.start(), sel.end(), "\n"));
    }

    /// Drops the in-progress composition. Called when the OS finalizes it (the
    /// committed text arrives as character keys) or abandons it.
    fn cancel_preedit(&mut self) {
        if self.preedit.is_empty() && self.preedit_caret == 0 {
            return;
        }
        self.preedit.clear();
        self.preedit_caret = 0;
        self.ime_dirty = true;
        self.revision.bump();
    }

    /// Deletes the selection, or the character before the caret.
    fn backspace(&mut self) {
        self.document.delete_selection_or_backward();
        self.ime_dirty = true;
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

    /// Lays the document out with the editor-grade [`TextLayout`] and returns it.
    ///
    /// The same shaping that produced the pixels also answers the caret and
    /// selection geometry, so the caret sits on the glyphs it is drawn over
    /// rather than near them.
    fn text_layout(&self, content_width: f64) -> TextLayout {
        let styled = StyledText {
            text: self.document.text(),
            base: TextStyle {
                size: self.font_size as f32,
                weight: FontWeight::Normal,
                color: self.color,
            },
            spans: Vec::new(),
            family: FontFamily::SansSerif,
            wrap: TextWrap::Word,
            tab_width: None,
            max_width: Some(content_width as f32),
            line_height: Some(EDITOR_LINE_PITCH as f32),
        };
        TextLayout::new(&styled)
    }

    /// The caret's box, in the editor's own coordinates.
    ///
    /// Shared by paint (to draw it) and `update` (to report the IME area), so
    /// the OS composition popup anchors to the same glyph the user sees. The
    /// rectangle comes from [`TextLayout`] — the same shaping that painted the
    /// text — so the caret is exactly as tall as the line's ink and vertically
    /// aligned with it.
    fn caret_rect(&self, bounds: Rect, content_width: f64) -> Option<Rect> {
        let layout = self.text_layout(content_width);
        let offset = self.document.selection().end();
        let affinity = CaretAffinity::Downstream;
        let point = layout.cursor_point(offset, affinity);
        let metrics = layout.line_metrics_at(offset, affinity)?;
        Some(Rect {
            x0: bounds.x0 + EDITOR_PADDING + point.x,
            x1: bounds.x0 + EDITOR_PADDING + point.x + CARET_WIDTH,
            y0: bounds.y0 + EDITOR_PADDING + point.y - metrics.ascent as f64,
            y1: bounds.y0 + EDITOR_PADDING + point.y + metrics.descent as f64,
        })
    }

    /// Re-reports the IME caret area after a move, so the OS composition popup
    /// follows the caret. Reshaping is only paid for when the caret moved.
    fn sync_ime_area(&mut self, cx: &mut dyn UpdateContext) {
        if !self.ime_dirty {
            return;
        }
        self.ime_dirty = false;
        let area = if self.focused {
            let bounds = cx.bounds();
            if bounds.width() > 0.0 {
                let content_width = (bounds.width() - 2.0 * EDITOR_PADDING).max(0.0);
                self.caret_rect(bounds, content_width)
            } else {
                None
            }
        } else {
            None
        };
        if area != self.ime_area {
            self.ime_area = area;
            cx.set_ime_cursor_area(area);
        }
    }
}

impl Widget for EditorWidget {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        // A page whose editor *is* the content asks for the keyboard on mount:
        // typing then works without a click first. Keyed by element, because a
        // rebuilt page mounts the shared editor under a new id and has to take
        // the keyboard again.
        if self.mounted_id != Some(id) {
            self.mounted_id = Some(id);
            self.focused = false;
        }
        if self.autofocus && self.auto_focused_for != Some(id) {
            self.auto_focused_for = Some(id);
            cx.request_focus(id);
        }
        self.revision.sync(cx, id);
        self.sync_ime_area(cx);
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
        let layout = self.text_layout(600.0);
        let line_count = layout.lines().len().max(1) as f64;
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
        let layout = self.text_layout(content_width);
        if layout.lines().is_empty() {
            return;
        }

        let origin = Point::new(bounds.x0 + EDITOR_PADDING, bounds.y0 + EDITOR_PADDING);
        let sel = self.document.selection();
        let sel_start = sel.start().min(sel.end());
        let sel_end = sel.start().max(sel.end());
        let has_sel = sel_start != sel_end;

        for line in layout.lines() {
            let line_top = origin.y + line.min_coord as f64;
            let line_bottom = origin.y + line.max_coord as f64;

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
                Point::new(origin.x, origin.y + line.baseline as f64),
                line_str,
                self.font_size as f32,
                FontWeight::Normal,
                self.color,
                &FontFamily::SansSerif,
            );
        }

        // Caret: the bar at the selection head, plus a focus ring on
        // the box so it is obvious whether typing will land here.
        let caret = self.caret_rect(bounds, content_width);
        if let Some(caret) = caret
            && sel.is_caret()
        {
            cx.fill_rect(caret, CARET_FILL);
        }
        if self.focused {
            // A 1px focus ring, drawn as four bands along the inside edge: the
            // editor is a large white box, and without a mark it is impossible
            // to tell whether typing will land in it.
            const RING: Color = Color::rgb(0x4C, 0x9A, 0xFF);
            let t = 1.0;
            let i = 1.0;
            for rect in [
                Rect {
                    x0: bounds.x0 + i,
                    y0: bounds.y0 + i,
                    x1: bounds.x1 - i,
                    y1: bounds.y0 + t,
                },
                Rect {
                    x0: bounds.x0 + i,
                    y0: bounds.y1 - t,
                    x1: bounds.x1 - i,
                    y1: bounds.y1 - i,
                },
                Rect {
                    x0: bounds.x0 + i,
                    y0: bounds.y0 + i,
                    x1: bounds.x0 + t,
                    y1: bounds.y1 - i,
                },
                Rect {
                    x0: bounds.x1 - t,
                    y0: bounds.y0 + i,
                    x1: bounds.x1 - i,
                    y1: bounds.y1 - i,
                },
            ] {
                cx.fill_rect(rect, RING);
            }
        }

        // IME composition: the in-progress text is drawn at the caret in the
        // accent ink and underlined, so a composition is visible *before* it is
        // committed. A commit arrives as character keys and replaces it.
        if !self.preedit.is_empty()
            && let Some(caret) = caret {
                let baseline = caret.y1 - (caret.y1 - caret.y0) * 0.2;
                cx.fill_text_in(
                    Point::new(caret.x0, baseline),
                    &self.preedit,
                    self.font_size as f32,
                    FontWeight::Normal,
                    COMPOSITION_FILL,
                    &FontFamily::SansSerif,
                );
                let width = self.preedit.chars().count() as f64 * self.font_size * 0.6;
                cx.fill_rect(
                    Rect {
                        x0: caret.x0,
                        y0: caret.y1 - 1.0,
                        x1: caret.x0 + width,
                        y1: caret.y1,
                    },
                    COMPOSITION_FILL,
                );
                let head = self.preedit_caret.min(self.preedit.len());
                let chars = self.preedit[..head].chars().count() as f64;
                let x = caret.x0 + chars * self.font_size * 0.6;
                cx.fill_rect(
                    Rect {
                        x0: x,
                        y0: caret.y0,
                        x1: x + CARET_WIDTH,
                        y1: caret.y1,
                    },
                    COMPOSITION_FILL,
                );
            }
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        match cx.event() {
            Event::Pointer(pointer) => {
                if pointer.phase == PointerPhase::Down && pointer.is_primary_click() {
                    let len = self.document.len();
                    self.document
                        .set_selection(Selection::caret(len.min(self.document.len())));
                    self.cancel_preedit();
                    self.ime_dirty = true;
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

                // Text input. A character key while a composition is open is
                // the OS finalizing it: the committed text replaces the
                // preedit, so the preedit is dropped first.
                if key.is_text_input()
                    && let Key::Char(ch) = key.key {
                        self.cancel_preedit();
                        self.insert_char(ch);
                        return true;
                    }

                match key.key {
                    Key::Named(floom_types::NamedKey::Enter) => {
                        self.cancel_preedit();
                        self.insert_newline();
                        true
                    }
                    Key::Named(floom_types::NamedKey::Backspace) => {
                        self.cancel_preedit();
                        self.backspace();
                        true
                    }
                    Key::Named(floom_types::NamedKey::Delete) => {
                        self.cancel_preedit();
                        self.delete_forward();
                        true
                    }
                    _ => false,
                }
            }
            Event::Scroll(_) => false,
            // The composition in progress. It is drawn after the caret and is
            // *not* part of the document; a commit arrives as character keys
            // and replaces it (see the key arm above). Ignoring these events is
            // what makes typing under an input method look like a dead widget:
            // the letters go into a composition the editor never showed.
            Event::Ime(update) => match update {
                ImeUpdate::OpenChanged(open) => {
                    if !open {
                        self.cancel_preedit();
                    }
                    true
                }
                ImeUpdate::Preedit { text, caret, .. } => {
                    self.preedit = text;
                    self.preedit_caret = caret;
                    self.ime_dirty = true;
                    self.revision.bump();
                    // The OS popup has to follow the composition *now*: the
                    // runner reads the host's area right after this dispatch.
                    let bounds = cx.bounds();
                    let area = if bounds.width() > 0.0 {
                        let content_width = (bounds.width() - 2.0 * EDITOR_PADDING).max(0.0);
                        self.caret_rect(bounds, content_width)
                    } else {
                        None
                    };
                    cx.set_ime_cursor_area(area);
                    true
                }
            },
        }
    }

    fn set_focused(&mut self, focused: bool) {
        if self.focused != focused {
            self.focused = focused;
            self.ime_dirty = true;
            self.revision.bump();
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
    fn ime_preedit_is_shown_but_not_inserted() {
        use floom_widgets::testkit::TestEventContext;

        let mut w = EditorWidget::from_text("hi");
        let before = w.text();
        let id = floom_core::ElementId::INVALID;
        let bounds = Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 400.0,
            y1: 200.0,
        };
        let mut cx = TestEventContext::new(
            id,
            bounds,
            Event::Ime(ImeUpdate::Preedit {
                text: "ni".into(),
                caret: 2,
                selection: None,
            }),
        );
        assert!(w.event(&mut cx, id), "a preedit is handled");
        assert_eq!(w.preedit(), "ni");
        assert_eq!(w.text(), before, "a composition is not in the document");
        assert!(
            cx.ime_caret.is_some(),
            "the composition caret anchors the OS popup"
        );

        // The commit arrives as a character key: it replaces the preedit.
        let mut cx = TestEventContext::new(
            id,
            bounds,
            Event::Key(floom_types::KeyEvent::down(Key::Char('你'))),
        );
        assert!(w.event(&mut cx, id));
        assert_eq!(w.text(), "hi你");
        assert_eq!(w.preedit(), "", "the commit consumed the composition");
    }

    #[test]
    fn closing_the_composition_drops_the_preedit() {
        use floom_widgets::testkit::TestEventContext;

        let mut w = EditorWidget::from_text("hi");
        let id = floom_core::ElementId::INVALID;
        let bounds = Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 400.0,
            y1: 200.0,
        };
        let mut cx = TestEventContext::new(
            id,
            bounds,
            Event::Ime(ImeUpdate::Preedit {
                text: "n".into(),
                caret: 1,
                selection: None,
            }),
        );
        w.event(&mut cx, id);
        let mut cx = TestEventContext::new(id, bounds, Event::Ime(ImeUpdate::OpenChanged(false)));
        w.event(&mut cx, id);
        assert_eq!(w.preedit(), "");
        assert_eq!(w.text(), "hi");
    }

    #[test]
    fn layout_request_grows_with_lines() {
        let w = EditorWidget::from_text("short");
        let small = w.layout_request().unwrap().height.unwrap().px().unwrap() as f64;

        let w = EditorWidget::from_text("a\nb\nc\nd\ne");
        let tall = w.layout_request().unwrap().height.unwrap().px().unwrap() as f64;
        assert!(tall > small, "more lines means more height");
    }

    /// The caret is the same text the pixels are drawn at — not a taller box
    /// roughly around it. Regression for the first editor, whose caret came from
    /// a fixed line pitch and the line box (including leading) and therefore sat
    /// beside the glyphs rather than on them.
    #[test]
    fn caret_is_aligned_with_the_text_ink() {
        let mut w = EditorWidget::from_text("hello");
        let bounds = Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 600.0,
            y1: 200.0,
        };
        let content_width = bounds.x1 - bounds.x0 - 2.0 * EDITOR_PADDING;

        let at_start = w.caret_rect(bounds, content_width).expect("caret exists");
        w.insert_char('!'); // "hello!"
        let after = w.caret_rect(bounds, content_width).expect("caret exists");

        // A wider document pushes the caret right.
        assert!(after.x0 > at_start.x0, "the caret advances with the text");

        // The caret tracks the font's ink height (ascent + descent, ~ the
        // font size), not the full line box, so it is not taller than the text.
        let height = after.y1 - after.y0;
        assert!(
            height > 10.0 && height < 30.0,
            "caret height tracks the ~16px font, was {height}"
        );

        // The caret straddles the glyph baseline, so it reads as on the text.
        let centre = (after.y0 + after.y1) / 2.0;
        let top = bounds.y0 + EDITOR_PADDING;
        assert!(
            centre > top && centre < top + EDITOR_LINE_PITCH,
            "caret sits on the first line's text, centre at {centre}"
        );
    }
}
