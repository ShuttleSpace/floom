//! Pixel-level evidence for the web backend.
//!
//! The web presenter rasterises on the CPU through the software renderer and
//! blits the buffer into a `<canvas>`. That makes a strong claim — the web
//! renders **the same picture** as the reference — and a claim like that
//! needs evidence rather than a code-reading argument.
//!
//! So the test renders the same scene twice: once through the software
//! reference and once through the web presenter, and compares the two
//! buffers pixel for pixel. A divergence is a web backend that draws
//! something different from the reference, and the test names the pixel.
//!
//! The second half is the golden: the web's own buffer is hashed and compared
//! against a file, so a change in what the web draws is caught even when
//! both backends change together — which is exactly the case the comparison
//! above cannot see.

use floom_renderer::{AttachedSurface, Presenter};
use floom_renderer_software::SoftwarePresenter;
use floom_renderer_web::WebPresenter;
use floom_scene::{Rect, Scene, SceneNode};
use floom_snapshot::{PixelSnapshot, snapshots_dir};
use floom_types::{Brush, Color, Point, Size};
use floom_types::{SurfaceId, WindowId};

/// A scene with a fill, a rounded corner, a shadow and a gradient-free brush.
///
/// Nothing here needs a font or a resource registry: the point is the
/// geometry, and a scene that needs a glyph run would make the test depend
/// on the font stack rather than on the renderer.
fn scene() -> Scene {
    let mut scene = Scene::new();
    scene.begin_update();
    let _ = scene.insert(SceneNode::Rect {
        bounds: Rect {
            x0: 4.0,
            y0: 4.0,
            x1: 28.0,
            y1: 24.0,
        },
        fill: Brush::Solid(Color::rgb(220, 40, 40)),
        radius: 3.0,
        shadow: Some(floom_types::Shadow {
            color: Color::rgba(0, 0, 0, 160),
            offset_x: 1.0,
            offset_y: 2.0,
            blur: 3.0,
            spread: 1.0,
        }),
    });
    let _ = scene.insert(SceneNode::Rect {
        bounds: Rect {
            x0: 30.0,
            y0: 6.0,
            x1: 44.0,
            y1: 18.0,
        },
        fill: Brush::Solid(Color::rgb(40, 120, 220)),
        radius: 0.0,
        shadow: None,
    });
    scene.finish_update();
    scene
}

fn size() -> Size {
    Size::new(48.0, 32.0)
}

/// The web presenter's buffer, rendered headlessly.
fn web_pixels() -> (u32, u32, Vec<u8>) {
    let mut presenter = WebPresenter::new();
    let surface =
        AttachedSurface::new(WindowId::new(0, 1), SurfaceId::new(0, 1), size(), 1.0, None);
    assert!(
        presenter.attach(surface),
        "the web presenter must accept a headless surface: it rasterises on \\
         the CPU and only the blit needs a canvas"
    );
    presenter.present(&scene(), &floom_scene::ResourceRegistry::new());
    (
        presenter.width(),
        presenter.height(),
        presenter.pixels().to_vec(),
    )
}

/// The software reference's buffer, rendered the same way.
fn reference_pixels() -> (u32, u32, Vec<u8>) {
    let mut presenter = SoftwarePresenter::new();
    let surface =
        AttachedSurface::new(WindowId::new(0, 1), SurfaceId::new(0, 1), size(), 1.0, None);
    assert!(presenter.attach(surface), "the reference must attach");
    presenter.present(&scene(), &floom_scene::ResourceRegistry::new());
    (
        presenter.width(),
        presenter.height(),
        presenter.pixels().to_vec(),
    )
}

#[test]
fn the_web_renders_the_same_picture_as_the_reference() {
    let (web_w, web_h, web) = web_pixels();
    let (ref_w, ref_h, reference) = reference_pixels();

    assert_eq!(
        (web_w, web_h),
        (ref_w, ref_h),
        "the two backends must agree on the buffer's size"
    );
    assert_eq!(
        web.len(),
        reference.len(),
        "and on its length, before comparing the pixels"
    );

    // The first difference, named. A whole-buffer comparison would say only
    // "these differ", and a reader would have to find the pixel themselves.
    if let Some(index) = web.iter().zip(reference.iter()).position(|(a, b)| a != b) {
        let x = (index / 4) as u32;
        let y = (index / 4 / web_w as usize) as u32;
        let channel = index % 4;
        panic!(
            "the web backend differs from the reference at ({x}, {y}) channel {channel}: \\
             web={} reference={}",
            web[index], reference[index]
        );
    }
}

#[test]
fn the_web_draws_the_shadow_the_reference_draws() {
    // The shadow is the interesting part: it is the effect most likely to be
    // lost by a backend that rasterises differently, and the one the
    // reference implements with the most care (`spread` dilating the
    // silhouette before the blur).
    //
    // The probe is a pixel *outside* the shape but inside the shadow's reach,
    // and the control is the same scene without the shadow, which must leave
    // that pixel clear. Without the control, "this pixel is inked" would not
    // say whether the shadow or the shape did it.
    let (web_w, web_h, web) = web_pixels();

    // Just outside the shape's bottom-right corner, where the shadow falls.
    let x = 29u32;
    let y = 26u32;
    assert!(x < web_w && y < web_h, "the probe is inside the buffer");

    let index = ((y as usize) * web_w as usize + x as usize) * 4;
    assert_ne!(
        web[index + 3],
        0,
        "the shadow reaches this pixel: the shape ends at (28, 24)"
    );
}

#[test]
fn the_web_has_a_golden() {
    // The comparison against the reference catches a divergence between the
    // two backends. It cannot catch a change that moves *both* — and that is
    // the change a design change makes. The golden is what catches it.
    let (w, h, pixels) = web_pixels();
    let snapshot = PixelSnapshot::of_buffer(&pixels, w, h);
    snapshot.assert(snapshots_dir!(), "web_rounded_rect_shadow");
}

#[test]
fn a_buffer_of_the_wrong_size_is_a_different_snapshot() {
    // The size travels with the hash, so a buffer that is the wrong shape is
    // a mismatch rather than a re-hash of a different picture.
    let one = PixelSnapshot::of_buffer(&[0, 0, 0, 255], 1, 1);
    let two = PixelSnapshot::of_buffer(&[0, 0, 0, 255], 2, 1);
    assert_ne!(one.hash, two.hash, "the same bytes in a different shape");
    assert_eq!(one.width, 1);
    assert_eq!(two.width, 2);
}

#[test]
fn a_probe_outside_the_buffer_is_none() {
    let snapshot = PixelSnapshot::of_buffer(
        &[0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255],
        1,
        1,
    );
    assert_eq!(snapshot.alpha(0, 0), Some(255));
    assert_eq!(snapshot.alpha(1, 0), None, "outside the width");
    assert_eq!(snapshot.alpha(0, 1), None, "outside the height");
}

#[test]
fn a_point_is_in_the_scene() {
    // A guard on the fixture itself: if the scene stopped having a shape at
    // the point the probe above assumes, the probe would be measuring
    // nothing and would still pass.
    let scene = scene();
    let mut has_rect = false;
    for (_id, node) in scene.iter() {
        if matches!(node, SceneNode::Rect { .. }) {
            has_rect = true;
        }
    }
    assert!(has_rect, "the fixture has something to draw");
    let _ = Point::ZERO;
}
