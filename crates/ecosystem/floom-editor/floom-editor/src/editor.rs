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
//! A source line is one or more **rows** on screen. With soft wrap off a line
//! is one row; with it on, a long line is several, and the engine decides
//! where the breaks fall — the widget asks the layout how many rows a line
//! took and places them a line height apart.
//!
//! The line height is the same for every row, and it is a number the *widget*
//! knows rather than one it measures: the engine is told the height, so the
//! rows it shapes are exactly that far apart, and the scroll position, the
//! gutter and the "which row is the caret on" question all agree because they
//! are all counting the same thing.
//!
//! The scroll position is anchored on a source line rather than on a pixel
//! offset into the document. A pixel offset would need the height of every
//! line above the viewport to be interpreted, which is a whole document of
//! layout for a number that only ever moves a little at a time; anchoring it
//! on a line means scrolling forward is one line of layout and scrolling
//! backward is one line of layout.
//!
//! ## The selection
//!
//! The selection is one or more regions, and it is the editor core's own
//! [`Selection`] rather than a single region. A selection with one region is
//! an ordinary selection; a selection with several is the editor's multiple
//! cursors — every edit applies to all of them, and every caret is drawn.

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
use floom_types::{StyledText, TextStyle, TextWrap};
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
    /// The selection: one or more regions.
    ///
    /// A region's `start` is the anchor and its `end` is the cursor, so a
    /// selection that was dragged backwards keeps both. The regions are kept
    /// in order from the leftmost to the rightmost, and the *last inserted*
    /// one is the primary — the one the keyboard acts on and the one the
    /// editor draws in its accent ink.
    pub sel: Selection,
    /// The first source line that is on screen.
    pub scroll_line: usize,
    /// How far into [`Self::scroll_line`] the view is scrolled, in pixels.
    ///
    /// The scroll is anchored on a line rather than on a pixel offset into the
    /// document, so that scrolling is a matter of laying out one line rather
    /// than of measuring every line above the viewport.
    pub scroll_offset: f64,
    /// A counter that changes whenever anything in here changes.
    ///
    /// The widget watches it to notice a change made through a handle rather
    /// than through the widget, which is the one change it cannot see from
    /// the inside.
    pub generation: u64,
}

impl EditorState {
    /// The caret's position: a byte offset into the document.
    ///
    /// This is the *primary* caret's position: with several carets, this is
    /// the last one that was placed, and it is the one the keyboard moves.
    pub fn cursor(&self) -> usize {
        self.primary().map(|r| r.end).unwrap_or(0)
    }

    /// The primary region: the one the keyboard acts on.
    pub fn primary(&self) -> Option<SelRegion> {
        self.sel.last_inserted().copied()
    }

    /// The primary region's anchor.
    fn anchor(&self) -> usize {
        self.primary().map(|r| r.start).unwrap_or(0)
    }

    /// The primary selection's range: a byte range into the document.
    pub fn range(&self) -> Range<usize> {
        match self.primary() {
            Some(region) => region.min()..region.max(),
            None => 0..0,
        }
    }

    /// Whether anything is selected.
    pub fn has_selection(&self) -> bool {
        self.sel.regions().iter().any(|region| !region.is_caret())
    }

    /// How many carets the editor has.
    ///
    /// One is an ordinary editor; more than one is the editor's multiple
    /// cursors.
    pub fn caret_count(&self) -> usize {
        self.sel.len()
    }

    /// Moves the caret, keeping the anchor where it is.
    pub fn move_cursor(&mut self, offset: usize) {
        let anchor = self.anchor();
        let affinity = self.sel.get_cursor_affinity();
        self.replace_primary(SelRegion::new(anchor, offset, affinity, None));
        self.generation += 1;
    }

    /// Moves the caret and the anchor: the selection collapses.
    pub fn set_caret(&mut self, offset: usize) {
        self.sel = Selection::caret(offset, CursorAffinity::Forward);
        self.generation += 1;
    }

    /// Extends the selection to `offset`, keeping the anchor where it is.
    pub fn extend_to(&mut self, offset: usize) {
        let anchor = self.anchor();
        self.replace_primary(SelRegion::new(anchor, offset, CursorAffinity::Forward, None));
        self.generation += 1;
    }

