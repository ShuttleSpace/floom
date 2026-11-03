//! Pixel-level evidence for subtree transforms: a `GroupPush`/`GroupPop`
//! scope maps everything it wraps, and nothing it does not.
//!
//! Same discipline as `paths.rs` — pixel probes against a plain backdrop, with
//! the control case in the same test. "The rotated spot is ink" is worth
//! nothing on its own; it is worth something next to "the untransformed spot
//! is ink without the scope and empty with it".

use floom_renderer::{FrameInfo, render_once};
use floom_renderer_software::SoftwareRenderer;
use floom_scene::{Point, Rect, Scene, SceneNode};
use floom_types::{Color, Path, Stroke, Transform};

const BACKDROP: Color = Color::rgb(20, 30, 40);
const INK: Color = Color::rgb(220, 40, 40);

fn fill(bounds: Rect, color: Color) -> SceneNode {
    SceneNode::Rect {
        bounds,
        fill: floom_types::Brush::from(color),
        radius: 0.0,
        shadow: None,
    }
}

/// Renders a 24×24 buffer from a node list.
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

fn is_ink(sw: &SoftwareRenderer, x: u32, y: u32) -> bool {
    sw.pixel_rgba(x, y)
        .map(|p| p[0].abs_diff(INK.r) < 12 && p[1].abs_diff(INK.g) < 12)
        .unwrap_or(false)
}

fn alpha(sw: &SoftwareRenderer, x: u32, y: u32) -> u8 {
    sw.pixel_rgba(x, y).expect("probe inside the buffer")[3]
}

// ---------------------------------------------------------------------------
// rotation
// ---------------------------------------------------------------------------

/// A quarter turn about the buffer centre maps a rect from the left half onto
/// the top half: the new spot is ink, the old one is not, and the control
/// scene without the scope paints the old spot. That triple is the whole
/// claim — the scope maps, and nothing else does.
#[test]
fn a_quarter_turn_moves_the_picture_to_where_rotation_says() {
    // Rect on the left edge, centred vertically: [1..6]×[10..14].
    let bounds = Rect {
        x0: 1.0,
        y0: 10.0,
        x1: 6.0,
        y1: 14.0,
    };
    let scope = Transform::rotate_around(std::f64::consts::FRAC_PI_2, Point::new(12.0, 12.0));
    // Screen coordinates have y down, so +90° carries a point left of the
    // pivot to a point *above* it: the rect's centre (3.5, 12) maps to
    // (12, 3.5), and the whole rect to [10..14]×[1..6].
    let rotated = render(vec![
        SceneNode::GroupPush { transform: scope },
        fill(bounds, INK),
        SceneNode::GroupPop,
    ]);
    assert!(
        is_ink(&rotated, 12, 3),
        "the rotated rect paints where the rotation puts it"
    );
    assert!(
        alpha(&rotated, 3, 11) == 0,
        "the rect's original spot is empty: the scope moved the picture"
    );

    // Control: the same rect without the scope paints the original spot.
    let plain = render(vec![fill(bounds, INK)]);
    assert!(
        is_ink(&plain, 3, 11),
        "control: without the scope the rect is where it was declared"
    );
}

/// A scope must end where its marker says. A rect painted *after* the pop
/// stays put even while the scope moved everything inside it.
#[test]
fn a_group_pop_ends_the_scope() {
    let scope = Transform::rotate_around(std::f64::consts::FRAC_PI_2, Point::new(12.0, 12.0));
    let inside = Rect {
        x0: 1.0,
        y0: 10.0,
        x1: 6.0,
        y1: 14.0,
    };
    let outside = Rect {
        x0: 18.0,
        y0: 10.0,
        x1: 23.0,
        y1: 14.0,
    };
    let sw = render(vec![
        SceneNode::GroupPush { transform: scope },
        fill(inside, INK),
        SceneNode::GroupPop,
        fill(outside, INK),
    ]);
    assert!(
        is_ink(&sw, 20, 11),
        "the node after the pop paints where it was declared: the scope ended"
    );
}

// ---------------------------------------------------------------------------
// scaling
// ---------------------------------------------------------------------------

/// Scaling by two about the rect's own top-left doubles the picture's extent:
/// a probe beyond the original edge is ink under the scope and empty without.
#[test]
fn a_scale_doubles_the_picture_beyond_its_old_edge() {
    // Rect [2..6]×[2..6]; scale 2 about (2,2) maps it to [2..10]×[2..10].
    let bounds = Rect {
        x0: 2.0,
        y0: 2.0,
        x1: 6.0,
        y1: 6.0,
    };
    let scope = Transform::scale_around(2.0, 2.0, Point::new(2.0, 2.0));
    let scaled = render(vec![
        SceneNode::GroupPush { transform: scope },
        fill(bounds, INK),
        SceneNode::GroupPop,
    ]);
    assert!(
        is_ink(&scaled, 8, 8),
        "the scaled rect covers ground beyond its original edge"
    );
    // Control: unscaled, that ground is empty.
    let plain = render(vec![fill(bounds, INK)]);
    assert!(alpha(&plain, 8, 8) == 0, "control: unscaled it stops at 6");
}

/// An identity scope draws exactly what an absent scope draws — the fast path
/// and the scope machinery agree, and a scope that degrades to identity is not
/// observable in pixels.
#[test]
fn an_identity_scope_is_pixel_equivalent_to_no_scope() {
    let bounds = Rect {
        x0: 4.0,
        y0: 8.0,
        x1: 14.0,
        y1: 18.0,
    };
    let scoped = render(vec![
        SceneNode::GroupPush {
            transform: Transform::IDENTITY,
        },
        fill(bounds, INK),
        SceneNode::GroupPop,
    ]);
    let plain = render(vec![fill(bounds, INK)]);
    assert_eq!(
        scoped.pixels(),
        plain.pixels(),
        "identity scope and no scope draw the same buffer"
    );
}

