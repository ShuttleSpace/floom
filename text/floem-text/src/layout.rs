//! Parley-backed text shaper emitting renderer-neutral glyph runs.

use floem_types::{Point, Size};
use parley::layout::{Alignment, AlignmentOptions, PositionedLayoutItem};
use parley::style::{FontFamily, FontStack, FontWeight, GenericFamily, StyleProperty};
use parley::{FontContext, LayoutContext};

/// Horizontal alignment for laid-out paragraphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum TextAlign {
    #[default]
    Start = 0,
    Middle = 1,
    End = 2,
}

impl TextAlign {
    fn to_parley(self) -> Alignment {
        match self {
            Self::Start => Alignment::Start,
            Self::Middle => Alignment::Center,
            Self::End => Alignment::End,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextLayoutOptions {
    pub max_width: f64,
    pub font_size: f32,
    pub align: TextAlign,
}

impl Default for TextLayoutOptions {
    fn default() -> Self {
        Self {
            max_width: f64::INFINITY,
            font_size: 16.0,
            align: TextAlign::Start,
        }
    }
}

/// Single positioned glyph (renderer-neutral).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glyph {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub advance: f32,
}

/// Contiguous glyphs sharing font/size (one Parley glyph run).
///
/// Carries the font *data* itself, not a font index: renderers (vello, Skia)
/// rasterise directly from the blob, so a glyph run is self-contained and no
/// shared font table has to stay alive across the scene → renderer boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct GlyphRun {
    pub font: peniko::FontData,
    pub font_size: f32,
    pub origin: Point,
    pub glyphs: Vec<Glyph>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LineMetrics {
    pub baseline: f32,
    pub ascent: f32,
    pub descent: f32,
    pub width: f32,
    /// Byte range into the source string (UTF-8).
    pub text_range: std::ops::Range<usize>,
}

/// Fully shaped + wrapped text ready for painting.
#[derive(Debug, Clone, PartialEq)]
pub struct LaidOutText {
    pub size: Size,
    pub runs: Vec<GlyphRun>,
    pub lines: Vec<LineMetrics>,
    /// Visual run count — useful for bidi / fallback assertions.
    pub run_count: usize,
}

/// Owns Parley contexts and produces [`LaidOutText`].
pub struct TextShaper {
    font_cx: FontContext,
    layout_cx: LayoutContext<()>,
}

impl Default for TextShaper {
    fn default() -> Self {
        Self::new()
    }
}

impl TextShaper {
    pub fn new() -> Self {
        Self {
            font_cx: FontContext::new(),
            layout_cx: LayoutContext::new(),
        }
    }

    pub fn shape(&mut self, text: &str, options: &TextLayoutOptions) -> LaidOutText {
        let mut builder = self
            .layout_cx
            .ranged_builder(&mut self.font_cx, text, 1.0, true);
        builder.push_default(StyleProperty::FontSize(options.font_size));
        builder.push_default(StyleProperty::FontWeight(FontWeight::NORMAL));
        builder.push_default(StyleProperty::FontStack(FontStack::Single(
            FontFamily::Generic(GenericFamily::SansSerif),
        )));
        let mut layout = builder.build(text);

        let width = if options.max_width.is_finite() {
            Some(options.max_width as f32)
        } else {
            None
        };
        layout.break_all_lines(width);
        layout.align(
            width,
            options.align.to_parley(),
            AlignmentOptions::default(),
        );

        let mut runs = Vec::new();
        let mut lines = Vec::new();

        for line in layout.lines() {
            let metrics = line.metrics();
            lines.push(LineMetrics {
                baseline: metrics.baseline,
                ascent: metrics.ascent,
                descent: metrics.descent,
                width: metrics.advance,
                text_range: line.text_range(),
            });

            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                let run = glyph_run.run();
                let mut glyphs = Vec::with_capacity(glyph_run.glyphs().count());
                // Parley reports a *pen advance* plus a per-glyph offset, and a
                // glyph run here is a positioned sequence — the renderers draw
                // `glyph.x` at the run origin and never look at the advance. So
                // the pen has to be accumulated while copying, or every glyph
                // keeps its own offset (0 for plain LTR text) and the whole run
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
                if glyphs.is_empty() && text.is_empty() {
                    continue;
                }
                runs.push(GlyphRun {
                    font: run.font().clone(),
                    font_size: run.font_size(),
                    origin: Point::new(glyph_run.offset() as f64, metrics.baseline as f64),
                    glyphs,
                });
            }
        }

        let run_count = runs.len();
        let width = layout.width() as f64;
        let height = layout.height() as f64;
        LaidOutText {
            size: Size::new(width, height),
            runs,
            lines,
            run_count,
        }
    }

    /// UTF-16-ish selection geometry helper: byte range → first glyph cluster x.
    pub fn caret_x(&mut self, text: &str, options: &TextLayoutOptions, byte_index: usize) -> f32 {
        let laid = self.shape(text, options);
        let mut x = 0.0f32;
        let mut consumed = 0usize;
        // Approximate: advance through glyphs until byte_index in first line.
        if let Some(line) = laid.lines.first() {
            let _ = line;
            for run in &laid.runs {
                for g in &run.glyphs {
                    if consumed >= byte_index {
                        return x;
                    }
                    x += g.advance;
                    // One glyph ≈ one scalar for ASCII tests; multi-byte handled loosely.
                    consumed += 1;
                }
            }
        }
        x
    }
}