    /// Moves every caret to its new offset, collapsing each selection.
    ///
    /// The offsets are the carets' new positions, in the order of the
    /// regions. An arrow key moves *every* caret, because an editor with
    /// several carets that moved only one of them would be an editor with
    /// several carets and a broken one.
    ///
    /// The selections collapse because that is what an arrow key does: a
    /// selection that was there is a thing to be left behind, not dragged
    /// along.
    pub fn move_cursors(&mut self, targets: &[usize]) {
        let regions = self.sel.regions_mut();
        for (region, target) in regions.iter_mut().zip(targets) {
            region.start = *target;
            region.end = *target;
        }
        self.generation += 1;
    }

    /// Extends every selection to its caret's new position.
    ///
    /// The anchors stay where they are, which is the whole point of holding
    /// shift: the selection grows towards the caret rather than jumping.
    pub fn extend_cursors(&mut self, targets: &[usize]) {
        let regions = self.sel.regions_mut();
        for (region, target) in regions.iter_mut().zip(targets) {
            region.end = *target;
        }
        self.generation += 1;
    }

    /// Adds a caret at `offset`, so the editor has more than one.
    ///
    /// A caret that lands on another one is merged into it: two carets in the
    /// same place are one caret with two names.
    pub fn add_cursor(&mut self, offset: usize) {
        self.sel.add_region(SelRegion::caret(offset, CursorAffinity::Forward));
        self.generation += 1;
    }

    /// Replaces the primary region with `region`.
    fn replace_primary(&mut self, region: SelRegion) {
        self.sel.replace_last_inserted_region(region);
    }
}

/// The core's cursor affinity, as the engine's.
///
/// The two vocabularies say the same thing in different words: the core's
/// "forward" is the engine's "downstream". The engine has its own type
/// because a renderer should not have to name the shaper's.
fn engine_affinity(affinity: CursorAffinity) -> CaretAffinity {
    match affinity {
        CursorAffinity::Forward => CaretAffinity::Downstream,
        CursorAffinity::Backward => CaretAffinity::Upstream,
    }
}

/// Where the carets end up after every one of `ranges` was replaced by
/// `content_len` bytes.
///
/// A replaced region leaves a caret after the replacement, and every region
/// *before* it shifts the text by however much longer or shorter its own
/// replacement was — so the second caret is not the first one's plus the
/// content's length, it is that plus the first region's change as well.
fn caret_positions(ranges: &[Range<usize>], content_len: usize) -> Vec<usize> {
    let mut shift = 0usize;
    let mut positions = Vec::with_capacity(ranges.len());
    for range in ranges {
        positions.push(range.start + content_len + shift);
        shift += content_len - (range.end - range.start);
    }
    positions
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
    /// How many rows the line took on screen.
    ///
    /// One when the line does not wrap; several when it does.
    rows: usize,
}

impl LineLayout {
    /// The top of the layout's first row, in the layout's own coordinates.
    ///
    /// The editor places the rows by their distance from this, so a rectangle
    /// the layout reports in layout coordinates becomes a rectangle on screen
    /// by subtracting it.
    fn min_coord(&self) -> f64 {
        self.layout
            .lines()
            .first()
            .map(|line| f64::from(line.min_coord))
            .unwrap_or(0.0)
    }
}

/// The laid-out lines, by source line.
///
/// The cache is keyed on *which* document the lines came from, that document's
/// revision, and the width the lines were laid out at. All three are needed:
/// a line laid out at one revision and asked for at another is a line whose
/// glyphs do not match its text, a line laid out at one width and asked for at
/// another is a line that wraps differently, and a line laid out from a
/// *different* document is a line that is simply the wrong text. The last one
/// is not hypothetical — every new `TextDocument` starts at revision `0`, so
/// swapping the document behind the editor for a freshly opened file compares
/// equal on the number alone. The IDE demo switches tabs exactly that way.
#[derive(Default, Clone)]
struct LineCache {
    /// The document the entries were laid out from.
    ///
    /// Held rather than reduced to a pointer so the comparison is `Arc::ptr_eq`
    /// on two live handles; at most one previous document is kept alive, and
    /// only until the next one is laid out.
    document: Option<Arc<dyn Document>>,
    revision: u64,
    width: f64,
    entries: HashMap<usize, Arc<LineLayout>>,
    /// The height of the whole document, when it has been worked out.
    ///
    /// It is worked out by laying out every line, so it is cached: the
    /// editor only needs it to know where the end of the document is.
    total_height: Option<f64>,
}

