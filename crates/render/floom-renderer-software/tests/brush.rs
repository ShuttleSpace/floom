// SPDX-License-Identifier: MIT
//! Pixel-level evidence for MOTION-GAP A3 (brushes) and A4 (shadows).
//!
//! Same discipline as `paths.rs` and `transforms.rs`: every positive probe is
//! paired with the control that makes it fail-safe. "This pixel is blue" is
//! worthless alone — it becomes evidence next to "the same shape holding one
//! colour puts the *same* blue at both ends", which is the difference between
//! a gradient and a lucky sample.
//!
//! The software backend is the reference here because it answers per pixel.
//! What it cannot answer — whether another backend *tried* — is what its own
//! counters are for, so the counts are asserted alongside the pixels rather
//! than trusted.

use floom_renderer::{FrameInfo, render_once};
use floom_renderer_software::SoftwareRenderer;
use floom_scene::{Point, Rect, Scene, SceneNode};
use floom_types::{Brush, Color, Shadow};

const RED: Color = Color::rgb(220, 40, 40);
const BLUE: Color = Color::rgb(40, 60, 220);

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
    Rect { x0, y0, x1, y1 }
}

fn node(bounds: Rect, brush: Brush, shadow: Option<Shadow>) -> SceneNode {
    SceneNode::Rect {
        bounds,
        fill: brush,
        radius: 0.0,
        shadow,
    }
}

/// Renders one scene into a fresh 24×24 buffer. The buffer starts transparent,
/// so "nothing reached this pixel" is a readable answer rather than an
/// assumption.
fn render(nodes: Vec<SceneNode>) -> SoftwareRenderer {
    let mut scene = Scene::new();
    scene.begin_update();
    for node in nodes {
        let _ = scene.insert(node);
    }
    scene.finish_update();
    let mut sw = SoftwareRenderer::new(24, 24);
    render_once(&mut sw, &FrameInfo::software(24, 24, 1), &scene);
    sw
}

fn pixel(sw: &SoftwareRenderer, x: u32, y: u32) -> [u8; 4] {
    sw.pixel_rgba(x, y).expect("probe inside the buffer")
}

fn alpha(sw: &SoftwareRenderer, x: u32, y: u32) -> u8 {
    pixel(sw, x, y)[3]
}

/// How far apart two pixels' colours are, summed per channel.
fn colour_distance(a: [u8; 4], b: [u8; 4]) -> u32 {
    a[0].abs_diff(b[0]) as u32 + a[1].abs_diff(b[1]) as u32 + a[2].abs_diff(b[2]) as u32
}

// ---------------------------------------------------------------------------
// A3 — brushes
// ---------------------------------------------------------------------------

/// A linear gradient puts a *different* colour at each end of one rect, and
/// the same rect holding one colour does not. That pair is the whole claim:
/// the difference is the ramp, not the rectangle.
#[test]
fn a_linear_gradient_changes_colour_across_the_shape() {
    let bounds = rect(0.0, 0.0, 20.0, 20.0);
    let ramp = Brush::linear(
        &[(0.0, RED), (1.0, BLUE)],
        Point::new(0.0, 0.0),
        Point::new(20.0, 0.0),
    );

    let sw = render(vec![node(bounds, ramp.clone(), None)]);
    let left = pixel(&sw, 3, 10);
    let right = pixel(&sw, 17, 10);
    assert!(
        colour_distance(left, right) > 200,
        "the two ends of the ramp are far apart, got {left:?} and {right:?}"
    );
    assert!(
        left[0] > left[2],
        "the red stop is nearer the start of the axis: {left:?}"
    );
    assert!(
        right[2] > right[0],
        "the blue stop is nearer the end of the axis: {right:?}"
    );
    assert_eq!(
        sw.gradients_drawn, 1,
        "one gradient went through the sampler"
    );

    // Control: one colour, same rect, same probes. If the probes above were
    // reading noise, this shape would read as a gradient too.
    let flat = render(vec![node(bounds, Brush::Solid(RED), None)]);
    assert_eq!(
        colour_distance(pixel(&flat, 3, 10), pixel(&flat, 17, 10)),
        0,
        "a solid fill paints one colour everywhere"
    );
    assert_eq!(flat.gradients_drawn, 0, "no per-pixel sampling happened");
}

/// A radial gradient falls off from its centre: the middle is one stop, the
/// rim is the other, and past the radius the last stop holds (a gradient
/// clamps, it does not wrap).
#[test]
fn a_radial_gradient_falls_off_from_its_centre() {
    let bounds = rect(0.0, 0.0, 20.0, 20.0);
    let ramp = Brush::radial(&[(0.0, RED), (1.0, BLUE)], Point::new(10.0, 10.0), 8.0);
    let sw = render(vec![node(bounds, ramp, None)]);

    assert!(
        pixel(&sw, 10, 10)[0] > pixel(&sw, 10, 10)[2],
        "centre is red"
    );
    // 4px out along +x is roughly halfway to the radius: neither stop.
    let mid = pixel(&sw, 14, 10);
    assert!(
        colour_distance(mid, pixel(&sw, 10, 10)) > 20
            && colour_distance(mid, pixel(&sw, 10, 2)) > 20,
        "the midpoint is a blend, not one of the stops: {mid:?}"
    );
    // Past the radius the last stop holds — the far corner is the rim colour.
    assert!(
        pixel(&sw, 1, 19)[2] > pixel(&sw, 1, 19)[0],
        "beyond the radius the last stop clamps: {:?}",
        pixel(&sw, 1, 19)
    );
}

