//! Colour arithmetic the design systems need and `floom_types::Color` does not
//! provide: alpha replacement, interpolation, WCAG contrast, and a tonal ramp.

use floom_types::Color;

/// WCAG 2.x contrast threshold for normal-size text.
pub const WCAG_AA_NORMAL: f64 = 4.5;
/// WCAG 2.x contrast threshold for large text and UI boundaries.
pub const WCAG_AA_LARGE: f64 = 3.0;
/// WCAG 2.x *enhanced* threshold for normal-size text.
pub const WCAG_AAA_NORMAL: f64 = 7.0;

/// `color` with its alpha replaced.
pub fn with_alpha(color: Color, a: u8) -> Color {
    Color::rgba(color.r, color.g, color.b, a)
}

/// Linear interpolation between two colours, including their alpha.
///
/// `t` is clamped to `0.0..=1.0`; the result is rounded rather than truncated,
/// so `lerp(a, b, 1.0) == b` exactly instead of landing one unit short.
pub fn lerp(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0) as f64;
    let mix = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t).round() as u8;
    Color::rgba(mix(a.r, b.r), mix(a.g, b.g), mix(a.b, b.b), mix(a.a, b.a))
}

/// WCAG 2.x relative luminance of a colour's RGB channels.
///
/// Alpha is ignored: this answers "how bright are these pixels", and the
/// compositing step that decides what they land on is the caller's business.
pub fn relative_luminance(color: Color) -> f64 {
    fn channel(v: u8) -> f64 {
        let c = v as f64 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
}

/// WCAG 2.x contrast ratio between two colours, in `1.0..=21.0`.
///
/// Symmetric by construction — contrast is a property of a *pair*, and the
/// formula's ordering is only there to keep the ratio above 1.
pub fn contrast_ratio(a: Color, b: Color) -> f64 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// Whether a pair clears a WCAG threshold.
pub fn meets_contrast(a: Color, b: Color, threshold: f64) -> bool {
    contrast_ratio(a, b) >= threshold
}

/// `layer` at `alpha` composited over `base`, source-over, non-premultiplied.
///
/// This is *not* [`lerp`], and using `lerp` for a state layer is a real bug:
/// `lerp` treats a transparent base as "black with zero alpha" and interpolates
/// the RGB away from black, so an 8% layer over nothing comes out as a dark
/// tint rather than as the accent colour at 8% alpha. Compositing blends alpha
/// and leaves the layer's own hue alone:
///
/// * base opaque → the layer tints it, result stays opaque;
/// * base clear  → the result is the layer at its own alpha.
pub fn composite_over(base: Color, layer: Color, alpha: f32) -> Color {
    let alpha = alpha.clamp(0.0, 1.0) as f64;
    let la = alpha * (layer.a as f64 / 255.0);
    let ba = base.a as f64 / 255.0;
    let out_a = la + ba * (1.0 - la);
    if out_a <= 0.0 {
        return Color::TRANSPARENT;
    }
    let channel = |l: u8, b: u8| {
        (((l as f64 * la) + (b as f64 * ba * (1.0 - la))) / out_a)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color::rgba(
        channel(layer.r, base.r),
        channel(layer.g, base.g),
        channel(layer.b, base.b),
        (out_a * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

/// The candidate that contrasts most with `background`.
///
/// A focus ring cannot be drawn in a fixed token: the same variant's fill is a
/// primary tint when outlined and a solid primary when filled, so a ring in
/// `primary` disappears on one of them. Choosing by measurement makes the ring
/// a ring — and because [`readable_on`]'s two extremes are usually in the
/// candidate list, the result clears the 3:1 boundary threshold by construction.
pub fn most_contrasting(background: Color, candidates: &[Color]) -> Color {
    candidates
        .iter()
        .copied()
        .max_by(|a, b| {
            contrast_ratio(*a, background)
                .partial_cmp(&contrast_ratio(*b, background))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or_else(|| readable_on(background))
}

/// Black or white, whichever contrasts more with `background`.
///
/// A derived palette cannot copy a published one's on-colours: those were chosen
/// against *that* primary, and a tonal ramp seeded with yellow produces a
/// primary on which white text is illegible. Picking by measurement makes the
/// contrast property hold by construction — the worse of the two candidates is
/// always at least `sqrt(21) ≈ 4.58`, which clears AA for normal text.
pub fn readable_on(background: Color) -> Color {
    if contrast_ratio(Color::WHITE, background) >= contrast_ratio(Color::BLACK, background) {
        Color::WHITE
    } else {
        Color::BLACK
    }
}

/// A lightness ramp derived from a seed colour, for dynamic colour schemes.
///
/// **What this is:** the seed converted to HSL, with hue and saturation held and
/// lightness swept across [`TonalPalette::STEPS`]. It is deterministic, monotone
/// in lightness, and cheap.
///
/// **What this is not:** Material 3's HCT tonal palette. HSL lightness is not
/// perceptually uniform, so two tones that are equally spaced here are not
/// equally spaced to the eye, and a saturated yellow ramps very differently from
/// a desaturated blue. Naming it `tonal_ramp` and saying so is the honest
/// option; claiming HCT would be a lie a reviewer could catch by measuring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TonalPalette {
    tones: Vec<(u8, Color)>,
}

impl TonalPalette {
    /// The tone values sampled, lightest last.
    pub const STEPS: [u8; 11] = [0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100];

    /// The colour at a given tone, or the nearest sampled tone.
    pub fn tone(&self, tone: u8) -> Color {
        let nearest = self
            .tones
            .iter()
            .min_by_key(|(t, _)| (*t as i32 - tone as i32).abs())
            .expect("a ramp always has tones");
        nearest.1
    }

    /// Every `(tone, colour)` pair, lightest last.
    pub fn tones(&self) -> &[(u8, Color)] {
        &self.tones
    }
}

/// Build a [`TonalPalette`] from a seed colour.
pub fn tonal_ramp(seed: Color) -> TonalPalette {
    let (h, s, _l) = rgb_to_hsl(seed);
    let tones = TonalPalette::STEPS
        .iter()
        .map(|&tone| {
            // Tone 0 is black and 100 is white regardless of seed — clamping the
            // endpoints keeps a ramp usable as a "darkest/lightest" reference.
            let l = if tone == 0 {
                0.0
            } else if tone == 100 {
                1.0
            } else {
                tone as f64 / 100.0
            };
            (tone, hsl_to_rgb(h, s, l))
        })
        .collect();
    TonalPalette { tones }
}

/// `(hue degrees, saturation 0..1, lightness 0..1)`.
fn rgb_to_hsl(c: Color) -> (f64, f64, f64) {
    let r = c.r as f64 / 255.0;
    let g = c.g as f64 / 255.0;
    let b = c.b as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d == 0.0 {
        return (0.0, 0.0, l);
    }
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let hue = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (hue * 60.0, s, l)
}

fn hsl_to_rgb(h_deg: f64, s: f64, l: f64) -> Color {
    if s == 0.0 {
        let v = (l * 255.0).round() as u8;
        return Color::rgb(v, v, v);
    }
    let h = h_deg.rem_euclid(360.0) / 360.0;
    fn hue_to_channel(p: f64, q: f64, mut t: f64) -> f64 {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 1.0 / 2.0 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let to_u8 = |v: f64| (v * 255.0).round().clamp(0.0, 255.0) as u8;
    Color::rgb(
        to_u8(hue_to_channel(p, q, h + 1.0 / 3.0)),
        to_u8(hue_to_channel(p, q, h)),
        to_u8(hue_to_channel(p, q, h - 1.0 / 3.0)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contrast_endpoints_match_the_wcag_reference() {
        // The canonical value: black against white is 21:1. Anything else means
        // the transfer function or its threshold is wrong.
        let r = contrast_ratio(Color::WHITE, Color::BLACK);
        assert!((r - 21.0).abs() < 1e-9, "got {r}");
        assert_eq!(contrast_ratio(Color::WHITE, Color::WHITE), 1.0);
    }

    #[test]
    fn contrast_is_symmetric() {
        let a = Color::rgb(37, 99, 235);
        let b = Color::rgb(250, 250, 250);
        assert_eq!(contrast_ratio(a, b), contrast_ratio(b, a));
    }

    #[test]
    fn meets_contrast_uses_the_threshold_it_is_given() {
        // The same colour, opposite verdicts: a light grey is legible on black
        // and unusable on white. (A *mid* grey would not show this — 118 sits at
        // ~4.5:1 against both extremes, which is how this assertion first came
        // out wrong.)
        let light = Color::rgb(200, 200, 200);
        assert!(
            !meets_contrast(light, Color::WHITE, WCAG_AA_NORMAL),
            "light grey on white clears AA?"
        );
        assert!(meets_contrast(light, Color::BLACK, WCAG_AA_LARGE));
        assert!(meets_contrast(light, Color::BLACK, WCAG_AAA_NORMAL));

        // The comparison is inclusive: check either side of the black/white
        // pair's exact 21:1.
        assert!(meets_contrast(Color::WHITE, Color::BLACK, 20.9));
        assert!(!meets_contrast(Color::WHITE, Color::BLACK, 21.1));
    }

    #[test]
    fn readable_on_clears_aa_for_any_background() {
        // The property must hold for *every* background, not just the flattering
        // ones — a saturated yellow is the case that breaks a palette which
        // assumes white text.
        let backgrounds = [
            Color::rgb(0, 0, 0),
            Color::rgb(255, 255, 255),
            Color::rgb(255, 241, 0), // yellow: white text is unreadable here
            Color::rgb(37, 99, 235), // blue
            Color::rgb(4, 108, 78),  // green
            Color::rgb(128, 128, 128), // mid grey: the worst case
            Color::rgb(103, 80, 164), // the Material seed
        ];
        for bg in backgrounds {
            let fg = readable_on(bg);
            let ratio = contrast_ratio(fg, bg);
            assert!(
                ratio >= WCAG_AA_NORMAL,
                "readable_on({bg:?}) = {fg:?} only reaches {ratio:.3}:1"
            );
        }
        // Mid grey is the floor of the property, and it is the crossing point of
        // the two candidates rather than either extreme.
        assert!(
            contrast_ratio(
                readable_on(Color::rgb(128, 128, 128)),
                Color::rgb(128, 128, 128)
            ) > 4.5
        );
    }

    #[test]
    fn lerp_hits_both_endpoints_exactly() {
        let a = Color::rgb(0, 0, 0);
        let b = Color::rgb(255, 255, 255);
        assert_eq!(lerp(a, b, 0.0), a);
        assert_eq!(lerp(a, b, 1.0), b, "rounding must reach the endpoint");
        assert_eq!(lerp(a, b, 0.5), Color::rgb(128, 128, 128));
        // Out-of-range t is clamped, not wrapped.
        assert_eq!(lerp(a, b, -3.0), a);
        assert_eq!(lerp(a, b, 9.0), b);
    }

    #[test]
    fn with_alpha_replaces_alpha_only() {
        let c = with_alpha(Color::rgb(10, 20, 30), 128);
        assert_eq!((c.r, c.g, c.b), (10, 20, 30));
        assert_eq!(c.a, 128);
    }

    #[test]
    fn compositing_over_an_opaque_base_keeps_it_opaque() {
        let out = composite_over(Color::WHITE, Color::BLACK, 0.5);
        assert_eq!(out, Color::rgb(128, 128, 128));
        assert_eq!(out.a, 255);
        // A zero-alpha layer changes nothing.
        assert_eq!(
            composite_over(Color::WHITE, Color::BLACK, 0.0),
            Color::WHITE
        );
    }

    #[test]
    fn compositing_over_a_clear_base_keeps_the_layers_hue() {
        // The bug this exists to prevent: `lerp` from a transparent black pulls
        // the RGB toward black, giving a muddy dark tint where the design asks
        // for the accent at 8%.
        let accent = Color::rgb(103, 80, 164);
        let out = composite_over(Color::TRANSPARENT, accent, 0.08);
        assert_eq!(
            (out.r, out.g, out.b),
            (accent.r, accent.g, accent.b),
            "the layer's hue must survive"
        );
        assert_eq!(out.a, 20, "8% of 255 rounds to 20");
        assert_ne!(
            lerp(Color::TRANSPARENT, accent, 0.08),
            out,
            "lerp and composite must differ on a clear base"
        );
    }

    #[test]
    fn compositing_accumulates_alpha() {
        // Half-opaque white under a half-strength black layer: alpha 128 over
        // alpha 128 is three-quarters, not one or the other.
        let out = composite_over(Color::rgba(255, 255, 255, 128), Color::BLACK, 0.5);
        assert!(out.a > 128 && out.a < 255, "got alpha {}", out.a);
        // A clear base with a clear layer is still clear.
        assert_eq!(
            composite_over(Color::TRANSPARENT, Color::TRANSPARENT, 1.0),
            Color::TRANSPARENT
        );
    }

    #[test]
    fn a_tonal_ramp_is_deterministic_and_monotone_in_luminance() {
        let seed = Color::rgb(37, 99, 235);
        assert_eq!(tonal_ramp(seed), tonal_ramp(seed));

        let ramp = tonal_ramp(seed);
        let lums: Vec<f64> = ramp
            .tones()
            .iter()
            .map(|(_, c)| relative_luminance(*c))
            .collect();
        assert!(
            lums.windows(2).all(|w| w[0] < w[1]),
            "ramp is not monotone in luminance: {lums:?}"
        );
        assert_eq!(ramp.tone(0), Color::BLACK, "tone 0 is black");
        assert_eq!(ramp.tone(100), Color::WHITE, "tone 100 is white");
    }

    #[test]
    fn most_contrasting_picks_by_measurement_not_by_order() {
        // A primary tint and the primary itself are indistinguishable to a ring
        // drawn in `primary`; the helper must not return the first candidate.
        let tint = composite_over(Color::TRANSPARENT, Color::rgb(103, 80, 164), 0.12);
        let candidates = [
            Color::rgb(103, 80, 164), // primary: invisible here, and listed first
            Color::rgb(29, 27, 32),   // on_surface
            Color::WHITE,
            Color::BLACK,
        ];
        let ring = most_contrasting(tint, &candidates);
        assert_ne!(ring, candidates[0], "the invisible candidate won");
        assert!(
            contrast_ratio(ring, tint) >= WCAG_AA_LARGE,
            "ring {ring:?} on {tint:?} only reaches {:.2}:1",
            contrast_ratio(ring, tint)
        );

        // With black and white both offered, the boundary threshold is met for
        // any background — this is why the ring rule cannot fail.
        for bg in [
            Color::rgb(0, 0, 0),
            Color::rgb(255, 255, 255),
            Color::rgb(128, 128, 128),
            Color::rgb(255, 241, 0),
        ] {
            let ring = most_contrasting(bg, &[Color::WHITE, Color::BLACK]);
            assert!(contrast_ratio(ring, bg) >= WCAG_AA_LARGE, "bg {bg:?}");
        }

        // An empty candidate list still yields something readable.
        assert_eq!(
            most_contrasting(Color::WHITE, &[]),
            Color::BLACK,
            "fallback should be the readable choice"
        );
    }

    #[test]
    fn a_ramp_keeps_the_seeds_hue() {
        // A blue seed must not produce an orange mid-tone. Compare hue rather
        // than RGB, since lightness necessarily changes the channels.
        let seed = Color::rgb(37, 99, 235);
        let (seed_h, seed_s, _) = rgb_to_hsl(seed);
        for (tone, c) in tonal_ramp(seed).tones() {
            if *tone == 0 || *tone == 100 {
                continue; // endpoints are achromatic by construction
            }
            let (h, s, _) = rgb_to_hsl(*c);
            assert!(
                (h - seed_h).abs() < 1.0,
                "tone {tone} drifted to hue {h} from {seed_h}"
            );
            assert!((s - seed_s).abs() < 0.02, "tone {tone} changed saturation");
        }
    }

    #[test]
    fn an_achromatic_seed_yields_a_grey_ramp() {
        let ramp = tonal_ramp(Color::rgb(128, 128, 128));
        for (tone, c) in ramp.tones() {
            assert_eq!(c.r, c.g, "tone {tone} is not grey: {c:?}");
            assert_eq!(c.g, c.b, "tone {tone} is not grey: {c:?}");
        }
    }

    #[test]
    fn tone_lookup_snaps_to_the_nearest_sample() {
        let ramp = tonal_ramp(Color::rgb(37, 99, 235));
        assert_eq!(ramp.tone(11), ramp.tone(10));
        assert_eq!(ramp.tone(99), ramp.tone(100));
    }
}
