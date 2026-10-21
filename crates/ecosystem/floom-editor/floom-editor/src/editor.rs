//! The editor widget.
//!
//! ## What the widget is
//!
//! A leaf widget that paints the document itself: the gutter, the text, the
//! selection and the caret. It has no children, because every part of it is
//! drawn from the same layout and the same state — a gutter as a child would
//! have to be told the same thing the text is told, and the two would drift.
//!
//! ## Where the geometry comes from
//!
//! Every question the editor asks about the text — which byte is under the
//! pointer, where is the caret, what does the selection cover — is answered by
//! the engine's [`TextLayout`], from *one* shaping per line. The widget lays
//! the line out and the host paints it, and the two agree because both are
//! given the same description: the widget's [`StyledText`] and the host's are
//! the same value, so the glyphs the caret was measured against are the
//! glyphs that get painted.
//!
//! ## The line model
//!
//! One line of the document is one line on screen. There is no soft wrap in
//! this port: the engine can wrap ([`floom_types::TextWrap`]), and the engine
//! *does* — but a wrapped line is several lines on screen, and the scroll
//! position, the line numbers and the "which line is the caret on" question
//! all have to agree about that. This port answers them for the unwrapped
//! case only, and the gap is named rather than faked.
//!
//! The consequence is visible: a long line runs past the right edge instead
//! of wrapping. Everything else about the line — its height, its baseline,
//! where the caret sits on it — is the engine's answer, not an estimate.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::{Arc, Mutex};

use floom_core::{
    CreateContext, ElementId, Event, EventContext, Key, KeyPhase, LayoutRequest, Modifiers,
    NamedKey, PaintContext, Point, Rect, SemanticsContext, SemanticsRole, UpdateContext, Widget,
    WidgetAny,
};
use floom_editor_core::buffer::rope_text::RopeText;
use floom_editor_core::cursor::CursorAffinity;
use floom_editor_core::selection::{SelRegion, Selection};
use floom_text::{CaretAffinity, TextLayout};
use floom_types::{StyledText, TextStyle};
use floom_widgets::primitives::Revision;

use crate::document::Document;
use crate::style::EditorStyle;
use crate::styling::Styling;

/// The editor's mutable state, shared with whoever holds a handle.
///
/// The state is behind a mutex rather than a signal because 2.0's slots are
/// `Send + Sync` and a signal is `Rc`-based: the same move `floom-widgets`'s
/// text input makes for its buffer.
pub struct EditorState {
    /// The document being edited.
    pub doc: Arc<dyn Document>,
    /// The selection: `start` is the anchor and `end` is the cursor.
    ///
    /// The anchor is where the selection began and the cursor is where it
    /// ends, so a selection that was dragged backwards keeps both: the range
    /// is [`SelRegion::min`]..[`SelRegion::max`] and the cursor is `end`.
    pub sel: SelRegion,
    /// The vertical scroll offset, in pixels from the top of the text.
    pub scroll: f64,
    /// A counter that changes whenever anything in here changes.
    ///
    /// The widget watches it to notice a change made through a handle rather
    /// than through the widget, which is the one change it cannot see from
    /// the inside.
    pub generation: u64,
}

impl EditorState {
    /// The caret's position: a byte offset into the document.
    pub fn cursor(&self) -> usize {
        self.sel.end
    }

    /// The selection's range: a byte range into the document.
    pub fn range(&self) -> Range<usize> {
        self.sel.min()..self.sel.max()
    }

    /// Whether anything is selected.
    pub fn has_selection(&self) -> bool {
        self.sel.min() != self.sel.max()
    }

    /// Moves the caret, keeping the anchor where it is.
    pub fn move_cursor(&mut self, offset: usize) {
        self.sel.end = offset;
        self.generation += 1;
    }

    /// Moves the caret and the anchor: the selection collapses.
    pub fn set_caret(&mut self, offset: usize) {
        self.sel.start = offset;
        self.sel.end = offset;
        self.generation += 1;
    }

    /// Extends the selection to `offset`, keeping the anchor where it is.
    pub fn extend_to(&mut self, offset: usize) {
        self.sel.end = offset;
        self.generation += 1;
    }

    /// Scrolls by `delta` pixels, never past the ends of the document.
    pub fn scroll_by(&mut self, delta: f64, max_scroll: f64) {
        let next = (self.scroll + delta).clamp(0.0, max_scroll.max(0.0));
        if next != self.scroll {
            self.scroll = next;
            self.generation += 1;
        }
    }
}

