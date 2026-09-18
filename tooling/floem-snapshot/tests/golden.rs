//! End-to-end golden-file workflow for the snapshot engine.
//!
//! Run once with `UPDATE_SNAPSHOTS=1` to (re)create `snapshots/*.snap`, then
//! plain `cargo test` holds them. The unit tests in `lib.rs` assert stability
//! in-process; these assert the file round-trip and the mismatch path.

use floem_accessibility::{SemanticsRole, SemanticsTree};
use floem_snapshot::{
    PixelSnapshot, SceneSnapshot, SemanticsSnapshot, assert_snapshot, semantics_node,
    single_rect_scene,
};
use floem_types::{Color, ElementId, Size};

#[test]
fn scene_golden_roundtrip() {
    let scene = single_rect_scene(Size::new(8.0, 8.0), Color::rgb(200, 100, 50));
    let snap = SceneSnapshot::of(&scene);
    snap.assert("golden_scene_single_rect");
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
    SemanticsSnapshot::of(&tree).assert("golden_semantics_two_nodes");
}

#[test]
fn pixel_golden_roundtrip() {
    let scene = single_rect_scene(Size::new(8.0, 8.0), Color::rgb(200, 100, 50));
    PixelSnapshot::of(&scene, None, Size::new(8.0, 8.0), 1.0).assert("golden_pixel_single_rect");
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
        assert_snapshot("does_not_exist_xyz", "content");
    });
    assert!(
        result.is_err(),
        "asserting against a missing golden must panic"
    );
}