impl LineCache {
    /// Whether the entries describe `document` at `revision`, laid out at
    /// `width`.
    fn matches(&self, document: &Arc<dyn Document>, revision: u64, width: f64) -> bool {
        self.revision == revision
            && self.width == width
            && self
                .document
                .as_ref()
                .is_some_and(|cached| Arc::ptr_eq(cached, document))
    }

    /// Adopts a new key, dropping everything laid out under the old one.
    fn adopt(&mut self, document: &Arc<dyn Document>, revision: u64, width: f64) {
        self.entries.clear();
        self.total_height = None;
        self.document = Some(document.clone());
        self.revision = revision;
        self.width = width;
    }
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
                sel: Selection::caret(0, CursorAffinity::Forward),
                scroll_line: 0,
                scroll_offset: 0.0,
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

    /// The height of one row, in logical pixels.
    ///
    /// The engine is told this height when it lays the text out, so the rows
    /// it shapes are exactly this far apart. That is what makes the scroll
    /// position — which counts rows — mean anything.
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

    /// The laid-out line, from the cache or by laying it out.
    fn line_layout(&self, line: usize, width: f64) -> Arc<LineLayout> {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let doc = state.doc.clone();
        let revision = doc.revision();
        drop(state);
        let mut cache = self.lines.lock().unwrap_or_else(|e| e.into_inner());
        if !cache.matches(&doc, revision, width) {
            cache.adopt(&doc, revision, width);
        }
        if let Some(entry) = cache.entries.get(&line) {
            return entry.clone();
        }
        let entry = Arc::new(self.build_line_layout(doc.as_ref(), line, width));
        cache.entries.insert(line, entry.clone());
        entry
    }

    /// How many rows a source line took on screen.
    fn line_rows(&self, line: usize, width: f64) -> usize {
        self.line_layout(line, width).rows.max(1)
    }

    /// How tall a source line is on screen, in pixels.
    fn line_height_px(&self, line: usize, width: f64) -> f64 {
        self.line_rows(line, width) as f64 * self.line_height()
    }

    /// Lays out one line of the document.
    fn build_line_layout(&self, doc: &dyn Document, line: usize, width: f64) -> LineLayout {
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
        let soft_wrap = self.style.soft_wrap;
        let styled = StyledText {
            text: content.to_string(),
            base,
            spans,
            family: self.styling.font_family(),
            // With soft wrap off the line is as wide as it needs to be and
            // runs past the edge; with it on the engine breaks the line at the
            // width of the text area, and the line becomes several rows.
            wrap: if soft_wrap {
                TextWrap::Word
            } else {
                TextWrap::None
            },
            tab_width: Some(self.styling.tab_width()),
            max_width: if soft_wrap {
                Some(width as f32)
            } else {
                None
            },
            // The engine is told the height, so its rows are exactly this far
            // apart and the scroll position — which counts rows — means
            // something.
            line_height: Some(self.styling.line_height()),
        };
        let layout = TextLayout::new(&styled);
        LineLayout {
            rows: layout.visual_line_count().max(1),
            styled,
            layout,
            range: start..start + content.len(),
        }
    }

    /// Paints the markers that show the whitespace.
    ///
    /// A space is a dot in the middle of its advance and a tab is a line
    /// across its width. The point of both is to make the *width* of the
    /// whitespace visible, because the width is the part nobody can see.
    fn paint_whitespace(
        &self,
        cx: &mut dyn PaintContext,
        entry: &LineLayout,
        text_x: f64,
        top: f64,
        line_height: f64,
    ) {
        if !self.style.show_whitespace {
            return;
        }
        let content = entry.styled.text.as_str();
        for (index, ch) in content.char_indices() {
            if ch != ' ' && ch != '\t' {
                continue;
            }
            let start = entry.layout.cursor_point(index, CaretAffinity::Downstream).x;
            let end = entry
                .layout
                .cursor_point(index + ch.len_utf8(), CaretAffinity::Downstream)
                .x;
            let width = end - start;
            if width <= 0.0 {
                continue;
            }
            let row = entry.layout.row_of_offset(index).unwrap_or(0);
            let middle = top + row as f64 * line_height + line_height / 2.0;
            if ch == ' ' {
                // A dot in the middle of the space.
                let size = 2.0;
                let middle_x = text_x + start + width / 2.0;
                cx.fill_rect(
                    Rect {
                        x0: middle_x - size / 2.0,
                        y0: middle - size / 2.0,
                        x1: middle_x + size / 2.0,
                        y1: middle + size / 2.0,
                    },
                    self.style.whitespace_color,
                );
            } else {
                // A line across the tab, with a tick at its end: the shape a
                // tab has in every editor that draws one.
                cx.fill_rect(
                    Rect {
                        x0: text_x + start + 1.0,
                        y0: middle - 0.5,
                        x1: text_x + end - 2.0,
                        y1: middle + 0.5,
                    },
                    self.style.whitespace_color,
                );
                cx.fill_rect(
                    Rect {
                        x0: text_x + end - 3.0,
                        y0: middle - 3.0,
                        x1: text_x + end - 1.0,
                        y1: middle + 3.0,
                    },
                    self.style.whitespace_color,
                );
            }
        }
    }

