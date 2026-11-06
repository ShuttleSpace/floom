// SPDX-License-Identifier: MIT
//! Editor-grade text layout: styled ranges, hit testing, cursor and selection
//! geometry, and wrapped-line enumeration.
//!
//! [`TextLayout`] owns the shaped result and is the single source for both
//! geometry and painting. The paint path must paint [`TextLayout::runs`] rather
//! than shaping the string again, or the caret lands where the pixels are not.
//! Design notes and the 1.x history: `docs/api/editor-text-layout.md`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;

use crate::{Glyph, LineMetrics};
use floom_types::{
    Color, FontFamily, FontWeight, Point, Rect, Size, StyledText, TextSpan, TextStyle, TextWrap,
};
use parley::layout::{Alignment, AlignmentOptions, Layout, PositionedLayoutItem};
use parley::style::{
    FontStyle as ParleyFontStyle, LineHeight as ParleyLineHeight, OverflowWrap, StyleProperty,
    TextWrapMode,
};
use parley::{Affinity, Cursor, FontContext, LayoutContext, Selection};
use peniko::FontData;
use swash::scale::ScaleContext;

thread_local! {
    /// The font database the editor layout shapes in.
    ///
    /// Building one enumerates the system fonts, which is far too expensive to
    /// do per layout — and a layout is built on every keystroke, twice (once
    /// for the geometry and once for the paint). One per thread is enough:
    /// the fonts are the same on every thread.
    static FONT_CONTEXT: RefCell<FontContext> = RefCell::new(FontContext::new());
    /// The shaping context, which keeps the resolved faces between layouts.
    static LAYOUT_CONTEXT: RefCell<LayoutContext<TextBrush>> =
        RefCell::new(LayoutContext::new());
}

/// Where the caret lands when it sits on a boundary.
///
/// A byte offset is a *position between characters*, and on a boundary between
/// a Latin character and a combining mark — or between a character and a line
/// break — there are two of them. The affinity says which one the caret takes:
/// downstream is the one after the character, upstream the one before it.
///
/// The type is the engine's own rather than the shaper's, because the shaper's
/// version is the shaper's business: a caller that names it would be reaching
/// past this crate's surface, and a renderer would have to depend on the
/// shaper to answer a question about a caret.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CaretAffinity {
    /// The caret is after the character it is on.
    #[default]
    Downstream,
    /// The caret is before the character it is on.
    Upstream,
}

impl CaretAffinity {
    /// The affinity, as the shaper spells it.
    fn to_shaper(self) -> Affinity {
        match self {
            Self::Downstream => Affinity::Downstream,
            Self::Upstream => Affinity::Upstream,
        }
    }
}

/// The brush a shaped run carries.
///
/// A run's ink is part of the *shape*, not something bolted on afterwards: a
/// syntax-highlighted line is one shaping with several inks, and the run
/// boundaries are the span boundaries. Carrying the ink on the run is what lets
/// the paint path reproduce the layout exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextBrush(pub Color);

// The family → parley lowering lives in `layout::font_stack` and is shared with
// the plain text path, so a named family and the default both end up on the
// same CJK-capable fallback stack. Two copies of that list is how the editor's
// text ends up as tofu boxes on a device the plain text path handles fine.

/// Where a tab expanded to, so a byte offset in the source can be mapped to one
/// in the shaped text and back.
#[derive(Debug, Clone, Default)]
struct TabMap {
    /// `(source byte offset, expanded width)` for each tab, in source order.
    tabs: Vec<(usize, usize)>,
}

impl TabMap {
    /// The source byte offset as an offset into the expanded text.
    fn orig_to_display(&self, pos: usize) -> usize {
        let mut shift = 0usize;
        for &(tab_orig, tab_len) in &self.tabs {
            if tab_orig >= pos {
                break;
            }
            shift += tab_len - 1;
        }
        pos + shift
    }

    /// The offset into the expanded text as a source byte offset.
    fn display_to_orig(&self, pos: usize) -> usize {
        let mut shift = 0usize;
        for &(tab_orig, tab_len) in &self.tabs {
            let tab_display = tab_orig + shift;
            if tab_display >= pos {
                break;
            }
            if pos < tab_display + tab_len {
                return tab_orig;
            }
            shift += tab_len - 1;
        }
        pos - shift
    }
}