/// One laid-out line of the document.
#[derive(Clone)]
struct LineLayout {
    /// The description that was laid out — and the one that is painted.
    ///
    /// The same value goes to the host's paint op, which is what makes the
    /// painted glyphs the ones the geometry was measured against.
    styled: StyledText,
    /// The shaped result, for the geometry.
    layout: TextLayout,
    /// The byte range of the line in the document, without its line ending.
    range: Range<usize>,
}

/// The laid-out lines, by source line.
///
/// The cache is keyed on the document's revision and on the width the lines
/// were laid out at, because a line laid out at one revision and asked for at
/// another is a line whose glyphs do not match its text, and a line laid out
/// at one width and asked for at another is a line that wraps differently.
#[derive(Default, Clone)]
struct LineCache {
    revision: u64,
    width: f64,
    entries: HashMap<usize, Arc<LineLayout>>,
}

/// The editor: a document, a caret, and the ink between them.
///
/// The widget is `Clone` and `Send + Sync` because a 2.0 child slot's builder
/// is a `Fn() -> Box<dyn WidgetAny> + Send + Sync`: the editor is built once
/// per slot and has to be able to cross that boundary. The clone shares the
/// state — the document, the caret, the scroll — and starts with an empty
/// layout cache, which is a cache and is rebuilt anyway.
pub struct Editor {
    state: Arc<Mutex<EditorState>>,
    style: EditorStyle,
    styling: Arc<dyn Styling>,
    /// The laid-out lines.
    ///
    /// A mutex rather than a `RefCell` because the widget has to be `Sync`:
    /// a 2.0 child slot's builder is `Send + Sync`, and a `RefCell` is not.
    lines: Mutex<LineCache>,
    /// Whether a pointer is down, so a move extends the selection.
    dragging: bool,
    /// The generation the widget last painted.
    painted_generation: u64,
    /// The revision the widget last painted.
    painted_revision: u64,
    /// Whether the widget has painted at all.
    painted: bool,
    /// The repaint flag, in the house shape.
    revision: Revision,
}

impl Clone for Editor {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            style: self.style.clone(),
            styling: self.styling.clone(),
            lines: Mutex::new(LineCache::default()),
            dragging: false,
            painted_generation: self.painted_generation,
            painted_revision: self.painted_revision,
            painted: self.painted,
            revision: Revision::default(),
        }
    }
}

impl Editor {
    /// An editor over `doc`.
    pub fn new(doc: Arc<dyn Document>, style: EditorStyle, styling: Arc<dyn Styling>) -> Self {
        Self {
            state: Arc::new(Mutex::new(EditorState {
                doc,
                sel: SelRegion::caret(0, CursorAffinity::Forward),
                scroll: 0.0,
                generation: 0,
            })),
            style,
            styling,
            lines: Mutex::new(LineCache::default()),
            dragging: false,
            painted_generation: 0,
            painted_revision: 0,
            painted: false,
            revision: Revision::default(),
        }
    }

    /// A handle to the editor's state.
    ///
    /// The handle is what a caller holds to read the caret, to move it, or to
    /// change the document out from under the widget.
    pub fn state(&self) -> Arc<Mutex<EditorState>> {
        self.state.clone()
    }

