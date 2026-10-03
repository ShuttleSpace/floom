//! Pixel-level evidence for MOTION-GAP A4's back half: general filters.
//!
//! The claim under test is not "there is a blur somewhere" — shadows already
//! blur, and a shadow is not a filter. The claim is that a band of *drawn
//! picture* can be post-processed as a unit, which is a different thing from
//! decorating one shape: it has to see the composite of several nodes at once,
//! and it must leave everything outside its own scope exactly as it was.
//!
//! So every probe here is paired with the control that makes it fail-safe:
//!
//! - "ink crossed the silhouette edge" is worthless alone — next to "the same
//!   rect without the scope puts nothing at that pixel" it becomes evidence
//!   that a filter ran.
//! - "the background edge is still sharp" is what separates a *content* filter
//!   from a *backdrop* one. Nothing else does: both blur, and both would pass
//!   a probe that only asks whether pixels changed.
//! - per-axis radii get one probe each, because a blur that collapsed them
//!   into a single number would still pass every one-axis assertion.

use floem_renderer::{FrameInfo, render_once};
use floem_renderer_software::SoftwareRenderer;
use floem_scene::{Rect, Scene, SceneNode};
use floem_types::{Color, Filter};

const BLACK: Color = Color::rgb(0, 0, 0);
const WHITE: Color = Color::rgb(255, 255, 255);

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
    Rect { x0, y0, x1, y1 }
}

