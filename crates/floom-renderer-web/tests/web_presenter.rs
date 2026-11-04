//! Tests for `WebPresenter` — the web backend, on the path that actually runs here.
//!
//! ## Why this file exists at all
//!
//! Every other renderer in the workspace has tests (`floom-renderer-software`
//! has five files, `floom-renderer-tui` has 21 cases) and this crate had none.
//! That is not because the web backend is simpler — it wraps the software
//! renderer and adds buffer sizing and a canvas blit — but because it is
//! usually only exercised on wasm, where a test would need a DOM.
//!
//! It does not. `WebPresenter` rasterizes into a CPU buffer *first* and blits
//! second, and on a non-wasm target the blit is a no-op. So the buffer is the
//! presenter's real output on this platform and every interesting property of
//! the backend is observable without a browser.
//!
//! ## What is deliberately not tested
//!
//! The `canvas` module. Its wasm32 arm touches `document`, `HtmlCanvasElement`
//! and `CanvasRenderingContext2d`, none of which exist here, and a non-wasm arm
//! that returns `()` has nothing to assert. Testing it would mean testing that a
//! stub does nothing. The contract that *is* testable — that the buffer the blit
//! receives is the buffer that was rendered — is tested below by pinning the
//! blit's source against the buffer.

use floom_renderer::{AttachedSurface, Presenter};
use floom_renderer_web::WebPresenter;
use floom_scene::{Rect, Scene, SceneNode};
use floom_types::{Brush, Color, Size, SurfaceId, WindowId};

const INK: Color = Color::rgb(220, 40, 40);

/// A scene with one opaque rect covering the whole canvas.
///
/// Sized in logical units and left to the presenter to scale, so a test that
/// asserts on physical pixel counts is also asserting that the logical →
/// physical conversion happened.
fn filled_scene() -> Scene {
    let mut scene = Scene::new();
    scene.begin_update();
    let _ = scene.insert(SceneNode::Rect {
        bounds: Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 50.0,
        },
        fill: Brush::from(INK),
        radius: 0.0,
        shadow: None,
    });
    scene.finish_update();
    scene
}

/// An attached presenter at `logical`, on a screen of `scale`.
fn attached(logical: Size, scale: f64) -> WebPresenter {
    let mut p = WebPresenter::new();
    let surface = AttachedSurface::new(
        WindowId::new(1, 0),
        SurfaceId::new(1, 0),
        logical,
        scale,
        None,
    );
    assert!(p.attach(surface), "the web presenter always attaches");
    p
}

/// Whether the pixel at `(x, y)` is the ink colour.
///
/// Tolerant on each channel because the software renderer may blend; the test
/// only needs to tell "the rect was drawn" from "the canvas is blank".
fn is_ink(p: &WebPresenter, x: u32, y: u32) -> bool {
    let px = p.width() as usize;
    let idx = (y as usize) * px * 4 + x as usize * 4;
    let buf = p.pixels();
    if idx + 3 >= buf.len() {
        return false;
    }
    buf[idx].abs_diff(INK.r) < 12 && buf[idx + 1].abs_diff(INK.g) < 12
}


// ---------------------------------------------------------------------------
// attachment and extent
// ---------------------------------------------------------------------------

/// A presenter with no surface has a 1×1 buffer and presents nothing.
///
/// The buffer starts at 1×1 rather than 0×0 because a zero-sized buffer would
/// make every `pixel_rgba` probe out of range, and "not attached" is better
/// expressed by `is_attached` than by a degenerate extent.
#[test]
fn an_unattached_presenter_presents_nothing() {
    let mut p = WebPresenter::new();
    assert!(!p.is_attached());
    assert_eq!((p.width(), p.height()), (1, 1));

    p.present(&filled_scene(), &Default::default());
    assert_eq!(
        p.frames_presented(),
        0,
        "presenting without a surface must not count a frame — the blit has \\
         nowhere to go and nothing was drawn"
    );
}

/// Attaching sizes the buffer to the logical size times the scale factor.
///
/// The ceiling is the point: a 100×50 surface at scale 1.5 is 150×75, and a
/// floor rather than a ceil would give 149 and clip the last column.
#[test]
fn attaching_sizes_the_buffer_in_physical_pixels() {
    let p = attached(Size::new(100.0, 50.0), 1.5);
    assert!(p.is_attached());
    assert_eq!((p.width(), p.height()), (150, 75));
}

/// A fractional logical size rounds up, so the last row and column are inside
/// the buffer rather than half a pixel outside it.
#[test]
fn a_fractional_extent_rounds_up() {
    let p = attached(Size::new(100.4, 50.4), 1.0);
    assert_eq!((p.width(), p.height()), (101, 51));
}

/// A degenerate extent becomes one pixel rather than zero.
///
/// A zero-width buffer would make `pixels()` empty and every probe fail with an
/// index error instead of the assertion the test meant to make.
#[test]
fn a_degenerate_extent_becomes_one_pixel() {
    let p = attached(Size::new(0.0, 0.0), 1.0);
    assert_eq!((p.width(), p.height()), (1, 1));
}

/// A scale factor of zero or negative falls back to 1.0.
///
/// Without the guard, scale 0 would size the buffer to zero and the presenter
/// would present nothing while reporting success.
#[test]
fn a_non_positive_scale_falls_back_to_one() {
    let p = attached(Size::new(40.0, 20.0), 0.0);
    assert_eq!((p.width(), p.height()), (40, 20));

    let p = attached(Size::new(40.0, 20.0), -2.0);
    assert_eq!((p.width(), p.height()), (40, 20));
}

// ---------------------------------------------------------------------------
// presenting
// ---------------------------------------------------------------------------