    /// Paints the guides that show where the indentation is.
    ///
    /// A guide is a vertical line at each tab stop the indentation reaches,
    /// so the depth of the indentation is readable without counting the
    /// spaces — which is the whole reason a guide exists.
    fn paint_indent_guides(
        &self,
        cx: &mut dyn PaintContext,
        entry: &LineLayout,
        text_x: f64,
        top: f64,
        line_height: f64,
    ) {
        if !self.style.show_indent_guides {
            return;
        }
        let tab_width = self.styling.tab_width().max(1);
        // The tab stops the indentation reaches, as byte offsets into the
        // line: the guide is drawn where the column starts.
        let mut column = 0usize;
        let mut stops: Vec<usize> = Vec::new();
        for (index, ch) in entry.styled.text.char_indices() {
            if ch == ' ' {
                column += 1;
            } else if ch == '\t' {
                column += tab_width - (column % tab_width);
            } else {
                break;
            }
            if column % tab_width == 0 && column > 0 {
                stops.push(index + ch.len_utf8());
            }
        }
        let height = entry.rows as f64 * line_height;
        for offset in stops {
            let x = entry.layout.cursor_point(offset, CaretAffinity::Downstream).x + text_x;
            cx.fill_rect(
                Rect {
                    x0: x - 0.5,
                    y0: top,
                    x1: x + 0.5,
                    y1: top + height,
                },
                self.style.indent_guide_color,
            );
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
        let width = self.text_width(bounds);
        let (mut line, mut offset, cursor) = {
            let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            (state.scroll_line, state.scroll_offset, state.cursor())
        };
        let doc = self.document();
        let text = doc.rope_text();
        let caret_line = text.line_of_offset(cursor);

        if caret_line < line {
            // The caret is above the view: its line goes to the top.
            line = caret_line;
            offset = 0.0;
        } else {
            // Where the caret is, relative to the top of the view.
            let mut y = bounds.y0 - offset;
            for above in line..caret_line {
                y += self.line_height_px(above, width);
            }
            let entry = self.line_layout(caret_line, width);
            let row = entry
                .layout
                .row_of_offset(cursor.saturating_sub(entry.range.start))
                .unwrap_or(0);
            y += row as f64 * line_height;
            if y < bounds.y0 {
                let delta = bounds.y0 - y;
                self.scroll_up(&mut line, &mut offset, delta, width);
            } else if y + line_height > bounds.y1 {
                let delta = y + line_height - bounds.y1;
                let last = text.last_line();
                self.scroll_down(&mut line, &mut offset, delta, width, last, bounds.height());
            }
        }
        self.set_scroll(line, offset);
    }

    /// Scrolls the view up by `delta` pixels.
    ///
    /// The scroll is anchored on a line, so scrolling up means walking back
    /// through the lines above and giving each one its height back. The first
    /// line of the document is the end of the walk: there is nothing above it.
    fn scroll_up(&self, line: &mut usize, offset: &mut f64, delta: f64, width: f64) {
        let mut remaining = delta;
        while remaining > 0.0 {
            if *offset < 0.0 {
                // The line's top is *below* the viewport's top, which is what
                // a negative offset says: the end of the document is showing
                // and the line above it is off the screen. Scrolling up brings
                // that line back.
                let room = -*offset;
                if remaining <= room {
                    *offset += remaining;
                    return;
                }
                remaining -= room;
                *offset = 0.0;
                continue;
            }
            if remaining <= *offset {
                *offset -= remaining;
                return;
            }
            remaining -= *offset;
            *offset = 0.0;
            if *line == 0 {
                return;
            }
            *line -= 1;
            *offset = self.line_height_px(*line, width);
        }
    }

    /// Scrolls the view down by `delta` pixels.
    ///
    /// The last line of the document is the end of the walk, and the scroll
    /// stops there with the last row still on screen: an editor that scrolls
    /// its last line off the bottom has nothing left to show.
    fn scroll_down(
        &self,
        line: &mut usize,
        offset: &mut f64,
        delta: f64,
        width: f64,
        last: usize,
        viewport: f64,
    ) {
        let mut remaining = delta;
        while remaining > 0.0 {
            if *line >= last {
                break;
            }
            let height = self.line_height_px(*line, width);
            let room = height - *offset;
            if remaining <= room {
                *offset += remaining;
                break;
            }
            remaining -= room;
            *line += 1;
            *offset = 0.0;
        }
        self.clamp_to_end(line, offset, width, last, viewport);
    }

    /// Stops the scroll at the end of the document.
    ///
    /// Two cases, and they are different:
    ///
    /// * The document is taller than the viewport, so there is somewhere to
    ///   scroll to. The scroll stops with the *last line's bottom* at the
    ///   viewport's bottom, which means the last line's top is below the
    ///   viewport's top — a negative offset.
    /// * The document is shorter than the viewport, so there is nowhere to
    ///   scroll to, and the scroll goes back to the top. An editor that let
    ///   the user scroll a document that fits off the screen would be an
    ///   editor that could lose the text.
    fn clamp_to_end(
        &self,
        line: &mut usize,
        offset: &mut f64,
        width: f64,
        last: usize,
        viewport: f64,
    ) {
        if *line < last {
            return;
        }
        let total = self.content_height(width);
        if total <= viewport {
            *line = 0;
            *offset = 0.0;
            return;
        }
        *line = last;
        let height = self.line_height_px(last, width);
        let max_offset = height - viewport;
        if *offset > max_offset {
            *offset = max_offset;
        }
    }

    /// The height of the whole document, in pixels.
    ///
    /// Every line is laid out to answer it, which is the one place the editor
    /// pays for a whole document of layout — and it pays it to know where the
    /// end of the document is. The answer is cached until the document
    /// changes.
    fn content_height(&self, width: f64) -> f64 {
        let doc = self.document();
        let revision = doc.revision();
        {
            let cache = self.lines.lock().unwrap_or_else(|e| e.into_inner());
            if cache.matches(&doc, revision, width) {
                if let Some(height) = cache.total_height {
                    return height;
                }
            }
        }
        let text = doc.rope_text();
        let mut height = 0.0;
        for line in 0..=text.last_line() {
            height += self.line_height_px(line, width);
        }
        let mut cache = self.lines.lock().unwrap_or_else(|e| e.into_inner());
        // The heights above were laid out at this key; record the total under
        // the same one, or a later lookup would find it under a stale key.
        if !cache.matches(&doc, revision, width) {
            cache.adopt(&doc, revision, width);
        }
        cache.total_height = Some(height);
        height
    }

    /// Moves the scroll to `line` and `offset`, if it moved.
    fn set_scroll(&self, line: usize, offset: f64) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.scroll_line != line || state.scroll_offset != offset {
            state.scroll_line = line;
            state.scroll_offset = offset;
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
        let (scroll_line, scroll_offset) = {
            let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            (state.scroll_line, state.scroll_offset)
        };
        let line_height = self.line_height();
        if line_height <= 0.0 {
            return None;
        }
        let text_x = self.text_x(bounds);
        if point.x < text_x {
            return None;
        }
        let width = self.text_width(bounds);
        let doc = self.document();
        let text = doc.rope_text();
        let last = text.last_line();

        // The row under the pointer: a source line's rows are stacked a line
        // height apart, and the lines are stacked by however many rows each
        // of them took.
        let mut line = scroll_line;
        let mut top = bounds.y0 - scroll_offset;
        while line < last {
            let height = self.line_height_px(line, width);
            if point.y < top + height {
                break;
            }
            top += height;
            line += 1;
        }
        let entry = self.line_layout(line, width);
        // The point, in the line's own coordinates.
        let local = Point::new(point.x - text_x, point.y - top);
        let offset = entry.layout.hit_test(local)?;
        // The layout knows only its own line, so what it answers is an offset
        // into *that* line; the document's is the same offset plus where the
        // line starts.
        Some(entry.range.start + offset)
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

    /// Replaces every region with `content` and puts a caret after it.
    ///
    /// One edit, not one per region: the document's undo history should hold
    /// a keystroke, not a keystroke per caret.
    ///
    /// The carets are not simply the regions' starts plus the content's
    /// length: every region *before* one shifts the text by however much
    /// longer or shorter its replacement was.
    fn edit_regions(&self, content: &str) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let doc = state.doc.clone();
        let selection = state.sel.clone();
        let ranges: Vec<Range<usize>> = selection
            .regions()
            .iter()
            .map(|region| region.min()..region.max())
            .collect();
        doc.edit(&selection, content);
        let positions = caret_positions(&ranges, content.len());
        let mut sel = Selection::new();
        for position in positions {
            sel.add_region(SelRegion::caret(position, CursorAffinity::Forward));
        }
        state.sel = sel;
        state.generation += 1;
    }

    /// Deletes backwards at every caret.
    ///
    /// A region with a selection loses the selection; a caret loses the
    /// character before it, or the word before it.
    ///
    /// Reports whether anything was deleted.
    fn delete_backward(&self, word: bool) -> bool {
        let regions: Vec<SelRegion> = {
            let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            state.sel.regions().to_vec()
        };
        let mut ranges = Vec::with_capacity(regions.len());
        let mut deleted = false;
        for region in &regions {
            if !region.is_caret() {
                ranges.push(region.min()..region.max());
                deleted = true;
                continue;
            }
            let start = if word {
                self.word_backward(region.end)
            } else {
                self.previous_boundary(region.end)
            };
            if start < region.end {
                ranges.push(start..region.end);
                deleted = true;
            } else {
                ranges.push(region.end..region.end);
            }
        }
        if !deleted {
            return false;
        }
        self.delete_ranges(ranges);
        true
    }

    /// Deletes forwards at every caret.
    ///
    /// Reports whether anything was deleted.
    fn delete_forward(&self, word: bool) -> bool {
        let regions: Vec<SelRegion> = {
            let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            state.sel.regions().to_vec()
        };
        let mut ranges = Vec::with_capacity(regions.len());
        let mut deleted = false;
        for region in &regions {
            if !region.is_caret() {
                ranges.push(region.min()..region.max());
                deleted = true;
                continue;
            }
            let end = if word {
                self.word_forward(region.end)
            } else {
                self.next_boundary(region.end)
            };
            if end > region.end {
                ranges.push(region.end..end);
                deleted = true;
            } else {
                ranges.push(region.end..region.end);
            }
        }
        if !deleted {
            return false;
        }
        self.delete_ranges(ranges);
        true
    }

    /// Deletes each of `ranges` and leaves a caret where each one started.
    fn delete_ranges(&self, ranges: Vec<Range<usize>>) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let doc = state.doc.clone();
        // The regions are deleted as one edit, so the undo history holds one
        // entry for the whole gesture rather than one per caret.
        let mut selection = Selection::new();
        for range in &ranges {
            selection.add_region(SelRegion::new(
                range.start,
                range.end,
                CursorAffinity::Forward,
                None,
            ));
        }
        doc.edit(&selection, "");
        let positions = caret_positions(&ranges, 0);
        let mut sel = Selection::new();
        for position in positions {
            sel.add_region(SelRegion::caret(position, CursorAffinity::Forward));
        }
        state.sel = sel;
        state.generation += 1;
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
        let (doc, scroll_line, scroll_offset, regions, cursor) = {
            let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            (
                state.doc.clone(),
                state.scroll_line,
                state.scroll_offset,
                state.sel.regions().to_vec(),
                state.cursor(),
            )
        };

        let text = doc.rope_text();
        let line_height = self.line_height();
        let text_x = self.text_x(bounds);
        let text_width = self.text_width(bounds);
        let last_line = text.last_line();
        let primary_line = text.line_of_offset(cursor);

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

        // The visible lines, each of which is one or more rows on screen.
        //
        // The scroll is anchored on a source line, and the offset says how
        // far that line's top is *above* the viewport's top — so a negative
        // offset means the line's top is below it, which is what the end of
        // the document looks like. The line above is then still partly on
        // screen, and the walk starts there.
        let mut line = scroll_line;
        let mut top = bounds.y0 - scroll_offset;
        while line > 0 && top > bounds.y0 {
            line -= 1;
            top -= self.line_height_px(line, text_width);
        }
        while line <= last_line && top < bounds.y1 {
            let entry = self.line_layout(line, text_width);
            let rows = entry.rows.max(1);

            // The current line's band, under everything else.
            //
            // The band is the primary caret's *row*: a wrapped line is
            // several rows, and the band follows the caret rather than
            // covering the whole line.
            if line == primary_line {
                let row = entry
                    .layout
                    .row_of_offset(cursor.saturating_sub(entry.range.start))
                    .unwrap_or(0);
                let band_top = top + row as f64 * line_height;
                cx.fill_rect(
                    Rect {
                        x0: text_x,
                        y0: band_top,
                        x1: bounds.x1,
                        y1: band_top + line_height,
                    },
                    self.style.current_line_color,
                );
            }

            // The selection, under the text.
            for region in &regions {
                let start = region.min().max(entry.range.start);
                let end = region.max().min(entry.range.end);
                if start < end {
                    // The layout knows only its own line, so the range is
                    // translated into the line's own coordinates before it is
                    // asked about.
                    for rect in entry.layout.selection_rects(
                        (start - entry.range.start)..(end - entry.range.start),
                    ) {
                        let y = top + (rect.y0 - entry.min_coord());
                        cx.fill_rect(
                            Rect {
                                x0: rect.x0 + text_x,
                                y0: y,
                                x1: rect.x1 + text_x,
                                y1: y + (rect.y1 - rect.y0),
                            },
                            self.style.selection_color,
                        );
                    }
                }
            }

            // The text.
            //
            // The whole line goes to the host in one piece, and the host lays
            // it out with the same description — so its rows land where this
            // layout's rows are, and a wrapped line is wrapped the same way
            // in both.
            cx.fill_styled_text(Point::new(text_x, top), &entry.styled);

            // The carets: one per region, each on its own row.
            for region in &regions {
                let caret = region.end;
                if caret < entry.range.start || caret > entry.range.end {
                    continue;
                }
                let Some(rect) = entry.layout.cursor_rect(
                    caret - entry.range.start,
                    engine_affinity(region.affinity),
                    self.style.caret_width,
                ) else {
                    continue;
                };
                let y = top + (rect.y0 - entry.min_coord());
                cx.fill_rect(
                    Rect {
                        x0: rect.x0 + text_x,
                        y0: y,
                        x1: rect.x1 + text_x,
                        y1: y + (rect.y1 - rect.y0),
                    },
                    self.style.cursor_color,
                );
            }

            // The decorations: the whitespace and the indentation.
            self.paint_whitespace(cx, &entry, text_x, top, line_height);
            self.paint_indent_guides(cx, &entry, text_x, top, line_height);

            // The gutter's line number, on the line's first row.
            let number = (line + 1).to_string();
            let ink = if line == primary_line {
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

            top += rows as f64 * line_height;
            line += 1;
        }
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        cx.set_role(SemanticsRole::TextField);
        cx.set_bounds(cx.bounds());
    }
}

impl Editor {
    /// Scrolls the editor by `delta` pixels.
    ///
    /// The scroll is anchored on a source line, so this walks the lines: down
    /// gives each line's height away, up takes it back. The end of the
    /// document stops the walk with the last row still on screen.
    fn scroll_by(&self, delta: f64, bounds: Rect) {
        if delta == 0.0 {
            return;
        }
        let width = self.text_width(bounds);
        let (mut line, mut offset) = {
            let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            (state.scroll_line, state.scroll_offset)
        };
        let doc = self.document();
        let text = doc.rope_text();
        let last = text.last_line();
        if delta > 0.0 {
            self.scroll_down(&mut line, &mut offset, delta, width, last, bounds.height());
        } else {
            self.scroll_up(&mut line, &mut offset, -delta, width);
        }
        self.set_scroll(line, offset);
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
                } else if pointer.modifiers.contains(Modifiers::ALT) {
                    // Alt and a click *adds* a caret rather than moving the
                    // one that is there, which is how a second cursor is put
                    // where the pointer is.
                    let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                    state.add_cursor(offset);
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
        state.sel = Selection::region(start, end, CursorAffinity::Forward);
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
        state.sel = Selection::region(start, end, CursorAffinity::Forward);
        state.generation += 1;
    }

