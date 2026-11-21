// SPDX-License-Identifier: MIT
//! Parley-backed text shaper emitting renderer-neutral glyph runs.

use std::borrow::Cow;

use floom_types::{FontFamily as FloomFontFamily, FontWeight, Point, Size};
use parley::layout::{Alignment, AlignmentOptions, PositionedLayoutItem};
use parley::style::{
    FontFamily, FontStack, FontWeight as ParleyFontWeight, GenericFamily, StyleProperty,
};
use parley::{FontContext, LayoutContext};

/// Registers `data` as the face behind `family` in `font_cx`.
///
/// Shared with the editor's font context: two font databases in one process is
/// already one more than ideal, and they must at least agree.
pub(crate) fn register_font_into(
    font_cx: &mut FontContext,
    data: &[u8],
    family: &FloomFontFamily,
) -> usize {
    let Some(generic) = generic_family(family) else {
        return 0;
    };
    let registered = font_cx
        .collection
        .register_fonts(parley::fontique::Blob::from(data.to_vec()), None);
    let faces = registered.iter().map(|(_, infos)| infos.len()).sum();
    if faces > 0 {
        let ids = registered.into_iter().map(|(id, _)| id);
        font_cx.collection.set_generic_families(generic, ids);
    }
    faces
}

/// The parley generic behind a Floom family, or `None` when it names one.
fn generic_family(family: &FloomFontFamily) -> Option<GenericFamily> {
    match family {
        FloomFontFamily::Serif => Some(GenericFamily::Serif),
        FloomFontFamily::SansSerif => Some(GenericFamily::SansSerif),
        FloomFontFamily::Monospace => Some(GenericFamily::Monospace),
        FloomFontFamily::Cursive => Some(GenericFamily::Cursive),
        FloomFontFamily::Fantasy => Some(GenericFamily::Fantasy),
        FloomFontFamily::Named(_) => None,
    }
}

/// Default font stack, tried left-to-right.
///
/// The generic sans-serif covers Latin, digits and symbols. The remaining
/// families are *fallbacks* so that Han (and Kana / Hangul) glyphs resolve to a
/// real face instead of `.notdef` tofu boxes. parley's automatic per-script
/// fallback does not reliably pick a Han-capable face on every platform — on
/// macOS the generic sans resolves to a font with no CJK coverage and the
/// auto-fallback returns that same font — so we name the common CJK families
/// explicitly. A name that is not installed is simply ignored by the resolver,
/// so listing every platform's fonts in one stack is harmless.
///
/// `FontFamily::Named(Cow::Borrowed(..))` is a constant expression, so this is a
/// zero-allocation `static`.
static DEFAULT_FONT_STACK: &[FontFamily<'static>] = &[
    FontFamily::Generic(GenericFamily::SansSerif),
    FontFamily::Named(Cow::Borrowed("PingFang SC")),
    FontFamily::Named(Cow::Borrowed("Hiragino Sans GB")),
    FontFamily::Named(Cow::Borrowed("Hiragino Sans")),
    FontFamily::Named(Cow::Borrowed("STHeiti")),
    FontFamily::Named(Cow::Borrowed("Microsoft YaHei")),
    FontFamily::Named(Cow::Borrowed("SimSun")),
    FontFamily::Named(Cow::Borrowed("Malgun Gothic")),
    FontFamily::Named(Cow::Borrowed("Noto Sans CJK SC")),
    FontFamily::Named(Cow::Borrowed("Noto Sans CJK JP")),
    FontFamily::Named(Cow::Borrowed("Noto Sans CJK KR")),
    FontFamily::Named(Cow::Borrowed("Noto Sans CJK TC")),
    FontFamily::Named(Cow::Borrowed("Source Han Sans SC")),
    FontFamily::Named(Cow::Borrowed("WenQuanYi Zen Hei")),
    FontFamily::Named(Cow::Borrowed("Droid Sans Fallback")),
];

/// The default [`FontStack`] used when a caller does not specify one.
///
/// Kept as a function so the family request and the fallback list are composed
/// in exactly one place: a named family that the device does not ship has to
/// land on the same CJK-capable stack the default does, or a phone with a
/// Chinese locale renders `你好` as tofu boxes while Latin text looks fine.
pub(crate) fn font_stack(family: &FloomFontFamily) -> FontStack<'static> {
    let primary = match generic_family(family) {
        Some(generic) => FontFamily::Generic(generic),
        None => match family {
            FloomFontFamily::Named(name) => FontFamily::Named(Cow::Owned(name.clone())),
            // `generic_family` answers for every variant but `Named`.
            _ => FontFamily::Generic(GenericFamily::SansSerif),
        },
    };
    let mut stack = Vec::with_capacity(DEFAULT_FONT_STACK.len() + 1);
    stack.push(primary);
    stack.extend(DEFAULT_FONT_STACK.iter().cloned());
    FontStack::List(Cow::Owned(stack))
}

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