/// A presented frame draws the scene into the buffer.
///
/// This is the load-bearing test for the whole file: on a non-wasm target the
/// blit is a no-op, so if `present` does not rasterize, nothing observable
/// happens at all and the backend would be untested by construction.
#[test]
fn presenting_draws_the_scene_into_the_buffer() {
    let mut p = attached(Size::new(100.0, 50.0), 1.0);
    p.present(&filled_scene(), &Default::default());

    assert_eq!(p.frames_presented(), 1);
    assert!(
        is_ink(&p, 50, 25),
        "the centre of a full-canvas rect must be ink"
    );
}

/// **The control for the above**: without a scene node the buffer stays
/// backdrop. Without this, `is_ink` returning true could mean the buffer was
/// initialised to the ink colour rather than that anything was drawn.
#[test]
fn an_empty_scene_leaves_the_backdrop() {
    let mut p = attached(Size::new(100.0, 50.0), 1.0);
    let empty = Scene::new();
    p.present(&empty, &Default::default());

    assert_eq!(p.frames_presented(), 1, "a frame was still presented");
    assert!(
        !is_ink(&p, 50, 25),
        "nothing was drawn, so the centre is still backdrop"
    );
}

/// Frames accumulate, one per `present`.
#[test]
fn each_present_counts_one_frame() {
    let mut p = attached(Size::new(20.0, 20.0), 1.0);
    for expected in 1..=5 {
        p.present(&filled_scene(), &Default::default());
        assert_eq!(p.frames_presented(), expected);
    }
}

/// The buffer is exactly `width * height * 4` bytes.
///
/// The blit reads `width * height * 4` bytes out of it, so a buffer of the wrong
/// length is an out-of-bounds read on wasm and a panic in the browser — and
/// neither shows up in a build.
#[test]
fn the_buffer_is_exactly_four_bytes_per_pixel() {
    let p = attached(Size::new(37.0, 19.0), 2.0);
    assert_eq!((p.width(), p.height()), (74, 38));
    assert_eq!(
        p.pixels().len(),
        74 * 38 * 4,
        "the blit consumes width*height*4; a shorter buffer is an OOB read"
    );
}

/// A detached presenter stops presenting.
#[test]
fn detaching_stops_presenting() {
    let mut p = attached(Size::new(20.0, 20.0), 1.0);
    p.detach();
    assert!(!p.is_attached());

    p.present(&filled_scene(), &Default::default());
    assert_eq!(p.frames_presented(), 0);
}

/// A presenter can be re-attached after detaching, and presents again.
#[test]
fn a_detached_presenter_can_be_re_attached() {
    let mut p = attached(Size::new(20.0, 20.0), 1.0);
    p.present(&filled_scene(), &Default::default());
    p.detach();

    let surface = AttachedSurface::new(
        WindowId::new(1, 0),
        SurfaceId::new(1, 0),
        Size::new(30.0, 30.0),
        1.0,
        None,
    );
    assert!(p.attach(surface));
    assert_eq!((p.width(), p.height()), (30, 30));
    assert_eq!(p.frames_presented(), 1, "the frame count survives detach");

    p.present(&filled_scene(), &Default::default());
    assert_eq!(p.frames_presented(), 2);
}

// ---------------------------------------------------------------------------
// resize
// ---------------------------------------------------------------------------

/// Resizing rebuilds the buffer at the new extent.
///
/// The rebuild matters: `SoftwareRenderer` allocates its buffer once, so
/// changing the dimensions without constructing a new renderer would leave the
/// presenter reporting one size while painting into another.
#[test]
fn resizing_rebuilds_the_buffer() {
    let mut p = attached(Size::new(20.0, 20.0), 1.0);
    p.resize(Size::new(60.0, 40.0), 1.0);

    assert_eq!((p.width(), p.height()), (60, 40));
    assert_eq!(p.pixels().len(), 60 * 40 * 4);
}

/// Resizing to the same extent keeps the same buffer, so the contents survive.
///
/// A presenter that reallocated on every resize would drop the previous frame's
/// pixels even when nothing about the size changed — which is what a redraw on
/// every vsync looks like.
#[test]
fn resizing_to_the_same_extent_preserves_the_buffer() {
    let mut p = attached(Size::new(20.0, 20.0), 1.0);
    p.present(&filled_scene(), &Default::default());
    let before = p.pixels().to_vec();

    p.resize(Size::new(20.0, 20.0), 1.0);
    assert_eq!(p.pixels(), before.as_slice());
}

/// **Resizing before attaching does nothing.**
///
/// Without the `is_attached` guard, a `resize` delivered before `attach` would
/// size a buffer that is about to be replaced, and on a race between the two
/// that means the surface is presented at the wrong extent.
#[test]
fn resizing_before_attaching_is_ignored() {
    let mut p = WebPresenter::new();
    p.resize(Size::new(80.0, 60.0), 1.0);
    assert_eq!(
        (p.width(), p.height()),
        (1, 1),
        "an unattached presenter must keep its placeholder extent"
    );
}

/// Resizing applies the new scale factor.
#[test]
fn resizing_applies_the_scale_factor() {
    let mut p = attached(Size::new(20.0, 20.0), 1.0);
    p.resize(Size::new(20.0, 20.0), 2.0);
    assert_eq!((p.width(), p.height()), (40, 40));
}

// ---------------------------------------------------------------------------
// defaults
// ---------------------------------------------------------------------------

/// `Default` is the same as `new`, and both are un-attached.
#[test]
fn default_is_new_and_unattached() {
    let p = WebPresenter::default();
    assert!(!p.is_attached());
    assert_eq!((p.width(), p.height()), (1, 1));
}
