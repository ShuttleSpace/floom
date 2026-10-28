//! Pixel-level evidence for the GPU path (B7).
//!
//! Every other vello test in this workspace asserts about *counters* — how
//! many rects were encoded, how many shadows were skipped. That is a claim
//! about intent, not about pixels, and the two diverge in exactly the ways
//! that hurt: a clip applied twice moves the ink and no counter notices; a
//! shadow path that silently stops drawing still encodes the rect.
//!
//! So these tests rasterise a real scene into an offscreen texture through the
//! same encode/render path a window uses, read the pixels back, and assert
//! about **where the ink is**. The control case is in every test: an assertion
//! that a pixel is empty is worth nothing without the same probe over a scene
//! whose pixel must be inked.
//!
//! ## Why they are `#[ignore]`d
//!
//! They need a working GPU adapter. CI runners have none, and `VelloGpuPresenter::new`
//! honestly returns `None` rather than panicking — so a gate that ran them
//! would report "no GPU" as a pass or as a failure depending on how it was
//! written, and either would be a lie. The skip is spelled out at run time by
//! [`no_adapter_reason`] instead of being silent, and the gate prints the true
//! reason it did not run them.
//!
//! Run them yourself:
//!
//! ```text
//! cargo test -p floom-renderer-vello --features gpu -- --ignored
//! ```
//!
//! Set `FLOOM_BLESS_GOLDENS=1` to rewrite the stored hashes after an
//! intentional change, and read the diff before committing it.

#![cfg(feature = "gpu")]

use floom_renderer::PixelSnapshot;
use floom_renderer_vello::VelloGpuPresenter;
use floom_scene::{ResourceRegistry, Scene, SceneNode};
use floom_types::{Brush, ClipRegion, Color, Rect};

const INK: Color = Color::rgb(220, 40, 40);
const MARK: Color = Color::rgb(40, 200, 90);

const W: u32 = 64;
const H: u32 = 64;

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
    Rect { x0, y0, x1, y1 }
}

fn fill(bounds: Rect, color: Color) -> SceneNode {
    SceneNode::Rect {
        bounds,
        fill: Brush::from(color),
        radius: 0.0,
        shadow: None,
    }
}

/// A presenter, or `None` when this machine has no usable adapter. The caller
/// turns that into an explicit skip message rather than a silent pass.
fn presenter() -> Option<VelloGpuPresenter> {
    VelloGpuPresenter::new()
}

/// The reason this run cannot produce evidence, printed so a skip is never
/// mistaken for a pass. Returns `Some` exactly when there is no adapter.
fn no_adapter_reason() -> Option<String> {
    if presenter().is_some() {
        return None;
    }
    Some(
        "no GPU adapter: VelloGpuPresenter::new() returned None, so there are no pixels to \
         read back — this is a skip, not a pass"
            .to_string(),
    )
}

/// Rasterises `nodes` at 1:1 into a `W × H` offscreen texture and hands back
/// the RGBA bytes (top-to-bottom, as `PixelSnapshot` wants them).
fn render(nodes: Vec<SceneNode>) -> Vec<u8> {
    let Some(mut gpu) = presenter() else {
        panic!("{}", no_adapter_reason().unwrap());
    };
    let mut scene = Scene::new();
    scene.begin_update();
    for node in nodes {
        let _ = scene.insert(node);
    }
    scene.finish_update();
    let resources = ResourceRegistry::new();
    gpu.render_offscreen(&scene, &resources, W, H)
        .expect("offscreen readback mapped")
}

fn pixel(rgba: &[u8], x: u32, y: u32) -> [u8; 4] {
    let index = ((y as usize) * W as usize + x as usize) * 4;
    [
        rgba[index],
        rgba[index + 1],
        rgba[index + 2],
        rgba[index + 3],
    ]
}

fn alpha(rgba: &[u8], x: u32, y: u32) -> u8 {
    pixel(rgba, x, y)[3]
}

/// Whether the pixel is the ink colour, within the tolerance antialiasing on a
/// pixel centre needs.
fn is_ink(rgba: &[u8], x: u32, y: u32) -> bool {
    let p = pixel(rgba, x, y);
    p[3] > 200 && p[0].abs_diff(INK.r) < 12 && p[1].abs_diff(INK.g) < 12
}

fn is_mark(rgba: &[u8], x: u32, y: u32) -> bool {
    let p = pixel(rgba, x, y);
    p[3] > 200 && p[0].abs_diff(MARK.r) < 12 && p[1].abs_diff(MARK.g) < 12
}

