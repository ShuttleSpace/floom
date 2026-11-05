// SPDX-License-Identifier: MIT
//! Pixel-level evidence for the two capabilities S2 adds to the scene: vector
//! paths (fill and stroke) and clipping.
//!
//! Every test here is a **pixel probe against a plain background**, with the
//! control case in the same test wherever a negative assertion could otherwise
//! pass because nothing was painted at all. "The corner pixel is not the fill
//! colour" is worth nothing on its own — it is worth something next to the same
//! probe over a shape that must produce the colour.

use floom_renderer::{FrameInfo, render_once};
use floom_renderer_software::SoftwareRenderer;
use floom_scene::{Point, Rect, Scene, SceneNode};
use floom_types::{ClipRegion, ClipShape, Color, LineCap, LineJoin, Path, Stroke, resolve_clip};

const BACKDROP: Color = Color::rgb(20, 30, 40);
const INK: Color = Color::rgb(220, 40, 40);
const MARK: Color = Color::rgb(40, 200, 90);

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
    Rect { x0, y0, x1, y1 }
}

fn fill(bounds: Rect, color: Color) -> SceneNode {
    SceneNode::Rect {
        bounds,
        fill: floom_types::Brush::from(color),
        radius: 0.0,
        shadow: None,
    }
}

/// Renders one scene into a fresh 24×24 buffer and hands back the renderer.
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

fn alpha(sw: &SoftwareRenderer, x: u32, y: u32) -> u8 {
    sw.pixel_rgba(x, y).expect("probe inside the buffer")[3]
}