    /// The document being edited.
    pub fn document(&self) -> Arc<dyn Document> {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).doc.clone()
    }

    /// Replaces the editor's style.
    pub fn set_style(&mut self, style: EditorStyle) {
        self.style = style;
        self.revision.bump();
    }

    /// Replaces how the text is set.
    pub fn set_styling(&mut self, styling: Arc<dyn Styling>) {
        self.styling = styling;
        // The lines were laid out in the old styling, so they are not the
        // lines the new styling would produce.
        self.lines
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entries
            .clear();
        self.revision.bump();
    }

    /// The height of one line, in logical pixels.
    fn line_height(&self) -> f64 {
        f64::from(self.styling.line_height())
    }

    /// Where the text starts, horizontally: the right edge of the gutter.
    fn text_x(&self, bounds: Rect) -> f64 {
        bounds.x0 + self.style.gutter_width
    }

    /// The width the text is laid out in.
    fn text_width(&self, bounds: Rect) -> f64 {
        (bounds.width() - self.style.gutter_width).max(0.0)
    }

    /// The first line on screen.
    fn first_visible_line(&self, scroll: f64) -> usize {
        let line_height = self.line_height();
        if line_height <= 0.0 {
            return 0;
        }
        (scroll / line_height).floor().max(0.0) as usize
    }

    /// How many lines fit on screen, plus one so a partly-scrolled line at the
    /// bottom is painted.
    fn visible_line_count(&self, bounds: Rect) -> usize {
        let line_height = self.line_height();
        if line_height <= 0.0 {
            return 1;
        }
        (bounds.height() / line_height).ceil() as usize + 1
    }

    /// The top of a line, in absolute coordinates.
    fn line_top(&self, bounds: Rect, line: usize, scroll: f64) -> f64 {
        bounds.y0 + line as f64 * self.line_height() - scroll
    }

    /// The laid-out line, from the cache or by laying it out.
    fn line_layout(&self, line: usize, width: f64) -> Arc<LineLayout> {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let doc = state.doc.clone();
        let revision = doc.revision();
        let mut cache = self.lines.lock().unwrap_or_else(|e| e.into_inner());
        if cache.revision != revision || cache.width != width {
            cache.entries.clear();
            cache.revision = revision;
            cache.width = width;
        }
        if let Some(entry) = cache.entries.get(&line) {
            return entry.clone();
        }
        let entry = Arc::new(self.build_line_layout(doc.as_ref(), line));
        cache.entries.insert(line, entry.clone());
        entry
    }

    /// Lays out one line of the document.
    fn build_line_layout(&self, doc: &dyn Document, line: usize) -> LineLayout {
        let text = doc.rope_text();
        let start = text.offset_of_line(line);
        let end = text.offset_of_line(line + 1);
        // The line's own text, without the line ending: the ending is not
        // drawn, and a range that included it would put the caret past the
        // end of the line.
        let content = text.slice_to_cow(start..end);
        let content = content.trim_end_matches(['\r', '\n']);
        let base = TextStyle {
            size: self.styling.font_size(),
            weight: self.styling.weight(),
            color: self.style.text_color,
        };
        let spans = self.styling.style_line(line, content, base);
        let styled = StyledText {
            text: content.to_string(),
            base,
            spans,
            family: self.styling.font_family(),
            // No soft wrap: see the module note.
            wrap: floom_types::TextWrap::None,
            tab_width: Some(self.styling.tab_width()),
            max_width: None,
        };
        let layout = TextLayout::new(&styled);
        LineLayout {
            styled,
            layout,
            range: start..start + content.len(),
        }
    }

    /// Scrolls the editor so the caret is on screen.
    ///
    /// The caret is what the user is looking at, so a keystroke that moves it
    /// off the screen has to bring it back: an editor that scrolls the text
    /// away from the caret is an editor that loses the caret.
    fn scroll_caret_into_view(&self, bounds: Rect) {
        let line_height = self.line_height();
        if line_height <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let doc = state.doc.clone();
        let cursor = state.cursor();
        let scroll = state.scroll;
        let text = doc.rope_text();
        let line = text.line_of_offset(cursor) as f64;
        let content_height = (text.last_line() as f64 + 1.0) * line_height;
        let max_scroll = (content_height - bounds.height()).max(0.0);
        let line_top = line * line_height;
        let line_bottom = line_top + line_height;
        let mut next = scroll;
        if line_top < scroll {
            next = line_top;
        } else if line_bottom > scroll + bounds.height() {
            next = line_bottom - bounds.height();
        }
        next = next.clamp(0.0, max_scroll);
        if next != scroll {
            state.scroll = next;
            state.generation += 1;
        }
    }

    /// The byte offset under `point`, in the document.
    ///
    /// `None` when the point is in the gutter, or past the end of the
    /// document: a click in the gutter is not a click in the text.
    fn offset_at(&self, point: Point, bounds: Rect) -> Option<usize> {
        // The state guard is scoped, not held: `line_layout` takes the same
        // lock, and a mutex is not reentrant. Holding it across the call is a
        // deadlock, not a slow path.
        let (scroll, line_height) = {
            let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            (state.scroll, self.line_height())
        };
        if line_height <= 0.0 {
            return None;
        }
        let text_x = self.text_x(bounds);
        if point.x < text_x {
            return None;
        }
        let line = self.first_visible_line(scroll)
            + (((point.y - bounds.y0 + scroll) / line_height).floor().max(0.0) as usize);
        let entry = self.line_layout(line, self.text_width(bounds));
        let top = self.line_top(bounds, line, scroll);
        // The point, in the line's own coordinates.
        let local = Point::new(point.x - text_x, point.y - top);
        entry.layout.hit_test(local)
    }

    /// Moves the caret and, when the shift key is down, extends the
    /// selection to it.
    fn move_cursor(&self, offset: usize, extend: bool) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if extend {
            state.extend_to(offset);
        } else {
            state.set_caret(offset);
        }
    }

    /// Replaces the selection with `content` and puts the caret after it.
    fn insert(&self, content: &str) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let doc = state.doc.clone();
        let selection = Selection::sel_region(state.sel);
        doc.edit(&selection, content);
        let start = state.sel.min();
        let after = start + content.len();
        state.set_caret(after);
    }

    /// Deletes the selection, if there is one.
    ///
    /// Reports whether anything was deleted, so a caller knows whether to
    /// delete a character as well.
    fn delete_selection(&self) -> bool {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !state.has_selection() {
            return false;
        }
        let doc = state.doc.clone();
        let selection = Selection::sel_region(state.sel);
        doc.edit(&selection, "");
        let at = state.sel.min();
        state.set_caret(at);
        true
    }

    /// The offset of the previous character boundary before `offset`.
    fn previous_boundary(&self, offset: usize) -> usize {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let doc = state.doc.clone();
        let text = doc.rope_text();
        if offset == 0 {
            return 0;
        }
        text.prev_grapheme_offset(offset, 1, 0)
    }

    /// The offset of the next character boundary after `offset`.
    fn next_boundary(&self, offset: usize) -> usize {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let doc = state.doc.clone();
        let text = doc.rope_text();
        let len = text.len();
        if offset >= len {
            return len;
        }
        text.next_grapheme_offset(offset, 1, len)
    }
}

