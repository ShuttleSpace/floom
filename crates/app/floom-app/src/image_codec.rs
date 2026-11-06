// SPDX-License-Identifier: MIT
//! Image decoding: encoded PNG/JPEG bytes → registry-ready pixels.
//!
//! The resource registry stores RGBA8 **premultiplied** (what both the GPU
//! upload and the software path consume verbatim), while decoders produce
//! straight-alpha RGBA. This module owns that one conversion so producers
//! never hand-roll it: [`decode_image`] normalises any supported format to
//! 8-bit RGBA and premultiplies, and [`WidgetHost::register_image_decoded`](crate::WidgetHost::register_image_decoded)
//! (in the parent crate) feeds the result straight into the registry.

/// A decoded image in the registry's native format.
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    /// RGBA8 **premultiplied**, row-major, `width * height * 4` bytes.
    pub pixels: Vec<u8>,
}

/// Decodes encoded image bytes (PNG, JPEG — whatever the `image` crate
/// features enable) into premultiplied RGBA8.
///
/// Returns `None` for malformed input; a decode failure must surface at the
/// call site, not corrupt a later GPU upload.
pub fn decode_image(bytes: &[u8]) -> Option<DecodedImage> {
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut pixels = rgba.into_raw();
    premultiply_rgba8(&mut pixels);
    Some(DecodedImage {
        width,
        height,
        pixels,
    })
}

/// In-place straight-alpha → premultiplied-alpha conversion, with rounding.
fn premultiply_rgba8(pixels: &mut [u8]) {
    for px in pixels.as_chunks_mut::<4>().0 {
        let a = u32::from(px[3]);
        for channel in &mut px[..3] {
            *channel = ((u32::from(*channel) * a + 127) / 255) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn premultiply_scales_channels_by_alpha() {
        let mut px = vec![255, 128, 0, 128, 200, 200, 200, 255, 10, 20, 30, 0];
        premultiply_rgba8(&mut px);
        // (255*128+127)/255 = 128, (128*128+127)/255 = 64, (0*128+127)/255 = 0
        assert_eq!(&px[0..4], &[128, 64, 0, 128]);
        // Fully opaque: unchanged.
        assert_eq!(&px[4..8], &[200, 200, 200, 255]);
        // Fully transparent: black.
        assert_eq!(&px[8..12], &[0, 0, 0, 0]);
    }
}