fn is_ink(sw: &SoftwareRenderer, x: u32, y: u32) -> bool {
    sw.pixel_rgba(x, y)
        .map(|p| p[0].abs_diff(INK.r) < 12 && p[1].abs_diff(INK.g) < 12)
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// A1 — paths
// ---------------------------------------------------------------------------

/// A circle drawn as a path is round. The probe is the bounding box's own
/// corner: a `Rect` with the same bounds covers it, a circle does not — which
/// is precisely the difference `Rect { radius }` cannot express for a
/// non-square box.
#[test]
fn a_filled_circle_leaves_the_corners_of_its_box_empty() {
    let circle = Path::circle(Point::new(12.0, 12.0), 8.0);
    let sw = render(vec![SceneNode::filled_path(circle, INK)]);
    assert_eq!(sw.paths_drawn, 1);

    assert!(is_ink(&sw, 12, 12), "the centre is inside the circle");
    assert!(
        !is_ink(&sw, 5, 5),
        "the box corner (5,5) is √2·(8−6.5) outside the circle"
    );
    assert_eq!(
        alpha(&sw, 5, 5),
        0,
        "nothing should be painted there at all"
    );

    // Control: the same bounds as a rect do fill that corner, so the probe
    // above is measuring the shape and not an empty frame.
    let boxed = render(vec![fill(rect(4.0, 4.0, 20.0, 20.0), INK)]);
    assert!(is_ink(&boxed, 5, 5), "control: a rect covers its corner");
}

/// A circle's edge is anti-aliased: the pixel straddling the arc is a partial
/// blend, and the pixel fully inside is not.
#[test]
fn a_circle_edge_is_antialiased() {
    let circle = Path::circle(Point::new(12.0, 12.0), 8.0);
    let sw = render(vec![SceneNode::filled_path(circle, INK)]);
    assert_eq!(alpha(&sw, 12, 12), 255, "interior is solid");
    // The arc crosses pixel (17,17) diagonally — the pixel that straddles a
    // curve is the one whose coverage has to be fractional.
    let edge = alpha(&sw, 17, 17);
    assert!(edge > 0 && edge < 255, "arc pixel alpha {edge}");
    assert_eq!(alpha(&sw, 20, 20), 0, "past the arc is clear");
}

/// A cubic that bulges outside its endpoints really is filled there. A
/// straight-line approximation would leave the bulge empty — so the probe is
/// the middle of the bulge, and the control is the chord it replaces.
#[test]
fn a_cubic_bulge_is_filled_where_the_chord_is_not() {
    let mut bulge = Path::new();
    bulge
        .move_to(Point::new(4.0, 18.0))
        .cubic_to(
            Point::new(4.0, 2.0),
            Point::new(20.0, 2.0),
            Point::new(20.0, 18.0),
        )
        .line_to(Point::new(20.0, 22.0))
        .line_to(Point::new(4.0, 22.0))
        .close();
    let sw = render(vec![SceneNode::filled_path(bulge, INK)]);
    // The curve's apex sits around y = 7 for these control points; the chord
    // between the endpoints is at y = 18.
    assert!(
        is_ink(&sw, 12, 8),
        "the bulge should be filled near its apex"
    );

    let mut chord = Path::new();
    chord
        .move_to(Point::new(4.0, 18.0))
        .line_to(Point::new(20.0, 18.0))
        .line_to(Point::new(20.0, 22.0))
        .line_to(Point::new(4.0, 22.0))
        .close();
    let control = render(vec![SceneNode::filled_path(chord, INK)]);
    assert!(
        !is_ink(&control, 12, 8),
        "control: the same probe over the chord is empty"
    );
}

/// A stroke paints the band around the path and nothing else: the band's
/// interior is solid, a point off the band but inside the path's bounding box
/// is not.
#[test]
fn a_stroke_paints_a_band_and_not_the_interior() {
    let line = Path::rect(rect(6.0, 6.0, 18.0, 18.0));
    let sw = render(vec![SceneNode::stroked_path(line, Stroke::new(4.0, MARK))]);
    let mark_at = |x: u32, y: u32| {
        sw.pixel_rgba(x, y)
            .map(|p| p[0].abs_diff(MARK.r) < 12)
            .unwrap_or(false)
    };
    assert!(mark_at(12, 6), "the top edge is stroked");
    assert!(mark_at(6, 12), "the left edge is stroked");
    assert!(!mark_at(12, 12), "the middle of the square is not");
}

/// Cap geometry reaches the pixel grid: a round cap paints beyond the
/// endpoint, a butt cap does not. Without the control this would also pass if
/// the stroke drew nothing at all.
#[test]
fn a_round_cap_paints_past_the_endpoint_where_a_butt_cap_stops() {
    let segment = || {
        let mut path = Path::new();
        path.move_to(Point::new(4.0, 12.0))
            .line_to(Point::new(16.0, 12.0));
        path
    };
    let round = render(vec![SceneNode::stroked_path(
        segment(),
        Stroke::new(6.0, INK).with_cap(LineCap::Round),
    )]);
    let butt = render(vec![SceneNode::stroked_path(
        segment(),
        Stroke::new(6.0, INK).with_cap(LineCap::Butt),
    )]);
    // Centre (18.5, 12.5) is 2.55 from the endpoint — inside the 3.0
    // half-width disc and beyond the segment's band.
    assert!(is_ink(&round, 18, 12), "the round cap reaches past the end");
    assert!(!is_ink(&butt, 18, 12), "the butt cap stops at the end");
    // Both paint the segment itself, so the difference is the cap and not the
    // whole draw call.
    assert!(is_ink(&round, 10, 12) && is_ink(&butt, 10, 12));
}

/// A miter join fills the corner notch that a bevel cuts off.
#[test]
fn a_miter_join_fills_where_a_bevel_join_leaves_a_gap() {
    let corner = || {
        let mut path = Path::new();
        path.move_to(Point::new(4.0, 18.0))
            .line_to(Point::new(18.0, 18.0))
            .line_to(Point::new(18.0, 4.0));
        path
    };
    let miter = render(vec![SceneNode::stroked_path(
        corner(),
        Stroke::new(6.0, INK).with_join(LineJoin::Miter),
    )]);
    let bevel = render(vec![SceneNode::stroked_path(
        corner(),
        Stroke::new(6.0, INK).with_join(LineJoin::Bevel),
    )]);
    // The corner turns from +x to −y, so the outside of the turn is the
    // bottom-right quadrant. (20,20)'s centre is 3.54 from the corner, along
    // the bisector: past the 3.0 half-width a bevel stops at, inside the 4.24
    // a miter reaches on a right angle.
    assert!(is_ink(&miter, 20, 20), "the miter fills the notch");
    assert!(!is_ink(&bevel, 20, 20), "the bevel cuts it");
    // Both fill the corner itself, so the difference really is the join.
    assert!(is_ink(&miter, 18, 18) || is_ink(&miter, 17, 17));
    assert!(is_ink(&bevel, 17, 17));
}

// ---------------------------------------------------------------------------
// A2 — clipping
// ---------------------------------------------------------------------------

fn clip_to(shape: ClipShape, bounds: Rect) -> SceneNode {
    SceneNode::ClipPush {
        region: resolve_clip(&shape, bounds).expect("a non-empty clip box"),
    }
}

/// A rectangle clip cuts what is painted outside it, and the *unclipped*
/// control shows the same node painting there.
#[test]
fn a_rect_clip_cuts_what_the_control_lets_through() {
    let scene = |clip: bool| {
        let mut nodes = vec![fill(rect(0.0, 0.0, 24.0, 24.0), BACKDROP)];
        if clip {
            nodes.push(clip_to(ClipShape::Box, rect(0.0, 0.0, 12.0, 24.0)));
        }
        nodes.push(fill(rect(0.0, 0.0, 24.0, 12.0), INK));
        if clip {
            nodes.push(SceneNode::ClipPop);
        }
        nodes
    };
    let clipped = render(scene(true));
    let unclipped = render(scene(false));

    assert!(is_ink(&clipped, 5, 5), "inside the clip the fill survives");
    assert!(
        is_ink(&unclipped, 18, 5),
        "control: without the clip the fill reaches here"
    );
    assert_eq!(
        clipped.pixel_rgba(18, 5).unwrap(),
        [BACKDROP.r, BACKDROP.g, BACKDROP.b, BACKDROP.a],
        "with the clip the backdrop is what shows"
    );
}

/// A rounded clip cuts its corners; a square clip over the same box does not.
/// The pair is the whole point — "the corner is not the fill colour" alone
/// would also hold for a clip that admitted nothing.
#[test]
fn a_rounded_clip_cuts_its_corner_where_a_square_clip_does_not() {
    let scene = |shape: ClipShape| {
        vec![
            fill(rect(0.0, 0.0, 24.0, 24.0), BACKDROP),
            clip_to(shape, rect(2.0, 2.0, 22.0, 22.0)),
            fill(rect(0.0, 0.0, 24.0, 24.0), INK),
            SceneNode::ClipPop,
        ]
    };
    let rounded = render(scene(ClipShape::RoundedBox { radius: 8.0 }));
    let square = render(scene(ClipShape::Box));

    assert!(is_ink(&rounded, 12, 12), "the middle is admitted");
    assert!(
        is_ink(&square, 3, 3),
        "control: a square clip admits its own corner"
    );
    assert_eq!(
        rounded.pixel_rgba(3, 3).unwrap(),
        [BACKDROP.r, BACKDROP.g, BACKDROP.b, BACKDROP.a],
        "the rounded corner is cut away"
    );
    // And the cut is anti-aliased. Alpha cannot show it — the backdrop under
    // the ink is opaque — so the evidence is the blend itself: the pixel the
    // arc crosses carries neither colour in full. (4,4) is where the 45° point
    // of the arc, centre (10,10) radius 8, lands.
    let arc = rounded.pixel_rgba(4, 4).expect("in bounds");
    assert!(
        arc[0] > BACKDROP.r && arc[0] < INK.r,
        "arc pixel should be a blend of backdrop and ink, got {arc:?}"
    );
}

/// A path clip admits exactly its path. The control is the same scene clipped
/// to the path's bounding box, which admits the corner the circle does not.
#[test]
fn a_path_clip_follows_the_path_not_its_bounding_box() {
    let scene = |shape: ClipShape| {
        vec![
            fill(rect(0.0, 0.0, 24.0, 24.0), BACKDROP),
            clip_to(shape, rect(2.0, 2.0, 22.0, 22.0)),
            fill(rect(0.0, 0.0, 24.0, 24.0), INK),
            SceneNode::ClipPop,
        ]
    };
    let circle = render(scene(ClipShape::Path(std::sync::Arc::new(Path::circle(
        Point::new(10.0, 10.0),
        10.0,
    )))));
    let boxed = render(scene(ClipShape::Box));

    assert!(is_ink(&circle, 12, 12), "the circle's centre is admitted");
    assert!(
        is_ink(&boxed, 5, 5),
        "control: the bounding box admits its corner"
    );
    assert_eq!(
        circle.pixel_rgba(3, 3).unwrap(),
        [BACKDROP.r, BACKDROP.g, BACKDROP.b, BACKDROP.a],
        "the circle's corner is cut away"
    );
}

/// A pop really restores: a node drawn after it is not clipped by a region
/// pushed before it. Without the pop the probe pixel would keep the backdrop.
#[test]
fn the_marker_after_a_pop_is_not_clipped() {
    let nodes = vec![
        fill(rect(0.0, 0.0, 24.0, 24.0), BACKDROP),
        clip_to(ClipShape::Box, rect(0.0, 0.0, 10.0, 24.0)),
        fill(rect(0.0, 0.0, 24.0, 24.0), INK),
        SceneNode::ClipPop,
        fill(rect(12.0, 0.0, 24.0, 8.0), MARK),
    ];
    let sw = render(nodes);
    assert_eq!(alpha(&sw, 18, 4), 255, "the post-pop node painted");
    assert_eq!(
        sw.pixel_rgba(18, 4).unwrap()[1],
        MARK.g,
        "and it is its own colour, so it was not silently clipped away"
    );
    assert_eq!(
        sw.pixel_rgba(18, 16).unwrap(),
        [BACKDROP.r, BACKDROP.g, BACKDROP.b, BACKDROP.a],
        "while the earlier clip still governs what came before the pop"
    );
}

/// The contract the scene IR documents: a walk may end with regions still
/// pushed, and the *next* walk must not inherit them.
///
/// This is the failure a stack with no reset at walk start produces — the
/// second frame would be clipped by the first frame's shape, which is a bug
/// that only shows up on the frame after a clipped widget disappears.
#[test]
fn a_clip_left_pushed_at_the_end_of_a_walk_does_not_leak_into_the_next() {
    let mut scene = Scene::new();
    scene.begin_update();
    let _ = scene.insert(fill(rect(0.0, 0.0, 24.0, 24.0), BACKDROP));
    let _ = scene.insert(clip_to(ClipShape::Box, rect(0.0, 0.0, 8.0, 8.0)));
    // Deliberately no pop: the scene ends with the clip in force.
    scene.finish_update();

    let mut sw = SoftwareRenderer::new(24, 24);
    render_once(&mut sw, &FrameInfo::software(24, 24, 1), &scene);

    let mut second = Scene::new();
    second.begin_update();
    let _ = second.insert(fill(rect(0.0, 0.0, 24.0, 24.0), INK));
    second.finish_update();
    render_once(&mut sw, &FrameInfo::software(24, 24, 2), &second);

    assert!(
        is_ink(&sw, 18, 18),
        "frame two must not be clipped by frame one's leftover region"
    );
}

/// Two clips in a row intersect: neither one alone would hide the probe pixel.
#[test]
fn nested_clips_intersect() {
    let nodes = vec![
        fill(rect(0.0, 0.0, 24.0, 24.0), BACKDROP),
        clip_to(ClipShape::Box, rect(0.0, 0.0, 16.0, 24.0)),
        clip_to(ClipShape::Box, rect(0.0, 0.0, 24.0, 10.0)),
        fill(rect(0.0, 0.0, 24.0, 24.0), INK),
        SceneNode::ClipPop,
        SceneNode::ClipPop,
    ];
    let sw = render(nodes);
    assert!(is_ink(&sw, 8, 4), "the intersection is admitted");
    assert_eq!(
        sw.pixel_rgba(18, 4).unwrap(),
        [BACKDROP.r, BACKDROP.g, BACKDROP.b, BACKDROP.a],
        "past the first clip's right edge"
    );
    assert_eq!(
        sw.pixel_rgba(8, 18).unwrap(),
        [BACKDROP.r, BACKDROP.g, BACKDROP.b, BACKDROP.a],
        "past the second clip's bottom edge"
    );
}

/// A clip region resolved from a widget declaration is what the renderer ends
/// up pushing — pinned here so the two halves of the API cannot drift.
#[test]
fn a_resolved_clip_region_is_what_the_renderer_applies() {
    let resolved = resolve_clip(
        &ClipShape::RoundedBox { radius: 6.0 },
        rect(2.0, 2.0, 22.0, 22.0),
    )
    .expect("non-empty");
    assert_eq!(
        resolved,
        ClipRegion::RoundedRect {
            bounds: rect(2.0, 2.0, 22.0, 22.0),
            radius: 6.0,
        }
    );
}