impl Widget for Editor {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        // A change made through the state handle — a caller setting the text,
        // say — is not visible from inside the widget, so the generation is
        // what notices it.
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let generation = state.generation;
        let revision = state.doc.revision();
        drop(state);
        if !self.painted || generation != self.painted_generation || revision != self.painted_revision
        {
            self.painted_generation = generation;
            self.painted_revision = revision;
            self.revision.bump();
        }
        self.revision.sync(cx, id);
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(
            LayoutRequest::new()
                .fill()
                .min_width(200.0)
                .min_height(self.styling.line_height() * 4.0),
        )
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        match cx.event() {
            Event::Pointer(pointer) => self.pointer_event(cx, id, pointer),
            Event::Key(key) => self.key_event(cx, key),
            Event::Scroll(scroll) => {
                let bounds = cx.bounds();
                // The wheel's sign is the framework's: a positive delta is a
                // scroll *down*, and the offset grows as the content moves up.
                // The same convention `floom-widgets`' scroller uses.
                self.scroll_by(scroll.delta_y, bounds);
                true
            }
            Event::Ime(_) => false,
        }
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let doc = state.doc.clone();
        let scroll = state.scroll;
        let selection = state.sel;
        let cursor = state.cursor();
        drop(state);

        let text = doc.rope_text();
        let line_height = self.line_height();
        let text_x = self.text_x(bounds);
        let text_width = self.text_width(bounds);
        let first = self.first_visible_line(scroll);
        let count = self.visible_line_count(bounds);
        let cursor_line = text.line_of_offset(cursor);

        // The background, and the gutter's own.
        cx.fill_rect(bounds, self.style.background);
        cx.fill_rect(
            Rect {
                x0: bounds.x0,
                y0: bounds.y0,
                x1: bounds.x0 + self.style.gutter_width,
                y1: bounds.y1,
            },
            self.style.gutter_background,
        );

