//! Gaussian-ish blur over an RGBA buffer, shared by every backend that has to
//! apply one to pixels it can reach.
//!
//! Two backends blur pixels and they must not disagree about what a blur looks
//! like. The software rasteriser owns its buffer outright and can blur in
//! place mid-walk; vello cannot — its rasterisation happens once, at the end —
//! so it renders a segment offscreen, reads it back and blurs here. Both land
//! in this file, so "the same `Filter::blur(8.0)`" means the same picture on
//! both, and a fix to the kernel fixes both.
//!
//! Three box passes approximate a Gaussian. One pass is visibly boxy; the
//! three-pass split is what makes the falloff read as a blur rather than as a
//! smear with hard shoulders.

use floom_types::Filter;

/// Box passes per blur.
///
/// Three is the usual trade: one pass is a square kernel (visible banding at
/// the edges of a large blur), two still shows its corners, three reads as a
/// Gaussian to the eye. Each pass costs one O(pixels) sweep, and the radius is
/// split across them, so the *spread* stays the filter's radius.
pub const BLUR_PASSES: usize = 3;

/// Blurs an RGBA8 buffer in place, premultiplied, with `filter`'s radii
/// scaled by `scale`.
///
/// `scale` is the device-pixel scale: a blur radius is specified in logical
/// px, so on a HiDPI surface the window must cover that many *device* pixels
/// or the blur would look half as wide on screen.
///
/// Premultiplied throughout, because that is what both backends' buffers hold.
/// Blurring straight (un-premultiplied) colour would let transparent pixels
/// whose RGB happens to be white bleed a white halo into the visible edge —
/// the classic artifact, and one that only appears over transparency, which is
/// exactly where a frosted panel sits.
pub fn blur_rgba(bytes: &mut [u8], w: usize, h: usize, filter: &Filter, scale: f64) {
    let Filter::Blur { radius_x, radius_y } = *filter;
    // A radius too small to survive the three-way split still gets a one-pixel
    // kernel: rounding it to zero would make a 1px blur silently do nothing.
    let kernel = |radius: f64| -> usize {
        if !(radius.is_finite() && radius > 0.0) {
            return 0;
        }
        ((radius * scale) / f64::from(BLUR_PASSES as u32))
            .round()
            .max(1.0) as usize
    };
    let (rx, ry) = (kernel(radius_x), kernel(radius_y));
    if w == 0 || h == 0 || (rx == 0 && ry == 0) {
        // Nothing to smear: the window passes through untouched, which is what
        // an identity filter asked for and what a zero-area window must do.
        return;
    }
    let mut fields: Vec<Vec<f32>> = (0..4).map(|_| vec![0.0f32; w * h]).collect();
    for i in 0..(w * h) {
        let a = f32::from(bytes[i * 4 + 3]) / 255.0;
        fields[3][i] = a;
        for c in 0..3 {
            fields[c][i] = f32::from(bytes[i * 4 + c]) / 255.0 * a;
        }
    }
    for field in fields.iter_mut() {
        for _ in 0..BLUR_PASSES {
            box_blur_xy(field, w, h, rx, ry);
        }
    }
    for i in 0..(w * h) {
        let a = fields[3][i];
        let inv = if a > 1e-6 { 1.0 / a } else { 0.0 };
        for c in 0..3 {
            let v = fields[c][i] * inv * 255.0;
            bytes[i * 4 + c] = v.round().clamp(0.0, 255.0) as u8;
        }
        bytes[i * 4 + 3] = (a * 255.0).round().clamp(0.0, 255.0) as u8;
    }
}

/// One horizontal+vertical pass of a box blur over a single-channel field,
/// `radius` pixels wide, via a sliding window — O(pixels), independent of
/// radius. Window edges clamp to the field, which is the edge behaviour a
/// shadow wants (the blur does not fall off a cliff at the pad boundary).
pub fn box_blur(field: &mut [f32], width: usize, height: usize, radius: usize) {
    box_blur_xy(field, width, height, radius, radius);
}