/// Expands tabs to spaces, recording where each one went.
///
/// The mapping is `None` when the text has no tabs, which is the common case
/// and means the source and display offsets are the same and no mapping is
/// needed.
fn expand_tabs(text: &str, tab_width: usize) -> (String, Option<TabMap>) {
    if !text.as_bytes().contains(&b'\t') {
        return (text.to_string(), None);
    }
    let mut display = String::with_capacity(text.len());
    let mut tabs = Vec::new();
    let mut col = 0usize;
    for (i, c) in text.char_indices() {
        if c == '\t' {
            let spaces = tab_width - (col % tab_width);
            tabs.push((i, spaces));
            display.extend(std::iter::repeat_n(' ', spaces));
            col += spaces;
        } else {
            display.push(c);
            col = if c == '\n' || c == '\r' { 0 } else { col + 1 };
        }
    }
    (display, Some(TabMap { tabs }))
}

/// A shaped run of text, ready to paint.
///
/// The ink travels with the run, so a line with several colours is one
/// shaping with several inks rather than several shapings.
#[derive(Debug, Clone, PartialEq)]
pub struct StyledRun {
    /// The font the run was shaped in.
    pub font: FontData,
    /// The size the run was shaped at.
    pub font_size: f32,
    /// Where the run starts, relative to the layout's origin.
    pub origin: Point,
    /// The ink the run is drawn in.
    pub color: Color,
    /// The glyphs, with their positions relative to the run's origin.
    pub glyphs: Vec<Glyph>,
}

/// A laid-out, styled text.
///
/// The whole point of this type is that it answers every question about the
/// text from *one* shaping, so the geometry the editor reasons about and the
/// pixels it paints are the same text.
#[derive(Clone)]
pub struct TextLayout {
    /// The source text.
    text: String,
    /// The shaped text, which differs from the source when tabs were expanded.
    display: String,
    /// How source offsets map to display offsets, when tabs were expanded.
    tab_map: Option<TabMap>,
    /// The base style the text was shaped with.
    base: TextStyle,
    /// The ranges that differ from the base style.
    spans: Vec<TextSpan>,
    /// The family the text was shaped in.
    family: FontFamily,
    /// Whether and how the text breaks lines.
    wrap: TextWrap,
    /// The width the text breaks at.
    max_width: Option<f32>,
    /// The pitch between the lines, when the caller asked for one.
    line_height: Option<f32>,
    /// The shaped result.
    layout: Layout<TextBrush>,
    /// The shaped runs, in paint order.
    runs: Vec<StyledRun>,
    /// The visual lines' metrics.
    lines: Vec<LineMetrics>,
    /// The size of the shaped text.
    size: Size,
}

impl std::fmt::Debug for TextLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextLayout")
            .field("text", &self.text)
            .field("size", &self.size)
            .field("lines", &self.lines.len())
            .field("runs", &self.runs.len())
            .finish()
    }
}

impl TextLayout {
    /// Lays out `styled` and answers geometry questions about it.
    ///
    /// The layout is shaped immediately: every question this type answers is
    /// about a specific shaping, and a layout that had to be shaped on first
    /// use would be one whose answers depended on when they were asked.
    pub fn new(styled: &StyledText) -> Self {
        let StyledText {
            text,
            base,
            spans,
            family,
            wrap,
            tab_width,
            max_width,
            line_height,
        } = styled;

        let (display, tab_map) = match tab_width {
            Some(width) if *width > 0 => expand_tabs(text, *width),
            _ => (text.clone(), None),
        };

        let mut layout = Self {
            text: text.clone(),
            display,
            tab_map,
            base: *base,
            spans: spans.clone(),
            family: family.clone(),
            wrap: *wrap,
            max_width: *max_width,
            line_height: *line_height,
            layout: Layout::new(),
            runs: Vec::new(),
            lines: Vec::new(),
            size: Size::ZERO,
        };
        layout.shape();
        layout
    }

    /// The source text this layout was built from.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The style every byte starts with.
    pub fn base_style(&self) -> TextStyle {
        self.base
    }

    /// The typeface the text was shaped in.
    pub fn family(&self) -> &FontFamily {
        &self.family
    }

    /// The size of the shaped text.
    pub fn size(&self) -> Size {
        self.size
    }

    /// The shaped runs, in paint order.
    ///
    /// These are the runs the paint path draws. Handing them over rather than
    /// the source string is what keeps the paint and the geometry in
    /// agreement.
    pub fn runs(&self) -> &[StyledRun] {
        &self.runs
    }

    /// The visual lines' metrics.
    pub fn lines(&self) -> &[LineMetrics] {
        &self.lines
    }

    /// The number of visual lines.
    pub fn visual_line_count(&self) -> usize {
        self.lines.len()
    }