// ---------------------------------------------------------------------------
// nesting and the scope/clip interaction
// ---------------------------------------------------------------------------

/// Two nested scopes compose: rotate 90° then scale 2× about the same pivot is
/// the product transform, probed where only the composition lands.
#[test]
fn nested_scopes_compose_like_their_product() {
    let pivot = Point::new(12.0, 12.0);
    let bounds = Rect {
        x0: 1.0,
        y0: 10.0,
        x1: 6.0,
        y1: 14.0,
    };
    // Scale is *inside* (pushed second), so it applies first: the rect
    // stretches away from the pivot (×1.5 about (12,12) → [−4.5..3]×[9..15]),
    // then the rotation carries it up. The product covers [9..15]×[−4.5..3].
    let nested = render(vec![
        SceneNode::GroupPush {
            transform: Transform::rotate_around(std::f64::consts::FRAC_PI_2, pivot),
        },
        SceneNode::GroupPush {
            transform: Transform::scale_around(1.5, 1.5, pivot),
        },
        fill(bounds, INK),
        SceneNode::GroupPop,
        SceneNode::GroupPop,
    ]);
    assert!(
        is_ink(&nested, 9, 2),
        "the composed scope places the picture where the product says"
    );
    // Control: rotation alone covers [10..14]×[1..6] — the composed picture
    // reaches x=9 only because the inner scale stretched it first.
    let rotated_only = render(vec![
        SceneNode::GroupPush {
            transform: Transform::rotate_around(std::f64::consts::FRAC_PI_2, pivot),
        },
        fill(bounds, INK),
        SceneNode::GroupPop,
    ]);
    assert!(
        alpha(&rotated_only, 9, 2) == 0,
        "control: rotation alone stops at x=10 — the composition needed both"
    );
}

/// A clip opened inside a scope is a window in that scope's space: rotating a
/// clip's box must carry the window with the content, not leave it behind.
#[test]
fn a_clip_inside_a_scope_travels_with_the_transform() {
    use floom_types::{ClipRegion, ClipShape, resolve_clip};
    // A rect on the left; a box clip covering only its right half, in scope
    // space. Rotated a quarter turn about the buffer centre, the whole
    // arrangement lands on the bottom half — the clip must land with it.
    let bounds = Rect {
        x0: 1.0,
        y0: 10.0,
        x1: 6.0,
        y1: 14.0,
    };
    let pivot = Point::new(12.0, 12.0);
    let scope = Transform::rotate_around(std::f64::consts::FRAC_PI_2, pivot);
    // In scope space the rect sits at [1..6]×[10..14]; the clip is its left
    // half, x ≤ 3.5. Rotated, the full rect lands on [10..14]×[1..6] and the
    // clipped half on [10..14]×[1..3.5].
    let clip = Rect {
        x0: 1.0,
        y0: 10.0,
        x1: 3.5,
        y1: 14.0,
    };
    let sw = render(vec![
        SceneNode::GroupPush { transform: scope },
        SceneNode::ClipPush {
            region: ClipRegion::Rect(clip),
        },
        fill(bounds, INK),
        SceneNode::ClipPop,
        SceneNode::GroupPop,
    ]);
    // The clip admits the left half of the rect; rotated, that half covers
    // [10..14]×[1..3.5]. Probe its middle.
    assert!(
        is_ink(&sw, 12, 2),
        "the clipped half paints inside the scope"
    );
    // The right half (x > 3.5) is clipped away in scope space; rotated it
    // would cover y 3.5..6 of the same column — empty, because the window
    // travelled with the content instead of staying behind.
    assert!(
        alpha(&sw, 12, 5) == 0,
        "the clipped-out half stays out: the window moved with the content"
    );
}

/// A stroked path inside a scope maps its *points*, not just its boxes: a
/// horizontal stroke left of the pivot, quarter-turned, stands vertically
/// above the pivot. The control scene without the scope keeps it on the
/// pivot's row — so the probe's difference is the scope's doing.
#[test]
fn a_path_inside_a_scope_maps_its_points() {
    let pivot = Point::new(12.0, 12.0);
    let scope = Transform::rotate_around(std::f64::consts::FRAC_PI_2, pivot);
    // A horizontal stroke from (4,12) to (10,12): left of the pivot.
    let mut path = Path::new();
    path.move_to(Point::new(4.0, 12.0))
        .line_to(Point::new(10.0, 12.0));
    let nodes = || {
        vec![
            SceneNode::GroupPush { transform: scope },
            SceneNode::stroked_path(path.clone(), Stroke::new(2.0, INK)),
            SceneNode::GroupPop,
        ]
    };
    let rotated = render(nodes());
    // y is down: the stroke's points map to (12,4)..(12,10), so (12,6) —
    // the rotated midpoint — is ink.
    assert!(
        is_ink(&rotated, 12, 6),
        "the rotated stroke stands above the pivot"
    );
    // Control: without the scope the stroke lies flat where it was declared;
    // (12,6) is backdrop, (7,12) is ink.
    let flat = render(vec![SceneNode::stroked_path(path, Stroke::new(2.0, INK))]);
    assert!(
        !is_ink(&flat, 12, 6),
        "control: without the scope the probe spot is backdrop"
    );
    assert!(
        is_ink(&flat, 7, 12),
        "control: without the scope the stroke is where it was declared"
    );
}