/// Everything that changes the shaped result, and therefore everything a cache
/// keyed on a shaped run has to compare.
///
/// The fields here and the fields of [`crate::LayoutCache`]'s key are the same
/// list on purpose. A field added to one and not the other is the bug this type
/// was extended to fix: floem-ui asks for a `Medium` weight on 50 call sites,
/// and a cache that forgot the weight would hand back the regular glyphs for
/// every one of them.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLayoutOptions {
    pub max_width: f64,
    pub font_size: f32,
    pub align: TextAlign,
    /// How heavy the run is drawn. Default is `Normal`, which is what every text
    /// path that does not ask gets.
    pub weight: FontWeight,
    /// The typeface the run is set in.
    ///
    /// `SansSerif` — the default — is the platform's own default with the CJK
    /// fallbacks behind it. A `Named` family is a *request*: a device that does
    /// not ship it falls back to the same stack, which is what makes setting one
    /// safe on a phone whose fonts the app cannot know.
    pub family: FloomFontFamily,
}

impl Default for TextLayoutOptions {
    fn default() -> Self {
        Self {
            max_width: f64::INFINITY,
            font_size: 16.0,
            align: TextAlign::Start,
            weight: FontWeight::Normal,
            family: FloomFontFamily::SansSerif,
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
    /// The top of the line box, in layout units.
    ///
    /// The line box is taller than the ink when the line has leading, and the
    /// leading is what a selection background has to cover to read as a
    /// highlight rather than as a band across the glyphs.
    pub min_coord: f32,
    /// The bottom of the line box, in layout units.
    pub max_coord: f32,
    /// The distance between this line's baseline and the next one's.
    ///
    /// This is the *pitch*, not the height of the ink: a line box with leading
    /// is shorter than the space the next line starts in. A caller that places
    /// lines uses this, because "the next line" starts a pitch below, not an
    /// ink height below.
    pub line_height: f32,
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
        let mut font_cx = FontContext::new();
        // Before anything asks for a family: the CJK faces the platform ships
        // outside the font backend's scan are what keep `你好` from being tofu.
        crate::system_fonts::register(&mut font_cx);
        Self {
            font_cx,
            layout_cx: LayoutContext::new(),
        }
    }

    pub fn shape(&mut self, text: &str, options: &TextLayoutOptions) -> LaidOutText {
        let mut builder = self
            .layout_cx
            .ranged_builder(&mut self.font_cx, text, 1.0, true);
        builder.push_default(StyleProperty::FontSize(options.font_size));
        // Previously hardcoded to NORMAL. Lowered through the engine-neutral
        // enum rather than taken from the shaper, so the number here is the one
        // the cache key carries.
        builder.push_default(StyleProperty::FontWeight(ParleyFontWeight::new(
            options.weight.to_value().into(),
        )));
        builder.push_default(StyleProperty::FontStack(font_stack(&options.family)));
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

    /// Registers `data` (a font file) as the face behind `family` on this
    /// shaper, and returns how many faces it added — 0 means it was not a font
    /// or `family` names one instead of a generic.
    ///
    /// Tests point [`FloomFontFamily::SansSerif`] at a bundled font: a golden
    /// that records a shaped advance width is reproducible only on the host
    /// whose default font produced it.
    pub fn register_font(&mut self, data: Vec<u8>, family: FloomFontFamily) -> usize {
        register_font_into(&mut self.font_cx, &data, &family)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression for the macOS-gate finding: CJK text must not render as
    /// `.notdef` tofu. The default font stack lists Han-capable families after
    /// the generic sans, so `你好世界` resolves to real glyph ids rather than 0.
    ///
    /// Gated to macOS because that is the platform this was reproduced on and
    /// the one with a guaranteed-installed CJK face (PingFang / Hiragino). On
    /// other platforms the assertion only holds if one of `DEFAULT_FONT_STACK`'s
    /// names is installed, which is an environment property, not a code one.
    #[test]
    #[cfg(target_os = "macos")]
    fn cjk_glyphs_are_not_tofu() {
        let mut shaper = TextShaper::new();
        let laid = shaper.shape("你好世界", &TextLayoutOptions::default());
        assert!(
            !laid.runs.is_empty(),
            "expected at least one glyph run for CJK text"
        );
        let tofu = laid.runs.iter().flat_map(|r| &r.glyphs).any(|g| g.id == 0);
        assert!(!tofu, "CJK glyphs resolved to .notdef (tofu box)");
    }

    /// A family request reaches the shaper.
    ///
    /// Monospace glyphs all advance by the same amount, so `iiii` is wider in a
    /// monospace face than in a proportional one. If the request were dropped —
    /// the failure this covers — the two shapings would be identical, which is
    /// invisible in a screenshot of a single string and is exactly how a
    /// `family()` setter that nobody wired up survives review.
    #[test]
    fn a_family_request_changes_the_shaped_advance() {
        let mut shaper = TextShaper::new();
        let proportional = shaper
            .shape("iiii", &TextLayoutOptions::default())
            .size
            .width;
        let monospace = shaper
            .shape(
                "iiii",
                &TextLayoutOptions {
                    family: FloomFontFamily::Monospace,
                    ..Default::default()
                },
            )
            .size
            .width;
        assert!(
            monospace > proportional,
            "monospace `iiii` ({monospace}) must be wider than proportional ({proportional})"
        );
    }

    /// The default stack still resolves plain Latin text to valid glyphs, so the
    /// CJK fallback list did not disturb the common case.
    #[test]
    fn latin_glyphs_resolve() {
        let mut shaper = TextShaper::new();
        let laid = shaper.shape("Hello, world", &TextLayoutOptions::default());
        let tofu = laid.runs.iter().flat_map(|r| &r.glyphs).any(|g| g.id == 0);
        assert!(!tofu, "Latin glyphs resolved to .notdef");
    }
}