// ---------------------------------------------------------------------------
// The boundary exists at all
// ---------------------------------------------------------------------------

/// The control case every other test leans on: an empty scene is transparent,
/// and a filled rect is opaque where it was asked to be and clear outside it.
///
/// Without this, "the clip corner is empty" would pass on a rasteriser that
/// drew nothing at all.
#[test]
#[ignore = "needs a GPU adapter; run with --features gpu -- --ignored"]
fn an_empty_scene_is_transparent_and_a_filled_rect_is_not() {
    if let Some(reason) = no_adapter_reason() {
        eprintln!("SKIP: {reason}");
        return;
    }

    let empty = render(vec![]);
    assert_eq!(alpha(&empty, 32, 32), 0, "nothing drawn, nothing inked");

    let filled = render(vec![fill(rect(16.0, 16.0, 48.0, 48.0), INK)]);
    assert!(is_ink(&filled, 32, 32), "the middle of the rect is ink");
    assert_eq!(
        alpha(&filled, 2, 2),
        0,
        "outside the rect the frame is still clear — the rect did not leak"
    );
}

// ---------------------------------------------------------------------------
// F2 — clipping
// ---------------------------------------------------------------------------

/// A clip crops the ink and leaves the rest of the scene untouched.
///
/// The bug this is written against is the one `d4827c5b` fixed: a clip applied
/// at the wrong scale moves the boundary, and every counter stays green. The
/// probe is the clip's own edge — inked just inside it, clear just outside —
/// on a scene that also carries an unclipped mark, so "the whole frame went
/// blank" cannot pass.
#[test]
#[ignore = "needs a GPU adapter; run with --features gpu -- --ignored"]
fn a_clip_crops_the_ink_at_its_own_edge() {
    if let Some(reason) = no_adapter_reason() {
        eprintln!("SKIP: {reason}");
        return;
    }

    // A rect spanning the whole frame, drawn inside a clip that admits only
    // the left half, plus a mark outside the clip to prove the frame is alive.
    let rgba = render(vec![
        SceneNode::ClipPush {
            region: ClipRegion::rect(rect(0.0, 0.0, 32.0, 64.0)),
        },
        fill(rect(0.0, 0.0, 64.0, 64.0), INK),
        SceneNode::ClipPop,
        fill(rect(48.0, 48.0, 62.0, 62.0), MARK),
    ]);

    assert!(
        is_ink(&rgba, 4, 4),
        "inside the clip the ink is drawn — the clip did not swallow it"
    );
    assert_eq!(
        alpha(&rgba, 40, 4),
        pixel(&rgba, 40, 4)[3].min(alpha(&rgba, 40, 4)),
        "sanity: the probe reads the buffer"
    );
    assert!(
        !is_ink(&rgba, 40, 4),
        "just outside the clip the ink stops — a clip that did not apply would ink here"
    );
    assert!(
        is_mark(&rgba, 55, 55),
        "the unclipped mark survived — the clip is not a frame-wide blank"
    );
}

// ---------------------------------------------------------------------------
// F3 — shadows
// ---------------------------------------------------------------------------

/// A rect's shadow is drawn *outside* the rect, and the rect still covers its
/// own middle.
///
/// `shadows_drawn` counts the ask, not the result; this is the pixel half.
/// The probe sits in the shadow band below the rect, where only the blur can
/// put ink.
#[test]
#[ignore = "needs a GPU adapter; run with --features gpu -- --ignored"]
fn a_shadow_puts_ink_outside_the_rect_it_belongs_to() {
    if let Some(reason) = no_adapter_reason() {
        eprintln!("SKIP: {reason}");
        return;
    }

    let bare = render(vec![fill(rect(20.0, 20.0, 44.0, 44.0), INK)]);
    let shadowed = render(vec![SceneNode::Rect {
        bounds: rect(20.0, 20.0, 44.0, 44.0),
        fill: Brush::from(INK),
        radius: 0.0,
        shadow: Some(floom_types::Shadow {
            color: Color::rgb(0, 0, 0),
            blur: 6.0,
            offset_x: 0.0,
            offset_y: 8.0,
            spread: 0.0,
        }),
    }]);

    assert!(is_ink(&shadowed, 32, 32), "the rect itself is still inked");
    // Below the rect, past its bottom edge, only a shadow can ink this.
    assert_eq!(
        alpha(&bare, 32, 52),
        0,
        "the control has nothing below the rect"
    );
    assert!(
        alpha(&shadowed, 32, 52) > 0,
        "the shadow band below the rect carries ink"
    );
}