    /// The keyboard's half of the event handling.
    fn key_event(&mut self, cx: &mut dyn EventContext, key: floom_core::KeyEvent) -> bool {
        if key.phase == KeyPhase::Up {
            return false;
        }
        let extend = key.modifiers.contains(Modifiers::SHIFT);
        let control = key.modifiers.contains(Modifiers::CONTROL);
        let meta = key.modifiers.contains(Modifiers::META);
        let alt = key.modifiers.contains(Modifiers::ALT);
        // Control and alt together add a caret rather than moving one, so the
        // two are told apart before either is read as a word motion.
        let add_caret = control && alt;
        let word = (control || meta) && !add_caret;
        match key.key {
            Key::Char(c) => {
                if word {
                    return false;
                }
                // The key is normalized to lower case and the case lives in the
                // shift modifier, so a shifted letter has to be shifted back
                // here — otherwise the editor types "abc" where the user asked
                // for "AbC".
                let c = if key.modifiers.contains(Modifiers::SHIFT) {
                    c.to_ascii_uppercase()
                } else {
                    c
                };
                // The edit replaces every region's selection, or inserts at
                // every caret — so a keystroke with several carets types at
                // all of them.
                self.edit_regions(&c.to_string());
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::Backspace) => {
                self.delete_backward(word);
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::Delete) => {
                self.delete_forward(word);
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::ArrowLeft) => {
                self.move_cursors(
                    &|offset| {
                        if word {
                            self.word_backward(offset)
                        } else {
                            self.previous_boundary(offset)
                        }
                    },
                    extend,
                );
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::ArrowRight) => {
                self.move_cursors(
                    &|offset| {
                        if word {
                            self.word_forward(offset)
                        } else {
                            self.next_boundary(offset)
                        }
                    },
                    extend,
                );
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::ArrowUp) => {
                if add_caret {
                    self.add_caret(-1);
                } else {
                    self.move_cursors(&|offset| self.line_offset(offset, -1), extend);
                }
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::ArrowDown) => {
                if add_caret {
                    self.add_caret(1);
                } else {
                    self.move_cursors(&|offset| self.line_offset(offset, 1), extend);
                }
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::Home) => {
                self.move_cursors(&|offset| self.line_start(offset), extend);
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::End) => {
                self.move_cursors(&|offset| self.line_end(offset), extend);
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::PageUp) | Key::Named(NamedKey::PageDown) => {
                let lines = if key.key == Key::Named(NamedKey::PageUp) {
                    -self.page_lines(cx.bounds())
                } else {
                    self.page_lines(cx.bounds())
                };
                self.move_cursors(&|offset| self.line_offset(offset, lines), extend);
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::Enter) => {
                self.edit_regions("\n");
                self.scroll_caret_into_view(cx.bounds());
                true
            }
            Key::Named(NamedKey::Tab) => {
                let indent = " ".repeat(self.styling.tab_width());
                self.edit_regions(&indent);
                true
            }
            _ => false,
        }
    }