        for index in 0..count {
            let line = first + index;
            if line > text.last_line() {
                break;
            }
            let entry = self.line_layout(line, text_width);
            let top = self.line_top(bounds, line, scroll);

            // The current line's band, under everything else.
            if line == cursor_line {
                cx.fill_rect(
                    Rect {
                        x0: text_x,
                        y0: top,
                        x1: bounds.x1,
                        y1: top + line_height,
                    },
                    self.style.current_line_color,
                );
            }

            // The selection, under the text.
            let range = selection.min()..selection.max();
            if range.start != range.end {
                let start = range.start.max(entry.range.start);
                let end = range.end.min(entry.range.end);
                if start < end {
                    for rect in entry.layout.selection_rects(start..end) {
                        cx.fill_rect(
                            Rect {
                                x0: rect.x0 + text_x,
                                y0: rect.y0 + top,
                                x1: rect.x1 + text_x,
                                y1: rect.y1 + top,
                            },
                            self.style.selection_color,
                        );
                    }
                }
            }

            // The text.
            cx.fill_styled_text(Point::new(text_x, top), &entry.styled);

            // The caret, on the line it is on.
            if line == cursor_line {
                if let Some(rect) = entry
                    .layout
                    .cursor_rect(cursor, CaretAffinity::Downstream, self.style.caret_width)
                {
                    cx.fill_rect(
                        Rect {
                            x0: rect.x0 + text_x,
                            y0: rect.y0 + top,
                            x1: rect.x1 + text_x,
                            y1: rect.y1 + top,
                        },
                        self.style.cursor_color,
                    );
                }
            }

            // The gutter's line number.
            let number = (line + 1).to_string();
            let ink = if line == cursor_line {
                self.style.gutter_accent_color
            } else {
                self.style.gutter_dim_color
            };
            let width = f64::from(self.styling.font_size()) * 0.6 * number.len() as f64;
            cx.fill_text(
                Point::new(
                    bounds.x0 + self.style.gutter_width - self.style.gutter_padding - width,
                    top + (line_height - f64::from(self.styling.font_size())) / 2.0,
                ),
                &number,
                self.styling.font_size(),
                self.styling.weight(),
                ink,
            );
        }
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        cx.set_role(SemanticsRole::TextField);
        cx.set_bounds(cx.bounds());
    }
}

