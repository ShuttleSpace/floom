// SPDX-License-Identifier: MIT
//! Golden integration (Task 10): the TSX-authored tree mounts and renders
//! headlessly through `floom-testkit` (software presenter, no GPU needed).
//!
//! This proves the whole pipeline on a real host — TSX -> UiDocument -> Rust ->
//! mounted widget -> first paint — without opening a window. The plan's
//! click->label-change assertion lives with the reactive-counter features
//! (closure handlers + dynamic labels) and is not yet supported by the lowered
//! TSX idiom, so it is intentionally omitted here.

use floom_testkit::TestApp;
use floom_types::Size;

#[test]
fn showcase_headless_pump() {
    #[allow(unused_imports, clippy::all)]
    mod ui_gen {
        include!(concat!(env!("OUT_DIR"), "/ui_gen.rs"));
    }

    let mut app = TestApp::new_boxed(Size::new(480.0, 320.0), ui_gen::build());

    // The tree mounted and presented at least one frame through the software
    // presenter.
    assert!(
        app.runner().stats().frames > 0,
        "expected a headless frame; the tree did not mount/render"
    );

    // The labels authored in counter.tsx are present in the rendered tree.
    let labels = app.labels();
    assert!(
        labels.iter().any(|l| l.contains("floom-tsx")),
        "TSX-authored label missing from render: {labels:?}"
    );
}
