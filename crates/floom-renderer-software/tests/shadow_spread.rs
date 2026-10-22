//! Pixel-level evidence for `Shadow::spread`.
//!
//! `spread` is CSS's fourth `box-shadow` length: it dilates the silhouette
//! *before* the blur. The discipline matches `brush.rs` — every positive probe
//! is paired with the control that makes it fail-safe, because "this pixel is
//! inked" only means something next to "the same shadow without spread leaves
//! it clear".
//!
//! The failure this guards against is the plausible-looking wrong
//! implementation: folding spread into the blur radius instead of the shape.
//! That reads as "a fatter, softer shadow", which is a different effect, so one
//! of the tests below has to distinguish the two.

use floom_renderer::{FrameInfo, render_once};
use floom_renderer_software::SoftwareRenderer;
use floom_scene::{Rect, Scene, SceneNode};
use floom_types::{Brush, Color, Shadow};

/// A card with a hard edge, so any ink outside it is shadow.
const CARD: Rect = Rect {
    x0: 8.0,
    y0: 8.0,
    x1: 24.0,
    y1: 20.0,
};

fn render(nodes: Vec<SceneNode>) -> SoftwareRenderer {
    let mut scene = Scene::new();
    scene.begin_update();
    for node in nodes {
        let _ = scene.insert(node);
    }
    scene.finish_update();
    let mut sw = SoftwareRenderer::new(48, 48);
    render_once(&mut sw, &FrameInfo::software(48, 48, 1), &scene);
    sw
}

fn alpha(sw: &SoftwareRenderer, x: u32, y: u32) -> u8 {
    sw.pixel_rgba(x, y).expect("probe inside the buffer")[3]
}

fn card(shadow: Shadow) -> SceneNode {
    SceneNode::Rect {
        bounds: CARD,
        fill: Brush::Solid(Color::rgb(220, 40, 40)),
        radius: 0.0,
        shadow: Some(shadow),
    }
}

/// A shadow with no blur and no offset, so the silhouette stays concentric
/// with the shape and only dilation can move its edge.
///
/// Note this is not a *hard* edge: the backend's blur kernel is
/// `max(round(blur / 2), 1)` and it runs three passes, so `blur: 0.0` still
/// smears by roughly a pixel. The tests below therefore compare two shadows
/// that differ only in `spread` rather than probing for a crisp boundary.
fn hard(spread: f64) -> Shadow {
    Shadow {
        color: Color::rgba(0, 0, 0, 255),
        blur: 0.0,
        offset_x: 0.0,
        offset_y: 0.0,
        spread,
    }
}

#[test]
fn spread_grows_the_silhouette_past_the_shape() {
    // 3px out from the card's left edge, still inside the 48px canvas.
    let sw = render(vec![card(hard(3.0))]);
    assert_eq!(sw.shadows_drawn, 1, "the shadow ran");
    assert!(
        alpha(&sw, 5, 14) > 0,
        "spread=3 should ink 3px outside the card's left edge (x=5)"
    );
}

#[test]
fn the_control_without_spread_leaves_that_pixel_clear() {
    // Same shadow, spread=0. If this inks the pixel the probe above is
    // measuring the card's own fill bleeding, not the shadow.
    let sw = render(vec![card(hard(0.0))]);
    assert_eq!(
        alpha(&sw, 5, 14),
        0,
        "without spread a hard shadow must not reach outside the shape"
    );
}

#[test]
fn spread_moves_the_edge_further_than_blur_alone_at_equal_magnitude() {
    // Both 4. If spread were folded into the blur radius instead of dilating
    // the shape, these two renders would be identical.
    let spread_only = render(vec![card(hard(4.0))]);
    let blur_only = render(vec![card(Shadow {
        color: Color::rgba(0, 0, 0, 255),
        blur: 4.0,
        offset_x: 0.0,
        offset_y: 0.0,
        spread: 0.0,
    })]);

    let spread_alpha = alpha(&spread_only, 5, 14);
    let blur_alpha = alpha(&blur_only, 5, 14);
    assert!(
        spread_alpha > 200,
        "spread=4 dilates a solid silhouette out to x=4, so x=5 (1.5px \
         inside that edge) must be near-opaque (got {spread_alpha})"
    );
    assert!(
        blur_alpha < 60,
        "a 4px blur is soft and must leave x=4 barely inked (got \
         {blur_alpha}); if this were as strong as the spread probe, spread \
         would be a blur alias rather than a dilation"
    );
}

#[test]
fn negative_spread_erodes_the_silhouette_inward() {
    // Shadow-only renders, so the card's own fill cannot be mistaken for
    // shadow ink. Eroding by 4px pulls the silhouette's left edge from x=8 in
    // to x=12, so a probe just inside the card's own edge loses most of its
    // ink relative to the same shadow with no erosion.
    let shadow_only = |spread: f64| {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: CARD,
            fill: Brush::Solid(Color::TRANSPARENT),
            radius: 0.0,
            shadow: Some(hard(spread)),
        });
        scene.finish_update();
        let mut sw = SoftwareRenderer::new(48, 48);
        render_once(&mut sw, &FrameInfo::software(48, 48, 1), &scene);
        sw
    };

    let intact = alpha(&shadow_only(0.0), 10, 14);
    let eroded = alpha(&shadow_only(-4.0), 10, 14);
    assert!(
        intact > 200,
        "x=10 is inside the un-eroded silhouette, so it must be inked \
         (got {intact})"
    );
    assert!(
        eroded < intact / 2,
        "eroding by 4px moves the left edge to x=12, so x=10 must lose most \
         of its ink ({eroded} vs {intact} intact)"
    );
}

#[test]
fn reach_accounts_for_positive_spread_but_not_negative() {
    let base = hard(0.0);
    assert_eq!(
        Shadow {
            spread: 0.0,
            ..base
        }
        .reach(),
        0.0,
        "a zero shadow reaches nothing"
    );
    assert_eq!(
        Shadow {
            blur: 5.0,
            spread: 3.0,
            ..base
        }
        .reach(),
        8.0,
        "reach must cover blur + dilation or a damage region would clip the shadow"
    );
    assert_eq!(
        Shadow {
            blur: 5.0,
            spread: -3.0,
            ..base
        }
        .reach(),
        5.0,
        "negative spread pulls the silhouette inward; it cannot extend reach"
    );
    assert_eq!(
        Shadow {
            blur: 0.0,
            offset_y: 4.0,
            spread: 2.0,
            ..base
        }
        .reach(),
        6.0,
        "reach must cover the offset as well as the dilation"
    );
}