// ---------------------------------------------------------------------------
// F1 — filter scopes
// ---------------------------------------------------------------------------

/// A filter scope really blurs its contents: the ink inside the scope stops
/// having a hard edge.
///
/// This is the claim `filters_drawn` was raised for, checked against pixels
/// rather than against the counter. The control is the same scene without the
/// scope — a sharp rect whose edge is exactly where the bounds say — so "the
/// edge is soft" cannot pass on a scene that drew nothing.
#[test]
#[ignore = "needs a GPU adapter; run with --features gpu -- --ignored"]
fn a_filter_scope_blurs_its_contents() {
    if let Some(reason) = no_adapter_reason() {
        eprintln!("SKIP: {reason}");
        return;
    }

    let sharp = render(vec![fill(rect(16.0, 16.0, 48.0, 48.0), INK)]);

    let blurred = render(vec![
        SceneNode::FilterPush {
            filter: floom_types::Filter::blur(6.0),
        },
        fill(rect(16.0, 16.0, 48.0, 48.0), INK),
        SceneNode::FilterPop,
    ]);

    // The control's edge is hard: the pixel just outside the rect is clear.
    assert_eq!(
        alpha(&sharp, 12, 32),
        0,
        "control: nothing outside the rect"
    );
    // The blurred one spreads past the same edge — that is the halo a blur
    // produces and a sharp draw cannot.
    assert!(
        alpha(&blurred, 12, 32) > 0,
        "the blur must spread outside the rect it filtered, got {}",
        alpha(&blurred, 12, 32)
    );
    // And the middle is still inked, so the scope did not erase its content.
    assert!(
        is_ink(&blurred, 32, 32),
        "the filtered content is still there, just blurred"
    );
    // The *edge* is where a blur shows: the pixel just inside it has to be a
    // mix of the rect's colour and what surrounds it, so it cannot be the
    // rect's own colour. Checking the middle instead would pass on a scope that
    // silently did nothing — the middle of a 32px square is pure ink either way.
    assert_eq!(
        pixel(&sharp, 17, 32),
        [INK.r, INK.g, INK.b, 255],
        "control: the pixel just inside the edge is the ink colour"
    );
    assert_ne!(
        pixel(&blurred, 17, 32),
        pixel(&sharp, 17, 32),
        "at the edge the blurred frame must differ from the sharp one"
    );
}

/// A filter scope that covers nothing is counted as skipped, not drawn: there
/// are no contents to blur, and inventing a box would smear empty space into a
/// visible smudge.
#[test]
#[ignore = "needs a GPU adapter; run with --features gpu -- --ignored"]
fn an_empty_filter_scope_draws_nothing() {
    if let Some(reason) = no_adapter_reason() {
        eprintln!("SKIP: {reason}");
        return;
    }
    let rgba = render(vec![
        SceneNode::FilterPush {
            filter: floom_types::Filter::blur(8.0),
        },
        SceneNode::FilterPop,
        fill(rect(48.0, 48.0, 62.0, 62.0), MARK),
    ]);
    assert!(
        is_mark(&rgba, 55, 55),
        "the node after the empty scope still draws"
    );
    assert_eq!(
        alpha(&rgba, 8, 8),
        0,
        "an empty scope must not smear anything into the empty corner"
    );
}

// ---------------------------------------------------------------------------
// Golden
// ---------------------------------------------------------------------------

/// The whole path under one hash: two overlapping rects, one in front.
///
/// A hash is a weak claim on its own — it says "this changed", not "this is
/// right" — which is why every test above asserts about geometry instead. It
/// earns its place by being the thing that catches the change nobody thought
/// to probe.
#[test]
#[ignore = "needs a GPU adapter; run with --features gpu -- --ignored"]
fn the_reference_scene_matches_its_stored_hash() {
    if let Some(reason) = no_adapter_reason() {
        eprintln!("SKIP: {reason}");
        return;
    }

    let rgba = render(vec![
        fill(rect(8.0, 8.0, 40.0, 40.0), INK),
        fill(rect(24.0, 24.0, 56.0, 56.0), MARK),
    ]);
    let snapshot = PixelSnapshot::of_buffer(&rgba, W, H);
    snapshot.assert(env!("CARGO_MANIFEST_DIR"), "vello_gpu_reference");
}