/// An empty stop list is a visual no-op rather than a panic in the paint walk:
/// the shape draws transparent, and the backend still counted the gradient it
/// sampled. Honest, because "you gave me no colours" must not look like "I
/// drew something".
#[test]
fn a_brush_with_no_stops_draws_nothing_rather_than_panicking() {
    let ramp = Brush::linear(&[], Point::new(0.0, 0.0), Point::new(20.0, 0.0));
    let sw = render(vec![node(rect(0.0, 0.0, 20.0, 20.0), ramp, None)]);
    assert_eq!(alpha(&sw, 10, 10), 0, "no colour anywhere means no ink");
    assert_eq!(sw.gradients_drawn, 1, "the request still went through");
}

// ---------------------------------------------------------------------------
// A4 — shadows
// ---------------------------------------------------------------------------

const CARD: Rect = Rect {
    x0: 2.0,
    y0: 2.0,
    x1: 18.0,
    y1: 10.0,
};

fn cast_down() -> Shadow {
    Shadow {
        color: Color::rgba(0, 0, 0, 255),
        blur: 2.0,
        offset_x: 0.0,
        offset_y: 6.0,
        spread: 0.0,
    }
}

/// A shadow reaches past the shape that cast it, and only when requested.
///
/// The probe sits *below* the card, where no part of the card itself is drawn:
/// inked with a shadow, empty without one. Because the buffer starts
/// transparent and the shadow's colour is opaque black, alpha is enough — a
/// probe inside empty space cannot be mistaken for anything else.
#[test]
fn a_shadow_inks_the_pixels_past_the_shape_that_cast_it() {
    let sw = render(vec![node(CARD, Brush::Solid(RED), Some(cast_down()))]);
    assert_eq!(sw.shadows_drawn, 1, "the blur ran");
    assert!(
        alpha(&sw, 10, 14) > 0,
        "six px below the card's bottom edge the offset silhouette lands"
    );
    assert!(
        alpha(&sw, 10, 20) == 0,
        "far enough away even the blur does not reach"
    );

    // Control: the same card, no shadow. Without this the probe above could
    // be reading stray ink and would still pass.
    let bare = render(vec![node(CARD, Brush::Solid(RED), None)]);
    assert_eq!(bare.shadows_drawn, 0);
    assert_eq!(
        alpha(&bare, 10, 14),
        0,
        "nothing below the card when nobody asked for a shadow"
    );
}

/// A zero-alpha shadow asks for nothing, so it draws nothing — and says so,
/// rather than counting a shadow it did not composite.
#[test]
fn a_transparent_shadow_draws_nothing_and_says_so() {
    let invisible = Shadow {
        color: Color::rgba(0, 0, 0, 0),
        ..cast_down()
    };
    let sw = render(vec![node(CARD, Brush::Solid(RED), Some(invisible))]);
    assert_eq!(sw.shadows_drawn, 0);
    assert_eq!(alpha(&sw, 10, 14), 0);
    // The card itself is still there — the invisible shadow took nothing with it.
    assert_eq!(pixel(&sw, 10, 6), [RED.r, RED.g, RED.b, 255]);
}

/// A shadow is *blurred*, and the blur — not the silhouette — is what reaches
/// sideways. The probe sits just right of the card's own x-range, where a
/// hard-edged copy of the silhouette (which is what "elevation" used to mean
/// here) could never put ink. That is the difference this test exists to keep:
/// the previous mutation campaign killed the offset but *survived* the blur,
/// because every probe until now sat inside the silhouette's own box.
#[test]
fn a_shadow_is_blurred_and_the_blur_is_what_reaches_sideways() {
    let soft = Shadow {
        color: Color::rgba(0, 0, 0, 255),
        blur: 6.0,
        offset_x: 0.0,
        offset_y: 6.0,
        spread: 0.0,
    };
    let sw = render(vec![node(CARD, Brush::Solid(RED), Some(soft))]);
    // The card spans x 2..18; the silhouette the shadow offsets covers the
    // same range at y 8..16. x=20 is outside it — only the blur reaches there.
    let beside = alpha(&sw, 20, 13);
    println!(
        "  blur probe column: x=20 -> {beside}, x=23 -> {}",
        alpha(&sw, 23, 13)
    );
    assert!(
        beside > 0,
        "the blurred silhouette reaches past its own edge; a hard copy would not"
    );
    // And it falls off rather than ending in a step: further out is dimmer,
    // which a hard-edged silhouette can never be at any distance.
    assert!(
        beside > alpha(&sw, 23, 13),
        "the ink fades outward: {} then {}",
        beside,
        alpha(&sw, 23, 13)
    );
}

/// A shadow lands *behind* its shape: the silhouette itself is still the
/// fill's colour, not the shadow's. This is the ordering guarantee a widget
/// relies on when it calls `draw_shadow` and then fills the same box.
#[test]
fn a_card_still_paints_its_own_fill_where_its_shadow_is() {
    let sw = render(vec![node(CARD, Brush::Solid(RED), Some(cast_down()))]);
    assert_eq!(
        pixel(&sw, 10, 6),
        [RED.r, RED.g, RED.b, 255],
        "the fill wins inside the card's own box"
    );
    // Directly under the bottom edge is where the two must not be confused:
    // shadow-only territory, visibly not the fill colour.
    let below = pixel(&sw, 10, 13);
    assert!(
        below != [RED.r, RED.g, RED.b, 255],
        "that is shadow, not fill"
    );
    assert!(below[3] > 0, "and there is ink there");
}
