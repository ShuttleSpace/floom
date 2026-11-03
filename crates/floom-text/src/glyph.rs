//! Swash glyph rasterisation, shared by every renderer.
//!
//! Both the CPU rasteriser and the GPU (vello) presenter draw text the way
//! mature GPU text stacks do — rasterise each glyph to an alpha mask with
//! swash at the *device* size, then blend that mask as a textured quad — rather
//! than asking the GPU to fill a vector outline. Two reasons:
//!
//! * Resolution: rasterising at `font_size * scale` is correct at any density.
//! * Portability: driver-specific vector-glyph paths can draw heavier, thinner
//!   or malformed glyphs on some Android GPUs. A mask sampled as a texture is
//!   the same everywhere.

use peniko::FontData;

/// One rasterised glyph: its 8-bit alpha coverage and swash's placement of the
/// mask relative to the pen (left is pen→image-left; top is image-top→baseline,
/// y-down).
pub struct GlyphMask {
    pub placement: swash::zeno::Placement,
    pub data: Vec<u8>,
}

/// Rasterises one glyph as an 8-bit alpha mask at `font_size` device pixels.
pub fn rasterize_glyph(
    font: swash::FontRef<'_>,
    font_size: f32,
    glyph_id: u32,
) -> Option<GlyphMask> {
    use swash::scale::image::Content;
    use swash::scale::{Render, ScaleContext, Source, StrikeWith};
    use swash::zeno::Format;

    thread_local! {
        static SCALE_CONTEXT: std::cell::RefCell<ScaleContext> =
            std::cell::RefCell::new(ScaleContext::new());
    }

    SCALE_CONTEXT.with_borrow_mut(|context| {
        let mut scaler = context.builder(font).size(font_size).build();
        let mut render = Render::new(&[
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
        ]);
        render.format(Format::Alpha);
        let image = render.render(&mut scaler, glyph_id as u16)?;
        if image.content != Content::Mask {
            // Colour glyphs (emoji) have no representation in an alpha mask.
            return None;
        }
        Some(GlyphMask {
            placement: image.placement,
            data: image.data,
        })
    })
}

/// [`rasterize_glyph`] from a run's font blob, the form both renderers hold.
pub fn rasterize_glyph_from_font(
    font: &FontData,
    font_size: f32,
    glyph_id: u32,
) -> Option<GlyphMask> {
    let bytes = font.data.as_ref();
    if bytes.is_empty() {
        return None;
    }
    let font_ref = swash::FontRef::from_index(bytes, font.index as usize)?;
    rasterize_glyph(font_ref, font_size, glyph_id)
}