    /// The distance between the layout's rows.
    ///
    /// The pitch is the shaper's line height, and it is the same for every
    /// row: a caller that knows it can place the fourth row without asking
    /// where the third one ended, which is what makes a scroll position
    /// expressible as a number of rows.
    ///
    /// A layout that asked for a line height gets exactly that; one that did
    /// not gets the font's own.
    pub fn row_pitch(&self) -> f64 {
        self.lines
            .first()
            .map(|l| f64::from(l.line_height))
            .unwrap_or(0.0)
    }

    /// The row a source byte offset is on.
    ///
    /// The row is the index of the *wrapped* line the offset is on: a source
    /// line that breaks into three rows has three row indices, and an offset
    /// in its third one answers `2`.
    ///
    /// `None` for an empty layout, which has no rows to speak of — though it
    /// still has one line, so a caller that wants "which row" for an empty
    /// document gets `0` from [`Self::visual_line_count`] and nothing here.
    pub fn row_of_offset(&self, offset: usize) -> Option<usize> {
        if self.layout.is_empty() {
            return None;
        }
        let display = self.orig_to_display(offset.min(self.text.len()));
        let cursor = Cursor::from_byte_index(&self.layout, display, Affinity::Downstream);
        let bbox = cursor.geometry(&self.layout, 0.0);
        Some(self.line_index_for_y(bbox.y0 as f32))
    }

    /// The baseline of visual line `nth`.
    pub fn visual_line_y(&self, nth: usize) -> Option<f32> {
        self.lines.get(nth).map(|l| l.baseline)
    }

    /// The byte range of visual line `index` in the source text.
    pub fn line_text_range(&self, index: usize) -> Option<Range<usize>> {
        self.lines.get(index).map(|l| l.text_range.clone())
    }

    /// The metrics of the visual line containing source byte `idx`.
    pub fn line_metrics_at(&self, idx: usize, affinity: CaretAffinity) -> Option<LineMetrics> {
        if self.layout.is_empty() {
            return None;
        }
        let display = self.orig_to_display(idx);
        let cursor = Cursor::from_byte_index(&self.layout, display, affinity.to_shaper());
        let bbox = cursor.geometry(&self.layout, 0.0);
        let line = self.line_index_for_y(bbox.y0 as f32);
        self.lines.get(line).cloned()
    }

    /// The byte offset under `point`, in the source text.
    ///
    /// The result is clamped to a character boundary, so it is always a
    /// position a caret can sit at. A point past the end of the text is the
    /// end of the text, not an error: a click past the last glyph puts the
    /// caret after it.
    pub fn hit_test(&self, point: Point) -> Option<usize> {
        if self.text.is_empty() {
            return Some(0);
        }
        let cursor = Cursor::from_point(&self.layout, point.x as f32, point.y as f32);
        Some(self.display_to_orig(cursor.index()))
    }

    /// The position of the caret when the cursor is at source byte `offset`.
    ///
    /// The `y` is the *baseline* of the visual line the caret is on, and the
    /// `x` is the left edge of the caret. A caller that wants the caret's
    /// rectangle uses [`Self::cursor_rect`].
    pub fn cursor_point(&self, offset: usize, affinity: CaretAffinity) -> Point {
        // An empty text is still one line, and the caret has to sit on it:
        // an editor with nothing in it still shows a caret.
        if self.layout.is_empty() {
            return Point::ZERO;
        }
        let display = self.orig_to_display(offset);
        let cursor = Cursor::from_byte_index(&self.layout, display, affinity.to_shaper());
        let bbox = cursor.geometry(&self.layout, 0.0);
        let line = self.line_index_for_y(bbox.y0 as f32);
        let baseline = self
            .lines
            .get(line)
            .map(|l| l.baseline as f64)
            .unwrap_or(0.0);
        Point::new(bbox.x0, baseline)
    }

    /// The rectangle the caret occupies when the cursor is at source byte
    /// `offset`.
    ///
    /// The rectangle is the caret's advance — a zero-width box at the cursor's
    /// position, `width` wide — over the full height of the visual line.
    pub fn cursor_rect(&self, offset: usize, affinity: CaretAffinity, width: f32) -> Option<Rect> {
        if self.layout.is_empty() {
            return None;
        }
        let display = self.orig_to_display(offset);
        let cursor = Cursor::from_byte_index(&self.layout, display, affinity.to_shaper());
        let bbox = cursor.geometry(&self.layout, width);
        let line = self.line_index_for_y(bbox.y0 as f32);
        let metrics = self.lines.get(line)?;
        Some(Rect {
            x0: bbox.x0,
            y0: metrics.min_coord as f64,
            x1: bbox.x1,
            y1: metrics.max_coord as f64,
        })
    }