fn filled(bounds: Rect, color: Color) -> SceneNode {
    SceneNode::Rect {
        bounds,
        fill: color.into(),
        radius: 0.0,
        shadow: None,
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

/// A sharp vertical edge down the middle: white on the left, black on the
/// right. The probe that tells a content filter from a backdrop one, because
/// only the latter is allowed to smear it.
fn split_background() -> Vec<SceneNode> {
    vec![
        filled(rect(0.0, 0.0, 12.0, 24.0), WHITE),
        filled(rect(12.0, 0.0, 24.0, 24.0), BLACK),
    ]
}

#[test]
fn a_filter_scope_blurs_the_picture_it_wraps() {
    // A black slab over the left half, wrapped in a blur. The only way ink
    // appears to the *right* of x = 12 is that something smeared it there.
    let sw = render(vec![
        SceneNode::FilterPush {
            filter: Filter::blur(6.0),
        },
        filled(rect(0.0, 0.0, 12.0, 24.0), BLACK),
        SceneNode::FilterPop,
    ]);

    assert!(alpha(&sw, 13, 12) > 20, "ink crossed the silhouette edge");
    // Falling off with distance is what separates a blur from a hard-edged
    // copy drawn two pixels wider.
    assert!(
        alpha(&sw, 11, 12) > alpha(&sw, 13, 12),
        "the edge fades outward"
    );
    assert!(
        alpha(&sw, 13, 12) > alpha(&sw, 16, 12),
        "and keeps fading outward"
    );
    assert_eq!(alpha(&sw, 20, 12), 0, "but not forever — the blur ends");
    assert_eq!(sw.filters_drawn, 1, "one scope was composited");
}

#[test]
fn the_same_picture_without_the_scope_puts_nothing_past_the_edge() {
    // The control for the probe above: without the scope the slab ends exactly
    // where it was drawn, so `alpha(13) > 20` above is evidence and not luck.
    let sw = render(vec![filled(rect(0.0, 0.0, 12.0, 24.0), BLACK)]);
    assert_eq!(alpha(&sw, 11, 12), 255);
    assert_eq!(alpha(&sw, 13, 12), 0);
    assert_eq!(sw.filters_drawn, 0);
}

#[test]
fn a_filter_scope_leaves_the_pixels_behind_it_sharp() {
    // This is the whole difference between `Modifier.blur` and
    // `BackdropFilter`. The background carries a hard edge at x = 12; the
    // filtered slab is drawn *over* it. A content filter must not touch it.
    let mut nodes = split_background();
    nodes.push(SceneNode::FilterPush {
        filter: Filter::blur(6.0),
    });
    nodes.push(filled(rect(4.0, 8.0, 20.0, 16.0), Color::rgb(220, 40, 40)));
    nodes.push(SceneNode::FilterPop);
    let sw = render(nodes);

    // Probed at y = 0: eight rows above the slab, i.e. outside the blur's
    // reach, so this pixel can only be the background as it was drawn.
    assert_eq!(
        pixel(&sw, 11, 0),
        [255, 255, 255, 255],
        "the white side of the backdrop is untouched"
    );
    assert_eq!(
        pixel(&sw, 13, 0),
        [0, 0, 0, 255],
        "and so is the black side — the edge did not smear"
    );
}

#[test]
fn a_backdrop_filter_blurs_what_was_already_drawn() {
    // The same hard edge, this time as the filter's *input*: the node asks
    // for the picture behind it, inside its own rect, to be blurred.
    let mut nodes = split_background();
    nodes.push(SceneNode::BackdropFilter {
        rect: rect(6.0, 6.0, 18.0, 18.0),
        filter: Filter::blur(6.0),
    });
    let sw = render(nodes);

    // One pixel left of the edge, inside the rect: was pure white, is now
    // pulled toward the black on the other side.
    let inside = pixel(&sw, 11, 12);
    assert_ne!(inside, [255, 255, 255, 255], "the edge smeared");
    assert!(inside[0] < 255, "white picked up ink from the black side");
    assert!(inside[0] > 0, "and did not simply become black");
    // Still opaque: a backdrop blur mixes what is there, it does not punch a
    // hole in it.
    assert_eq!(inside[3], 255);
    // And it is confined to its rect — the same probe outside it is untouched.
    assert_eq!(pixel(&sw, 11, 0), [255, 255, 255, 255]);
    assert_eq!(sw.backdrops_drawn, 1, "one backdrop was applied");
}

#[test]
fn the_same_edge_without_a_backdrop_filter_stays_sharp() {
    // The control for the two probes above.
    let sw = render(split_background());
    assert_eq!(pixel(&sw, 11, 12), [255, 255, 255, 255]);
    assert_eq!(pixel(&sw, 13, 12), [0, 0, 0, 255]);
    assert_eq!(sw.backdrops_drawn, 0);
}

#[test]
fn the_two_axes_blur_independently() {
    // A blur that collapsed both radii into one number would pass either half
    // of this test and fail the other.
    let square = filled(rect(8.0, 8.0, 16.0, 16.0), BLACK);

    let horizontal = render(vec![
        SceneNode::FilterPush {
            filter: Filter::blur_xy(8.0, 0.0),
        },
        square.clone(),
        SceneNode::FilterPop,
    ]);
    assert!(
        alpha(&horizontal, 20, 12) > 20,
        "a horizontal-only blur spreads sideways"
    );
    assert_eq!(alpha(&horizontal, 12, 20), 0, "and not at all vertically");

    let vertical = render(vec![
        SceneNode::FilterPush {
            filter: Filter::blur_xy(0.0, 8.0),
        },
        square,
        SceneNode::FilterPop,
    ]);
    assert!(
        alpha(&vertical, 12, 20) > 20,
        "a vertical-only blur spreads downward"
    );
    assert_eq!(alpha(&vertical, 20, 12), 0, "and not at all sideways");
}

#[test]
fn an_identity_filter_leaves_the_picture_exactly_as_it_was() {
    // A zero radius has nothing to smear, and the round trip through a layer
    // must not cost a pixel: the whole point of `is_identity` is that a caller
    // can ask for "no filter" and get the picture it would have had anyway.
    let plain = render(vec![filled(rect(2.0, 2.0, 10.0, 10.0), BLACK)]);
    let identity = render(vec![
        SceneNode::FilterPush {
            filter: Filter::blur(0.0),
        },
        filled(rect(2.0, 2.0, 10.0, 10.0), BLACK),
        SceneNode::FilterPop,
    ]);
    for y in 0..24 {
        for x in 0..24 {
            assert_eq!(
                pixel(&identity, x, y),
                pixel(&plain, x, y),
                "every pixel is unchanged at ({x}, {y})"
            );
        }
    }
}

#[test]
fn a_filter_scope_stays_inside_the_clip_that_was_in_force() {
    // The blur spreads past where its content was drawn, and that spread has
    // to be clipped too — otherwise a filter would leak out of the card it
    // belongs to.
    let sw = render(vec![
        SceneNode::ClipPush {
            region: floem_types::ClipRegion::Rect(rect(0.0, 0.0, 8.0, 24.0)),
        },
        SceneNode::FilterPush {
            filter: Filter::blur(6.0),
        },
        filled(rect(0.0, 0.0, 24.0, 24.0), BLACK),
        SceneNode::FilterPop,
        SceneNode::ClipPop,
    ]);
    // Probed at x = 0, not x = 7: the clip edge is where the layer's own ink
    // stops, so the blur's falloff reaches *inward* from it too. Far enough
    // inside, the layer is fully opaque.
    assert_eq!(alpha(&sw, 0, 12), 255, "inside the clip it paints");
    assert_eq!(alpha(&sw, 9, 12), 0, "and not one pixel past it");
}

#[test]
fn a_filter_scope_composites_over_what_preceded_it() {
    // The layer is transparent where nothing was drawn, so what was underneath
    // shows through — a filter is not an opaque card.
    let sw = render(vec![
        filled(rect(0.0, 0.0, 24.0, 24.0), WHITE),
        SceneNode::FilterPush {
            filter: Filter::blur(6.0),
        },
        filled(rect(0.0, 0.0, 12.0, 24.0), BLACK),
        SceneNode::FilterPop,
    ]);
    // Deep inside the slab the blurred layer is still opaque black...
    assert!(pixel(&sw, 4, 12)[0] < 60, "the slab still reads as black");
    // ...and far outside it the layer contributes nothing, so the white behind
    // is what shows.
    assert_eq!(pixel(&sw, 20, 12), [255, 255, 255, 255]);
}

#[test]
fn a_blur_keeps_the_inks_own_colour_at_its_partially_covered_edge() {
    // The premultiplied-alpha trap, and the one a straight-channel blur falls
    // into: outside a red slab the buffer holds *transparent* pixels, whose
    // RGB happens to be zero. Averaging channels directly therefore averages
    // that transparent black in as if it were ink, and the edge comes out dark
    // red instead of red-at-lower-coverage. Blurring premultiplied and
    // dividing back out is what keeps the hue.
    let sw = render(vec![
        SceneNode::FilterPush {
            filter: Filter::blur(6.0),
        },
        filled(rect(0.0, 0.0, 12.0, 24.0), Color::rgb(255, 0, 0)),
        SceneNode::FilterPop,
    ]);

    let edge = pixel(&sw, 14, 12);
    assert!(edge[3] > 20 && edge[3] < 235, "partially covered: {edge:?}");
    assert!(
        edge[0] > 220,
        "the red stayed red instead of darkening toward black: {edge:?}"
    );
    assert_eq!(edge[1], 0, "and picked up no other channel");
}

#[test]
fn a_blur_weights_what_it_mixes_by_how_opaque_it_is() {
    // The case that actually separates a premultiplied blur from a straight
    // one. A single colour over transparency does not: dividing the averaged
    // channels back out by the averaged alpha undoes the mistake, so both
    // paths give the same answer and the bug hides. Two colours where *one is
    // translucent* do not cancel — a straight blur gives the translucent one
    // equal say (b = 195 here), a premultiplied one gives it half (b = 98).
    let sw = render(vec![
        SceneNode::FilterPush {
            filter: Filter::blur(8.0),
        },
        filled(rect(0.0, 0.0, 12.0, 24.0), Color::rgb(255, 0, 0)),
        filled(rect(12.0, 0.0, 24.0, 24.0), Color::rgba(0, 0, 255, 128)),
        SceneNode::FilterPop,
    ]);

    let mix = pixel(&sw, 12, 12);
    assert!(mix[0] > 0 && mix[2] > 0, "both sides contributed: {mix:?}");
    assert!(
        mix[2] < 140,
        "the half-transparent blue carries half the weight, not all of it: {mix:?}"
    );
}