    /// How many lines a page is, in the viewport.
    fn page_lines(&self, bounds: Rect) -> i64 {
        let line_height = self.line_height();
        if line_height <= 0.0 {
            return 1;
        }
        ((bounds.height() / line_height).floor() as i64).max(1)
    }

    /// Moves every caret by a motion.
    ///
    /// The motion is applied to each caret's own position, so every caret
    /// moves by the same *rule* rather than by the same distance: two carets
    /// on different lines both go up a line and both land on their own
    /// columns.
    fn move_cursors(&self, motion: &dyn Fn(usize) -> usize, extend: bool) {
        let ends: Vec<usize> = {
            let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            state.sel.regions().iter().map(|region| region.end).collect()
        };
        let targets: Vec<usize> = ends.iter().map(|end| motion(*end)).collect();
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if extend {
            state.extend_cursors(&targets);
        } else {
            state.move_cursors(&targets);
        }
    }

    /// Adds a caret on the line above or below the primary one.
    ///
    /// `lines` is how many lines to move: `-1` for above, `1` for below.
    fn add_caret(&self, lines: i64) {
        let target = self.line_offset(self.cursor_offset(), lines);
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.add_cursor(target);
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
        // The line is clamped at both ends: there is no line above the first
        // one and none below the last, and a caret that walked off the
        // document would be a caret nowhere.
        let target = ((line as i64 + lines).max(0) as usize).min(text.last_line());
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