    /// The rectangles the selection covers.
    ///
    /// One rectangle per visual line the selection touches, in source byte
    /// order. A selection that ends at a line break covers that line up to its
    /// end and does not start a rectangle on the next one.
    ///
    /// The rectangles are *ink-tight*: they cover the selected glyphs' outlines
    /// rather than their advances, and they span the visual line's full
    /// height. Both matter because the editor paints the selection *behind*
    /// the text, and a rectangle that does not cover the ink shows the
    /// background through the glyphs.
    pub fn selection_rects(&self, range: Range<usize>) -> Vec<Rect> {
        if range.is_empty() || self.layout.is_empty() {
            return Vec::new();
        }
        let start = self.orig_to_display(range.start.min(self.text.len()));
        let end = self.orig_to_display(range.end.min(self.text.len()));
        if start >= end {
            return Vec::new();
        }
        let anchor = Cursor::from_byte_index(&self.layout, start, Affinity::Downstream);
        let focus = Cursor::from_byte_index(&self.layout, end, Affinity::Upstream);
        let selection = Selection::new(anchor, focus);
        let display_range = start..end;

        let mut rects = Vec::new();
        for (bbox, line_index) in selection.geometry(&self.layout) {
            let (min_x, max_x) = self.ink_extent(line_index, &display_range, bbox.x0, bbox.x1);
            match self.lines.get(line_index) {
                Some(metrics) => rects.push(Rect {
                    x0: min_x,
                    y0: metrics.min_coord as f64,
                    x1: max_x,
                    y1: metrics.max_coord as f64,
                }),
                None => rects.push(Rect {
                    x0: bbox.x0,
                    y0: bbox.y0,
                    x1: bbox.x1,
                    y1: bbox.y1,
                }),
            }
        }
        rects
    }

    /// The top and bottom of the visual line boxes.
    pub fn visual_bounds_y(&self) -> Option<(f32, f32)> {
        if self.lines.is_empty() {
            return None;
        }
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for line in &self.lines {
            min_y = min_y.min(line.min_coord);
            max_y = max_y.max(line.max_coord);
        }
        (min_y.is_finite() && max_y.is_finite()).then_some((min_y, max_y))
    }

    /// The top and bottom of the visual line boxes, ignoring the leading.
    ///
    /// This is the extent a layout is *centred* within: the leading above the
    /// first line and below the last one is not part of the text's height.
    pub fn centering_bounds_y(&self) -> Option<(f32, f32)> {
        if self.lines.is_empty() {
            return None;
        }
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for line in &self.lines {
            min_y = min_y.min(line.baseline - line.ascent);
            max_y = max_y.max(line.baseline);
        }
        (min_y.is_finite() && max_y.is_finite()).then_some((min_y, max_y))
    }

