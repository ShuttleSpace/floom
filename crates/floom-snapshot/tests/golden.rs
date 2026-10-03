//! End-to-end golden-file workflow for the snapshot engine.
//!
//! Run once with `UPDATE_SNAPSHOTS=1` to (re)create `snapshots/*.snap`, then
//! plain `cargo test` holds them. The unit tests in `lib.rs` assert stability
//! in-process; these assert the file round-trip and the mismatch path.

use floom_accessibility::{SemanticsRole, SemanticsTree};
use floom_snapshot::{
    PixelSnapshot, SceneSnapshot, SemanticsSnapshot, assert_snapshot, semantics_node,
    single_rect_scene, snapshots_dir,
};
use floom_types::{Color, ElementId, Size};

#[test]
fn scene_golden_roundtrip() {
    let scene = single_rect_scene(Size::new(8.0, 8.0), Color::rgb(200, 100, 50));
    let snap = SceneSnapshot::of(&scene);
    snap.assert(snapshots_dir!(), "golden_scene_single_rect");
}

#[test]
fn semantics_golden_roundtrip() {
    let mut tree = SemanticsTree::new();
    tree.set_node(
        ElementId::new(1, 1),
        semantics_node(SemanticsRole::StaticText, "hello"),
    );
    tree.set_node(
        ElementId::new(2, 1),
        semantics_node(SemanticsRole::Button, "ok"),
    );
    SemanticsSnapshot::of(&tree).assert(snapshots_dir!(), "golden_semantics_two_nodes");
}

#[test]
fn pixel_golden_roundtrip() {
    let scene = single_rect_scene(Size::new(8.0, 8.0), Color::rgb(200, 100, 50));
    PixelSnapshot::of(&scene, None, Size::new(8.0, 8.0), 1.0)
        .assert(snapshots_dir!(), "golden_pixel_single_rect");
}

#[test]
fn goldens_land_in_the_asserting_crates_own_directory() {
    // The defect this pins: `snapshot_path` used to call `env!` inside the
    // engine crate, so every downstream crate's goldens were written into
    // `crates/floom-snapshot/snapshots/` and two crates could collide on a
    // name. The macro must resolve to *this* crate.
    let path = floom_snapshot::snapshot_path(snapshots_dir!(), "example");
    assert_eq!(
        path,
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("snapshots")
            .join("example.snap")
    );
    assert!(
        path.starts_with(env!("CARGO_MANIFEST_DIR")),
        "a golden must live under the crate that asserts it: {}",
        path.display()
    );
}

#[test]
fn missing_golden_is_a_clear_failure() {
    // While regenerating baselines every missing file is *created*, so the
    // failure path cannot be exercised then.
    if std::env::var("UPDATE_SNAPSHOTS")
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        return;
    }
    // A name that does not exist must fail with a message pointing at the env
    // knob, not panic with an opaque path error.
    let result = std::panic::catch_unwind(|| {
        assert_snapshot(snapshots_dir!(), "does_not_exist_xyz", "content");
    });
    assert!(
        result.is_err(),
        "asserting against a missing golden must panic"
    );
}
