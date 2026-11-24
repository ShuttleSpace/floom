// SPDX-License-Identifier: MIT
//! A pixel golden that fails with a picture instead of a hash.

use floom_snapshot::{PixelSnapshot, ReferencePixels, assert_pixels};

/// `UPDATE_SNAPSHOTS` is process-wide, so the two tests that set it take turns
/// rather than racing: the harness runs them on threads of one process.
static ENVIRONMENT: std::sync::Mutex<()> = std::sync::Mutex::new(());
use floom_types::{Color, Point, Rect, Size};

fn scene(fill: Color) -> floom_scene::Scene {
    let size = Size::new(4.0, 2.0);
    let mut scene = floom_scene::Scene::new();
    scene.begin_update();
    let _ = scene.insert(floom_scene::SceneNode::Rect {
        bounds: Rect::from_origin_size(Point::new(0.0, 0.0), size),
        fill: floom_types::Brush::Solid(fill),
        radius: 0.0,
        shadow: None,
    });
    scene.finish_update();
    scene
}

fn shoot(fill: Color) -> PixelSnapshot {
    PixelSnapshot::of(&scene(fill), None, Size::new(4.0, 2.0), 1.0)
}

#[test]
fn a_pixel_golden_is_created_blessed_and_compared() {
    let _turn = ENVIRONMENT
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let dir = std::env::temp_dir().join(format!("floom-pixel-golden-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let manifest = dir.to_str().expect("utf-8 path");

    // SAFETY-ish: this test owns the variable for the duration of the process'
    // run of this binary, and no other test in this file reads it.
    unsafe { std::env::set_var("UPDATE_SNAPSHOTS", "1") };
    assert_pixels(&shoot(Color::rgb(10, 20, 30)), manifest, "probe");
    unsafe { std::env::remove_var("UPDATE_SNAPSHOTS") };

    let golden = dir.join("snapshots").join("probe.png");
    assert!(golden.exists(), "the golden was written: {golden:?}");

    // The same picture passes and leaves nothing behind.
    assert_pixels(&shoot(Color::rgb(10, 20, 30)), manifest, "probe");
    assert!(
        !dir.join("snapshots").join("probe-actual.png").exists(),
        "a picture that matches writes nothing"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_picture_that_moved_leaves_the_one_that_failed() {
    let _turn = ENVIRONMENT
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let dir = std::env::temp_dir().join(format!("floom-pixel-moved-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let manifest = dir.to_str().expect("utf-8 path");

    unsafe { std::env::set_var("UPDATE_SNAPSHOTS", "1") };
    assert_pixels(&shoot(Color::rgb(10, 20, 30)), manifest, "moved");
    unsafe { std::env::remove_var("UPDATE_SNAPSHOTS") };

    let failure = std::panic::catch_unwind(|| {
        assert_pixels(&shoot(Color::rgb(200, 0, 0)), manifest, "moved")
    })
    .expect_err("a different picture fails");
    let message = failure
        .downcast_ref::<String>()
        .cloned()
        .expect("the panic carries a message");
    assert!(
        message.contains("moved-actual.png"),
        "the failure names the file to look at: {message}"
    );
    let actual = dir.join("snapshots").join("moved-actual.png");
    assert!(actual.exists(), "the picture that failed was written");
    assert!(
        std::fs::read(&actual)
            .expect("read")
            .starts_with(b"\x89PNG"),
        "and it is a PNG"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