    /// The visual line whose box contains `y`.
    fn line_index_for_y(&self, y: f32) -> usize {
        // Mirrors Parley's own line-for-offset lookup so the cursor's line and
        // the painted line are the same line.
        let count = self.lines.len();
        if count <= 1 {
            return 0;
        }
        if y < 0.0 {
            return 0;
        }
        let mut lo = 0usize;
        let mut hi = count;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let metrics = &self.lines[mid];
            let ordering = if y < metrics.min_coord {
                std::cmp::Ordering::Greater
            } else if y >= metrics.max_coord {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            };
            match ordering {
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Equal => return mid,
            }
        }
        lo.saturating_sub(1).min(count - 1)
    }

    /// A source byte offset as an offset into the shaped text.
    fn orig_to_display(&self, pos: usize) -> usize {
        match &self.tab_map {
            Some(map) => map.orig_to_display(pos),
            None => pos,
        }
    }

    /// An offset into the shaped text as a source byte offset.
    fn display_to_orig(&self, pos: usize) -> usize {
        match &self.tab_map {
            Some(map) => map.display_to_orig(pos),
            None => pos,
        }
    }

    /// The horizontal extent of the selected ink on one visual line.
    ///
    /// Parley's selection geometry is advance-based, so it can be narrower
    /// than the ink of an italic or otherwise overhanging glyph. The
    /// rectangles are painted *behind* the text, so a rectangle that does not
    /// cover the ink shows the background through the glyphs. Unioning in the
    /// glyph outlines is what makes the highlight read as a highlight.
    fn ink_extent(
        &self,
        line_index: usize,
        range: &Range<usize>,
        min_x: f64,
        max_x: f64,
    ) -> (f64, f64) {
        let Some(line) = self.layout.get(line_index) else {
            return (min_x, max_x);
        };
        let mut min_x = min_x;
        let mut max_x = max_x;
        let mut run_offset = line.metrics().offset as f64;
        for run in line.runs() {
            let run_range = run.text_range();
            if run_range.end <= range.start || run_range.start >= range.end {
                run_offset += run.advance() as f64;
                continue;
            }
            let mut cluster_offset = run_offset;
            for cluster in run.visual_clusters() {
                let cluster_range = cluster.text_range();
                let cluster_advance = cluster.advance() as f64;
                if cluster_range.end > range.start && cluster_range.start < range.end {
                    for glyph in cluster.glyphs() {
                        if let Some(bounds) = glyph_outline_bounds(
                            run.font(),
                            run.font_size(),
                            run.normalized_coords(),
                            run.synthesis().skew(),
                            glyph.id,
                        ) {
                            min_x =
                                min_x.min(cluster_offset + glyph.x as f64 + bounds.min.x as f64);
                            max_x =
                                max_x.max(cluster_offset + glyph.x as f64 + bounds.max.x as f64);
                        }
                    }
                }
                cluster_offset += cluster_advance;
            }
            run_offset += run.advance() as f64;
        }
        (min_x, max_x)
    }

    /// Shapes the text and records the runs and line metrics.
    fn shape(&mut self) {
        let display = self.display.clone();
        let layout = {
            FONT_CONTEXT.with(|font_cx| {
                LAYOUT_CONTEXT.with(|layout_cx| {
                    let mut font_cx = font_cx.borrow_mut();
                    let mut layout_cx = layout_cx.borrow_mut();
                    Self::build(&mut layout_cx, &mut font_cx, &display, self)
                })
            })
        };
        self.collect(layout);
    }

    /// Shapes the text, with the spans applied.
    fn build(
        layout_cx: &mut LayoutContext<TextBrush>,
        font_cx: &mut FontContext,
        display: &str,
        styled: &Self,
    ) -> Layout<TextBrush> {
        let mut builder = layout_cx.ranged_builder(font_cx, display, 1.0, true);
        builder.push_default(StyleProperty::FontSize(styled.base.size));
        builder.push_default(StyleProperty::FontWeight(parley_weight(styled.base.weight)));
        builder.push_default(StyleProperty::FontStack(crate::layout::font_stack(
            &styled.family,
        )));
        builder.push_default(StyleProperty::FontStyle(ParleyFontStyle::Normal));
        builder.push_default(StyleProperty::Brush(TextBrush(styled.base.color)));

        // A span's range is a range in the *source* text, so it has to be
        // mapped onto the shaped text before it can be applied.
        for span in &styled.spans {
            let start = styled.orig_to_display(span.range.start).min(display.len());
            let end = styled.orig_to_display(span.range.end).min(display.len());
            if start >= end {
                continue;
            }
            let range = start..end;
            builder.push(StyleProperty::FontSize(span.style.size), range.clone());
            builder.push(
                StyleProperty::FontWeight(parley_weight(span.style.weight)),
                range.clone(),
            );
            builder.push(StyleProperty::Brush(TextBrush(span.style.color)), range);
        }

        let wrap_mode = match styled.wrap {
            TextWrap::None => TextWrapMode::NoWrap,
            TextWrap::Word => TextWrapMode::Wrap,
        };
        builder.push_default(StyleProperty::TextWrapMode(wrap_mode));
        builder.push_default(StyleProperty::OverflowWrap(OverflowWrap::BreakWord));

        // The line height is the *pitch* between the lines, not the height of
        // the ink: the shaper puts each line's baseline exactly this far below
        // the previous one's, which is what makes "the third line" a position
        // a caller can compute rather than measure.
        if let Some(height) = styled.line_height {
            builder.push_default(StyleProperty::LineHeight(ParleyLineHeight::Absolute(
                height,
            )));
        }

        let mut layout = builder.build(display);

        let max_advance = match styled.wrap {
            TextWrap::None => None,
            TextWrap::Word => styled.max_width,
        };
        layout.break_all_lines(max_advance);
        if let Some(width) = styled.max_width {
            layout.align(Some(width), Alignment::Start, AlignmentOptions::default());
        }
        layout
    }

    /// Records the runs and line metrics of a shaped layout.
    fn collect(&mut self, layout: Layout<TextBrush>) {
        let mut runs = Vec::new();
        let mut lines = Vec::new();

        for line in layout.lines() {
            let metrics = line.metrics();
            lines.push(LineMetrics {
                baseline: metrics.baseline,
                ascent: metrics.ascent,
                descent: metrics.descent,
                width: metrics.advance,
                text_range: self.display_to_orig_range(line.text_range()),
                min_coord: metrics.min_coord,
                max_coord: metrics.max_coord,
                line_height: metrics.line_height,
            });

            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                let run = glyph_run.run();
                let mut glyphs = Vec::with_capacity(glyph_run.glyphs().count());
                // Parley reports a pen advance plus a per-glyph offset, and a
                // glyph run here is a positioned sequence. The pen has to be
                // accumulated while copying, or every glyph keeps its own
                // offset (zero for plain left-to-right text) and the whole run
                // paints stacked at the origin — while the line's measured
                // width, the sum of the advances, stays correct and hides it.
                let mut pen_x = 0.0f32;
                for g in glyph_run.glyphs() {
                    glyphs.push(Glyph {
                        id: g.id,
                        x: pen_x + g.x,
                        y: g.y,
                        advance: g.advance,
                    });
                    pen_x += g.advance;
                }
                if glyphs.is_empty() && self.text.is_empty() {
                    continue;
                }
                runs.push(StyledRun {
                    font: run.font().clone(),
                    font_size: run.font_size(),
                    origin: Point::new(glyph_run.offset() as f64, metrics.baseline as f64),
                    color: glyph_run.style().brush.0,
                    glyphs,
                });
            }
        }

        let size = Size::new(layout.width() as f64, layout.height() as f64);
        self.layout = layout;
        self.runs = runs;
        self.lines = lines;
        self.size = size;
    }

    /// A shaped text range as a source text range.
    fn display_to_orig_range(&self, range: Range<usize>) -> Range<usize> {
        self.display_to_orig(range.start)..self.display_to_orig(range.end)
    }
}

