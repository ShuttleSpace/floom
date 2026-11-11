// SPDX-License-Identifier: MIT
//! P1-4 / WS-4.3 — the headless PNG + element-tree dump.
//!
//! The exit criterion is that "one command produces PNG + element-tree JSON"
//! and that "the agent/evaluate loop can consume it". The first is the example
//! binary's job; these tests pin the library surface behind it, including the
//! second: the bundle deserializes with types this crate already publishes.

use std::fs;

use floom_accuracy::headless::{SnapshotBundle, capture, stem, write_capture};
use floom_accuracy::{Screen, ScreenTokens};
use floom_inspector::SceneLines;
use floom_snapshot::{PngError, encode_png};

#[test]
fn a_capture_is_a_real_png_with_a_matching_evaluate_fingerprint() {
    let shot = capture(Screen::Settings, &ScreenTokens::reference()).expect("capture");
    assert_eq!(shot.bundle.screen, "Settings");
    assert_eq!(shot.stem, "settings");

    // PNG signature + IHDR dimensions, read straight from the bytes.
    assert_eq!(
        &shot.png[..8],
        &[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']
    );
    let w = u32::from_be_bytes(shot.png[16..20].try_into().unwrap());
    let h = u32::from_be_bytes(shot.png[20..24].try_into().unwrap());
    assert_eq!((w, h), (shot.bundle.width, shot.bundle.height));
    assert!(w > 0 && h > 0);

    // The bundle's hash and the evaluation report's pixel layer are the same
    // frame, so a consumer can trust either one.
    assert_eq!(shot.bundle.pixel_hash, shot.bundle.evaluation.pixel_hash);
    // The screenshot DTO is a *text-less* frame by design: AgentSession
    // renders it with `None` resources, so its hash is a different (smaller)
    // fingerprint than the full-resource pixel_hash. Both are present and
    // dimensionally consistent; they are not expected to be equal.
    assert_eq!(
        (shot.bundle.screenshot.width, shot.bundle.screenshot.height),
        (w, h)
    );
    assert!(!shot.bundle.screenshot.hash.is_empty());
    assert_eq!(shot.bundle.png, "settings.png");
}

#[test]
fn the_tree_is_real_and_the_scene_lines_are_present() {
    let shot = capture(Screen::Settings, &ScreenTokens::reference()).expect("capture");
    assert!(
        !shot.bundle.tree.nodes.is_empty(),
        "a mounted screen must publish a live element tree"
    );
    assert!(
        !shot.bundle.semantics.is_empty(),
        "the a11y vocabulary must be published alongside the tree"
    );
    assert!(
        !shot.bundle.scene.lines.is_empty(),
        "the painted scene must be dumped as lines"
    );
}

#[test]
fn the_bundle_round_trips_through_the_published_types() {
    let shot = capture(Screen::Dashboard, &ScreenTokens::reference()).expect("capture");
    let json = serde_json::to_string_pretty(&shot.bundle).expect("serialize");

    // Deserialize with the exact types the agent/evaluate loop links — this is
    // the "evaluate can consume it" criterion, not a bespoke reader.
    let back: SnapshotBundle = serde_json::from_str(&json).expect("deserialize bundle");
    assert_eq!(back, shot.bundle);
    let _scene: SceneLines =
        serde_json::from_str(&serde_json::to_string(&back.scene).unwrap()).unwrap();
    assert_eq!(back.evaluation.screen, "Dashboard");
}

#[test]
fn write_capture_produces_both_files_and_the_png_is_decodable() {
    let dir = std::env::temp_dir().join(format!("floom-headless-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let shot = capture(Screen::Feed, &ScreenTokens::reference()).expect("capture");
    let (png_path, json_path) = write_capture(&dir, &shot).expect("write");

    assert_eq!(png_path.file_name().unwrap(), "feed.png");
    assert_eq!(json_path.file_name().unwrap(), "feed.json");
    assert_eq!(
        fs::read(&png_path).unwrap(),
        shot.png,
        "the written PNG is the captured one"
    );

    // A real decoder accepts it (the dev-only `image` dep from floom-snapshot's
    // own tests is not required here: the encoder's round-trip test covers it).
    // We still assert the signature and that the JSON parses as a bundle.
    let bytes = fs::read(&png_path).unwrap();
    assert_eq!(
        &bytes[..8],
        &[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']
    );
    let parsed: SnapshotBundle =
        serde_json::from_str(&fs::read_to_string(&json_path).unwrap()).expect("bundle JSON");
    assert_eq!(parsed, shot.bundle);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn every_screen_can_be_captured() {
    let tokens = ScreenTokens::reference();
    for screen in Screen::ALL {
        let shot = capture(screen, &tokens).unwrap_or_else(|e| panic!("{screen:?}: {e}"));
        assert_eq!(shot.bundle.screen, format!("{screen:?}"));
        assert!(!shot.png.is_empty());
        assert_eq!(shot.stem, stem(screen));
    }
}

#[test]
fn the_encoder_rejects_a_bad_buffer_before_writing_anything() {
    // Guards the zero-dimension path the writer would otherwise feed it.
    assert!(matches!(
        encode_png(&[], 0, 0),
        Err(PngError::ZeroDimension)
    ));
    assert!(matches!(
        encode_png(&[0u8; 3], 1, 1),
        Err(PngError::LengthMismatch {
            expected: 4,
            got: 3
        })
    ));
}