/// [`box_blur`] with a radius per axis: a horizontal motion smear and a
/// frosted panel are both real requests, and collapsing them into one number
/// would force a caller to pick.
pub fn box_blur_xy(
    field: &mut [f32],
    width: usize,
    height: usize,
    radius_x: usize,
    radius_y: usize,
) {
    if width == 0 || height == 0 {
        return;
    }
    let mut tmp = vec![0.0f32; field.len()];

    if radius_x == 0 {
        // A zero-width window averages one pixel, so the horizontal pass is a
        // copy — running the sliding window anyway would only burn time.
        tmp.copy_from_slice(field);
    } else {
        // Horizontal: rows of `width`, window [x-radius, x+radius] clamped.
        for y in 0..height {
            let row = y * width;
            let mut lo = 0usize;
            let mut hi = radius_x.min(width - 1);
            let mut sum: f32 = field[row..=row + hi].iter().sum();
            for x in 0..width {
                tmp[row + x] = sum / (hi - lo + 1) as f32;
                let nlo = (x + 1).saturating_sub(radius_x);
                let nhi = (x + 1 + radius_x).min(width - 1);
                while hi < nhi {
                    hi += 1;
                    sum += field[row + hi];
                }
                while lo < nlo {
                    sum -= field[row + lo];
                    lo += 1;
                }
            }
        }
    }

    if radius_y == 0 {
        field.copy_from_slice(&tmp);
        return;
    }
    // Vertical: columns of `height` over the horizontal result.
    for x in 0..width {
        let mut lo = 0usize;
        let mut hi = radius_y.min(height - 1);
        let mut sum: f32 = (0..=hi).map(|y| tmp[y * width + x]).sum();
        for y in 0..height {
            field[y * width + x] = sum / (hi - lo + 1) as f32;
            let nlo = (y + 1).saturating_sub(radius_y);
            let nhi = (y + 1 + radius_y).min(height - 1);
            while hi < nhi {
                hi += 1;
                sum += tmp[hi * width + x];
            }
            while lo < nlo {
                sum -= tmp[lo * width + x];
                lo += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A buffer of one opaque colour blurs to itself: a constant has nothing
    /// to smear, so a result that changed would mean the kernel is not
    /// normalised.
    #[test]
    fn a_constant_field_survives_a_blur() {
        let (w, h) = (16usize, 16usize);
        let mut bytes = vec![0u8; w * h * 4];
        for px in bytes.chunks_exact_mut(4) {
            px.copy_from_slice(&[80, 120, 200, 255]);
        }
        blur_rgba(&mut bytes, w, h, &Filter::blur(4.0), 1.0);
        for px in bytes.chunks_exact(4) {
            assert_eq!(px[3], 255, "opaque stays opaque");
            assert!(
                px[0].abs_diff(80) <= 1 && px[2].abs_diff(200) <= 1,
                "a constant field must blur to itself, got {px:?}"
            );
        }
    }

    /// A hard edge becomes a ramp: the pixel just inside is no longer the
    /// foreground's own colour, and the pixel just outside is no longer fully
    /// transparent. Both halves matter — a blur that only softened inward
    /// would leave the silhouette the same size.
    #[test]
    fn a_hard_edge_becomes_a_ramp_on_both_sides() {
        let (w, h) = (32usize, 8usize);
        let mut bytes = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * 4;
                if x < w / 2 {
                    bytes[i..i + 4].copy_from_slice(&[255, 0, 0, 255]);
                }
            }
        }
        let before_inside = bytes[(4 * w + (w / 2 - 1)) * 4 + 3];
        let before_outside = bytes[(4 * w + (w / 2)) * 4 + 3];
        blur_rgba(&mut bytes, w, h, &Filter::blur(3.0), 1.0);
        let after_inside = bytes[(4 * w + (w / 2 - 1)) * 4 + 3];
        let after_outside = bytes[(4 * w + (w / 2)) * 4 + 3];

        assert_eq!(before_inside, 255);
        assert_eq!(before_outside, 0);
        assert!(
            after_outside > 0,
            "the blur must spread outward, got alpha {after_outside}"
        );
        assert!(
            after_inside < 255 || after_outside < after_inside,
            "and inward: the edge is a ramp now, inside={after_inside} outside={after_outside}"
        );
    }

    /// An identity filter is a no-op, not an expensive copy that rounds.
    #[test]
    fn an_identity_filter_leaves_the_buffer_byte_identical() {
        let (w, h) = (8usize, 8usize);
        let mut original = vec![0u8; w * h * 4];
        for (i, px) in original.chunks_exact_mut(4).enumerate() {
            px.copy_from_slice(&[(i % 251) as u8, (i % 253) as u8, (i % 247) as u8, 255]);
        }
        let mut bytes = original.clone();
        blur_rgba(&mut bytes, w, h, &Filter::blur(0.0), 1.0);
        assert_eq!(bytes, original, "a zero radius must not touch a byte");
    }

    /// The radius is logical, so a HiDPI surface must blur over more *device*
    /// pixels for the same visual width.
    ///
    /// Checked at a radius large enough to survive the three-way split without
    /// clamping: the kernel is `round(radius * scale / 3)` with a floor of 1,
    /// so small radii collapse to the floor and scale cannot widen them. That
    /// floor is deliberate (a 1px blur must do something rather than nothing)
    /// but it means this property only holds above it, which is what the test
    /// pins rather than pretending otherwise.
    #[test]
    fn scale_widens_the_ramp_once_the_kernel_is_above_its_floor() {
        let ramp_width = |scale: f64| {
            let (w, h) = (128usize, 8usize);
            let mut bytes = vec![0u8; w * h * 4];
            for y in 0..h {
                for x in 0..w {
                    let i = (y * w + x) * 4;
                    if x < w / 2 {
                        bytes[i..i + 4].copy_from_slice(&[255, 0, 0, 255]);
                    }
                }
            }
            blur_rgba(&mut bytes, w, h, &Filter::blur(6.0), scale);
            let row = 4 * w;
            (0..w)
                .filter(|&x| {
                    let a = bytes[(row + x) * 4 + 3];
                    a > 8 && a < 247
                })
                .count()
        };
        let one = ramp_width(1.0);
        let two = ramp_width(2.0);
        assert!(
            two > one,
            "scale 2.0 must spread the edge wider than 1.0 (got {two} vs {one})"
        );
    }

    /// Below the kernel floor, scale cannot change anything — and that is the
    /// honest answer, not a bug. Pinning it stops someone "fixing" the floor
    /// into a fractional kernel to make the property above hold everywhere.
    #[test]
    fn a_tiny_radius_collapses_to_one_pixel_pass_at_any_scale() {
        let (w, h) = (64usize, 8usize);
        let render = |scale: f64| {
            let mut bytes = vec![0u8; w * h * 4];
            for y in 0..h {
                for x in 0..w {
                    let i = (y * w + x) * 4;
                    if x < w / 2 {
                        bytes[i..i + 4].copy_from_slice(&[255, 0, 0, 255]);
                    }
                }
            }
            blur_rgba(&mut bytes, w, h, &Filter::blur(2.0), scale);
            bytes
        };
        assert_eq!(
            render(1.0),
            render(2.0),
            "radius 2.0 is under the floor round(2*2/3)=1 at both scales, so both are one pass"
        );
    }
}