/// A weight, as the shaper wants it.
fn parley_weight(weight: FontWeight) -> parley::style::FontWeight {
    parley::style::FontWeight::new(weight.to_value() as f32)
}

/// The key an outline bound is cached under.
#[derive(Clone, PartialEq, Eq, Hash)]
struct OutlineKey {
    blob_id: u64,
    font_index: u32,
    glyph_id: u32,
    size_bits: u32,
    coords: Vec<i16>,
    skew_bits: u32,
}

/// The ink bounds of one glyph, as `(x0, y0, x1, y1)` in layout units.
///
/// Named so the cache's value type says what the four floats mean; the tuple
/// itself is only ever unpacked in one place.
type OutlineBounds = (f32, f32, f32, f32);

thread_local! {
    /// The scaled outline bounds, cached by the font and glyph they came from.
    ///
    /// Scaling an outline is the expensive part of a selection rectangle, and
    /// the same glyph is asked for again on every repaint of the same
    /// selection. The cache is keyed by everything that can change the
    /// outline — the blob, the face within it, the glyph, the size, the
    /// variation coordinates and the synthetic skew — so a hit is the same
    /// outline the key describes.
    static OUTLINE_BOUNDS: std::cell::RefCell<HashMap<OutlineKey, Option<OutlineBounds>>> =
        std::cell::RefCell::new(HashMap::new());
    /// The scaler the outline bounds are measured with.
    static OUTLINE_SCALE_CONTEXT: std::cell::RefCell<ScaleContext> =
        std::cell::RefCell::new(ScaleContext::new());
}

/// The ink bounds of one glyph, in layout units.
///
/// `None` when the glyph has no outline — a space, or a glyph the face does
/// not have.
fn glyph_outline_bounds(
    font: &FontData,
    font_size: f32,
    normalized_coords: &[i16],
    skew: Option<f32>,
    glyph_id: u32,
) -> Option<swash::zeno::Bounds> {
    let key = OutlineKey {
        blob_id: font.data.id(),
        font_index: font.index,
        glyph_id,
        size_bits: font_size.to_bits(),
        coords: normalized_coords.to_vec(),
        skew_bits: skew.map(f32::to_bits).unwrap_or(0),
    };
    if let Some(cached) = OUTLINE_BOUNDS.with(|cache| cache.borrow().get(&key).copied()) {
        return cached.map(|(x0, y0, x1, y1)| {
            swash::zeno::Bounds::new(
                swash::zeno::Point::new(x0, y0),
                swash::zeno::Point::new(x1, y1),
            )
        });
    }

    let computed = compute_outline_bounds(font, font_size, normalized_coords, skew, glyph_id);
    OUTLINE_BOUNDS.with(|cache| {
        cache.borrow_mut().insert(key, computed);
    });
    computed.map(|(x0, y0, x1, y1)| {
        swash::zeno::Bounds::new(
            swash::zeno::Point::new(x0, y0),
            swash::zeno::Point::new(x1, y1),
        )
    })
}