impl Editor {
    /// Scrolls the editor by `delta` pixels.
    fn scroll_by(&self, delta: f64, bounds: Rect) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let doc = state.doc.clone();
        let line_height = self.line_height();
        let text = doc.rope_text();
        let lines = text.last_line() as f64 + 1.0;
        let content_height = lines * line_height;
        let max_scroll = (content_height - bounds.height()).max(0.0);
        state.scroll_by(delta, max_scroll);
    }

    /// The pointer's half of the event handling.
    fn pointer_event(
        &mut self,
        cx: &mut dyn EventContext,
        id: ElementId,
        pointer: floom_core::PointerEvent,
    ) -> bool {
        use floom_core::{PointerButton, PointerPhase};
        let bounds = cx.bounds();
        match pointer.phase {
            PointerPhase::Down => {
                if pointer.button != Some(PointerButton::Primary) {
                    return false;
                }
                cx.request_focus(id);
                let Some(offset) = self.offset_at(pointer.position, bounds) else {
                    return false;
                };
                let extend = pointer.modifiers.contains(Modifiers::SHIFT);
                if pointer.click_count >= 3 {
                    self.select_line(offset);
                } else if pointer.click_count == 2 {
                    self.select_word(offset);
                } else if extend {
                    self.move_cursor(offset, true);
                } else {
                    self.move_cursor(offset, false);
                }
                self.dragging = true;
                self.scroll_caret_into_view(bounds);
                true
            }
            PointerPhase::Move => {
                if !self.dragging {
                    return false;
                }
                let Some(offset) = self.offset_at(pointer.position, bounds) else {
                    return false;
                };
                self.move_cursor(offset, true);
                true
            }
            PointerPhase::Up => {
                if !self.dragging {
                    return false;
                }
                self.dragging = false;
                true
            }
            PointerPhase::Enter | PointerPhase::Leave | PointerPhase::Cancel => false,
        }
    }

    /// Selects the word under `offset`.
    fn select_word(&self, offset: usize) {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let doc = state.doc.clone();
        let text = doc.rope_text();
        let (start, end) = text.select_word(offset);
        drop(state);
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.sel = SelRegion::new(start, end, CursorAffinity::Forward, None);
        state.generation += 1;
    }

    /// Selects the line under `offset`.
    fn select_line(&self, offset: usize) {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let doc = state.doc.clone();
        let text = doc.rope_text();
        let line = text.line_of_offset(offset);
        let start = text.offset_of_line(line);
        let end = text.line_end_offset(line, true);
        drop(state);
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.sel = SelRegion::new(start, end, CursorAffinity::Forward, None);
        state.generation += 1;
    }

    /// The keyboard's half of the event handling.
    fn key_event(&mut self, cx: &mut dyn EventContext, key: floom_core::KeyEvent) -> bool {
        if key.phase == KeyPhase::Up {
            return false;
        }
        let extend = key.modifiers.contains(Modifiers::SHIFT);
        let word = key.modifiers.contains(Modifiers::CONTROL)
            || key.modifiers.contains(Modifiers::META);
        match key.key {
            Key::Char(c) => {
                if word {
                    return false;
                }
                self.delete_selection();
                // The key is normalized to lower case and the case lives in the
                // shift modifier, so a shifted letter has to be shifted back
                // here — otherwise the editor types "abc" where the user asked
                // for "AbC".
                let c = if key.modifiers.contains(Modifiers::SHIFT) {
                    c.to_ascii_uppercase()
                } else {
                    c
                };
                self.insert(&c.to_string());
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::Backspace) => {
                if self.delete_selection() {
                    return true;
                }
                let cursor = self.cursor_offset();
                let start = if word {
                    self.word_backward(cursor)
                } else {
                    self.previous_boundary(cursor)
                };
                if start < cursor {
                    let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                    state.sel = SelRegion::new(start, cursor, CursorAffinity::Forward, None);
                    state.generation += 1;
                    drop(state);
                    self.delete_selection();
                }
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::Delete) => {
                if self.delete_selection() {
                    return true;
                }
                let cursor = self.cursor_offset();
                let end = if word {
                    self.word_forward(cursor)
                } else {
                    self.next_boundary(cursor)
                };
                if end > cursor {
                    let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                    state.sel = SelRegion::new(cursor, end, CursorAffinity::Forward, None);
                    state.generation += 1;
                    drop(state);
                    self.delete_selection();
                }
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::ArrowLeft) => {
                let cursor = self.cursor_offset();
                let target = if word {
                    self.word_backward(cursor)
                } else {
                    self.previous_boundary(cursor)
                };
                self.move_cursor(target, extend);
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::ArrowRight) => {
                let cursor = self.cursor_offset();
                let target = if word {
                    self.word_forward(cursor)
                } else {
                    self.next_boundary(cursor)
                };
                self.move_cursor(target, extend);
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::ArrowUp) => {
                let cursor = self.cursor_offset();
                let target = self.line_offset(cursor, -1);
                self.move_cursor(target, extend);
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::ArrowDown) => {
                let cursor = self.cursor_offset();
                let target = self.line_offset(cursor, 1);
                self.move_cursor(target, extend);
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::Home) => {
                let cursor = self.cursor_offset();
                let target = self.line_start(cursor);
                self.move_cursor(target, extend);
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::End) => {
                let cursor = self.cursor_offset();
                let target = self.line_end(cursor);
                self.move_cursor(target, extend);
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::PageUp) | Key::Named(NamedKey::PageDown) => {
                let cursor = self.cursor_offset();
                let lines = if key.key == Key::Named(NamedKey::PageUp) {
                    -10
                } else {
                    10
                };
                let target = self.line_offset(cursor, lines);
                self.move_cursor(target, extend);
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::Enter) => {
                self.delete_selection();
                self.insert("\n");
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::Tab) => {
                self.delete_selection();
                let indent = " ".repeat(self.styling.tab_width());
                self.insert(&indent);
                true
            }
            _ => false,
        }
    }

    /// The caret's offset.
    fn cursor_offset(&self) -> usize {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).cursor()
    }

    /// The start of the line `offset` is on.
    fn line_start(&self, offset: usize) -> usize {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let doc = state.doc.clone();
        drop(state);
        let text = doc.rope_text();
        let line = text.line_of_offset(offset);
        text.offset_of_line(line)
    }

    /// The end of the line `offset` is on, before its line ending.
    fn line_end(&self, offset: usize) -> usize {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let doc = state.doc.clone();
        drop(state);
        let text = doc.rope_text();
        let line = text.line_of_offset(offset);
        // The line ending is not part of the line: a caret at the end of a
        // line sits after the last character, not on the newline.
        text.line_end_offset(line, true)
    }

    /// The offset of the same column on the line `offset` lines away.
    fn line_offset(&self, offset: usize, lines: i64) -> usize {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let doc = state.doc.clone();
        drop(state);
        let text = doc.rope_text();
        let line = text.line_of_offset(offset);
        let column = offset - text.offset_of_line(line);
        let target = (line as i64 + lines).max(0) as usize;
        let start = text.offset_of_line(target);
        let end = text.offset_of_line(target + 1);
        (start + column).min(end)
    }

    /// The start of the word before `offset`.
    fn word_backward(&self, offset: usize) -> usize {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let doc = state.doc.clone();
        drop(state);
        let text = doc.rope_text();
        text.move_n_words_backward(offset, 1, floom_editor_core::mode::Mode::Normal)
    }

    /// The end of the word after `offset`.
    fn word_forward(&self, offset: usize) -> usize {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let doc = state.doc.clone();
        drop(state);
        let text = doc.rope_text();
        text.move_n_words_forward(offset, 1)
    }
}

impl WidgetAny for Editor {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}