/// The ink bounds of one glyph, as a plain rectangle.
fn compute_outline_bounds(
    font: &FontData,
    font_size: f32,
    normalized_coords: &[i16],
    skew: Option<f32>,
    glyph_id: u32,
) -> Option<OutlineBounds> {
    let font_ref = swash::FontRef::from_index(font.data.data(), font.index as usize)?;
    let mut outline = OUTLINE_SCALE_CONTEXT.with(|context| {
        let mut context = context.borrow_mut();
        let mut scaler = context
            .builder(font_ref)
            .size(font_size)
            .hint(true)
            .normalized_coords(normalized_coords.iter().copied())
            .build();
        scaler.scale_outline(glyph_id as u16)
    })?;
    if let Some(angle) = skew {
        outline.transform(&swash::zeno::Transform::skew(
            swash::zeno::Angle::from_degrees(angle),
            swash::zeno::Angle::ZERO,
        ));
    }
    let bounds = outline.bounds();
    if bounds.is_empty() {
        return None;
    }
    Some((bounds.min.x, bounds.min.y, bounds.max.x, bounds.max.y))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(text: &str) -> StyledText {
        StyledText {
            text: text.to_string(),
            base: TextStyle {
                size: 16.0,
                weight: FontWeight::Normal,
                color: Color::rgb(0, 0, 0),
            },
            spans: Vec::new(),
            family: FontFamily::SansSerif,
            wrap: TextWrap::None,
            tab_width: None,
            max_width: None,
            line_height: None,
        }
    }

    #[test]
    fn lays_out_a_line() {
        let layout = TextLayout::new(&plain("hello"));
        assert!(layout.size().width > 0.0, "the text has width");
        assert_eq!(layout.lines().len(), 1, "one line");
        assert_eq!(layout.runs().len(), 1, "one run");
    }

    #[test]
    fn empty_text_has_no_runs() {
        let layout = TextLayout::new(&plain(""));
        assert!(layout.runs().is_empty(), "no ink, so no runs");
        assert_eq!(layout.size().width, 0.0, "no ink, so no width");
        // An empty paragraph is still one line: the caret has to sit
        // somewhere, and a layout with no lines has nowhere to put it.
        assert_eq!(layout.visual_line_count(), 1);
        assert_eq!(layout.hit_test(Point::new(0.0, 0.0)), Some(0));
    }

    #[test]
    fn hit_test_finds_the_byte_under_a_point() {
        let layout = TextLayout::new(&plain("hello"));
        // A point at the far left is before the first glyph.
        let first = layout.hit_test(Point::new(0.0, 8.0)).unwrap();
        assert_eq!(first, 0);
        // A point past the end of the text is past the last byte.
        let last = layout.hit_test(Point::new(10_000.0, 8.0)).unwrap();
        assert_eq!(last, 5);
    }

    #[test]
    fn the_caret_moves_right_as_the_cursor_moves_right() {
        let layout = TextLayout::new(&plain("hello"));
        let first = layout.cursor_point(0, CaretAffinity::Downstream);
        let third = layout.cursor_point(3, CaretAffinity::Downstream);
        assert!(third.x > first.x, "the caret moved right");
        assert_eq!(first.y, third.y, "the same line, so the same baseline");
    }

    #[test]
    fn a_selection_covers_the_selected_bytes() {
        let layout = TextLayout::new(&plain("hello world"));
        let rects = layout.selection_rects(0..5);
        assert_eq!(rects.len(), 1, "one line, so one rectangle");
        assert!(rects[0].width() > 0.0, "the rectangle has width");
    }

    #[test]
    fn a_selection_spanning_lines_covers_both() {
        let layout = TextLayout::new(&plain("hello\nworld"));
        let rects = layout.selection_rects(0..8);
        assert_eq!(rects.len(), 2, "two lines, so two rectangles");
    }

    #[test]
    fn a_span_changes_the_ink_of_its_range() {
        let mut styled = plain("hello");
        styled.spans.push(TextSpan {
            range: 0..2,
            style: TextStyle {
                size: 16.0,
                weight: FontWeight::Normal,
                color: Color::rgb(255, 0, 0),
            },
        });
        let layout = TextLayout::new(&styled);
        // The first run is the span's, and it is red; the rest is the base ink.
        assert!(
            layout
                .runs()
                .iter()
                .any(|r| r.color == Color::rgb(255, 0, 0)),
            "the span's ink is on a run: {:?}",
            layout.runs()
        );
    }

    #[test]
    fn wrapping_produces_more_than_one_line() {
        let mut styled = plain("the quick brown fox jumps over the lazy dog");
        styled.wrap = TextWrap::Word;
        styled.max_width = Some(60.0);
        let layout = TextLayout::new(&styled);
        assert!(layout.lines().len() > 1, "the text wrapped");
    }

    #[test]
    fn no_wrapping_keeps_one_line() {
        let mut styled = plain("the quick brown fox jumps over the lazy dog");
        styled.wrap = TextWrap::None;
        styled.max_width = Some(60.0);
        let layout = TextLayout::new(&styled);
        assert_eq!(layout.lines().len(), 1, "the text did not wrap");
    }

    #[test]
    fn tabs_are_expanded_and_offsets_still_map_back() {
        let mut styled = plain("a\tb");
        styled.tab_width = Some(4);
        let layout = TextLayout::new(&styled);
        // The tab became spaces, so the shaped text is wider than the source.
        let source_width = TextLayout::new(&plain("a\tb")).size().width;
        assert!(layout.size().width >= source_width);
        // A byte offset in the source still answers geometry questions.
        let point = layout.cursor_point(2, CaretAffinity::Downstream);
        assert!(point.x >= 0.0);
    }

    #[test]
    fn an_empty_text_still_has_a_caret() {
        let layout = TextLayout::new(&plain(""));
        let point = layout.cursor_point(0, CaretAffinity::Downstream);
        assert_eq!(point.x, 0.0, "the caret is at the start of the line");
        let rect = layout
            .cursor_rect(0, CaretAffinity::Downstream, 2.0)
            .expect("an empty line still has a caret rectangle");
        assert_eq!(
            rect.width(),
            2.0,
            "the caret is as wide as it was asked to be"
        );
        assert!(rect.height() > 0.0, "and as tall as the line");
    }

    #[test]
    fn a_visual_line_reports_its_source_range() {
        let layout = TextLayout::new(&plain("hello\nworld"));
        assert_eq!(layout.line_text_range(0), Some(0..6));
        assert_eq!(layout.line_text_range(1), Some(6..11));
        assert_eq!(layout.visual_line_count(), 2);
    }

    /// The line height the caller asked for is the distance between the rows.
    ///
    /// This is the property an editor's scroll position rests on: the scroll
    /// offset is a number of rows, and a row is a line height below the last
    /// one. If the shaper put the rows anywhere else, the caret would be
    /// measured against one set of rows and painted against another.
    #[test]
    fn the_requested_line_height_is_the_distance_between_the_rows() {
        let mut styled = plain("one\ntwo\nthree");
        styled.line_height = Some(40.0);
        let layout = TextLayout::new(&styled);
        assert_eq!(layout.row_pitch(), 40.0, "the pitch is what was asked for");
        let lines = layout.lines();
        assert!(
            (lines[1].baseline - lines[0].baseline - 40.0).abs() < 0.01,
            "the second row's baseline is {} below the first's",
            lines[1].baseline - lines[0].baseline
        );
        assert!(
            (lines[2].baseline - lines[1].baseline - 40.0).abs() < 0.01,
            "and the third's is {} below the second's",
            lines[2].baseline - lines[1].baseline
        );
    }

    /// A layout that did not ask for a line height still reports a pitch.
    #[test]
    fn a_layout_without_a_line_height_still_has_a_pitch() {
        let layout = TextLayout::new(&plain("hello"));
        assert!(
            layout.row_pitch() > 0.0,
            "the font's own line height is a pitch"
        );
    }

    /// An offset in a wrapped line answers the row it is on.
    ///
    /// The row is what an editor places the caret on, so an offset in the
    /// second wrapped row must answer `1` rather than `0`.
    #[test]
    fn an_offset_in_a_wrapped_line_answers_its_row() {
        let mut styled = plain("the quick brown fox jumps over the lazy dog");
        styled.wrap = TextWrap::Word;
        styled.max_width = Some(60.0);
        styled.line_height = Some(20.0);
        let layout = TextLayout::new(&styled);
        assert!(layout.visual_line_count() > 1, "the text wrapped");
        assert_eq!(
            layout.row_of_offset(0),
            Some(0),
            "the start is the first row"
        );
        // The last byte is on the last row.
        let last = layout.row_of_offset(styled.text.len() - 1);
        assert_eq!(
            last,
            Some(layout.visual_line_count() - 1),
            "the end is on the last row"
        );
    }
}
