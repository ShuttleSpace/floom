//! Reconciliation across the layout composites.
//!
//! These run the real `WidgetHost` with the real `floom-widgets` composites and
//! assert what the reconciler mounts/unmounts. Because removed subtrees are only
//! flagged `dead` (their `NodeId` slot is kept for stability), we count *live*
//! widgets via [`FrameStats::painted`], which the host increments once per
//! non-dead node.
//!
//! They drive that through `floom_testkit::TestApp`, with the root left
//! *unpinned* -- these tests are about which subtree exists, and pinning the root
//! to the viewport would decide the layout for them.
//!
//! The surface composites -- dialog, popup, bottom sheet, tabs -- used to be here
//! too, until they moved up a layer into `floom-ui-composites`. Their tests went
//! with them; see that crate's `tests/reconcile_surfaces.rs`.
//!
//! [`FrameStats::painted`]: floom_app::FrameStats::painted

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use floom_core::{Axis, Child, ChildKey, LayoutRequest, Point, Size};
use floom_testkit::TestApp;
use floom_widgets::{Button, Conditional, Grid, List, Stack, Text};

/// The viewport every test mounts into.
const VIEWPORT: Size = Size::new(240.0, 320.0);

/// Stable FNV-1a key for the string items used by the list/grid tests.
fn key_of(s: &String) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for byte in s.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// A column holding one child of the given size.
fn sized_column(
    width: f32,
    height: f32,
    build: impl Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static,
) -> impl floom_core::WidgetAny + 'static {
    Stack::new(Axis::Vertical).child(Child::new(
        ChildKey::index(0),
        LayoutRequest::new().width(width).height(height),
        build,
    ))
}

#[test]
fn stack_mounts_every_child_once() {
    let mut stack = Stack::new(Axis::Vertical).gap(4.0);
    for i in 0..3 {
        stack = stack.push(move || Box::new(Text::new(format!("row {i}"))));
    }
    let mut app = TestApp::unpinned(VIEWPORT, stack);
    let stats = app.frame();
    assert_eq!(stats.painted, 4, "1 stack + 3 texts");
}

#[test]
fn list_mounts_unmounts_on_diff_keeping_state_slots() {
    let list = List::<String>::new(
        Axis::Vertical,
        vec!["a".into(), "b".into(), "c".into()],
        |s: &String| {
            let mut h = 0u64;
            for b in s.bytes() {
                h = h.wrapping_mul(31).wrapping_add(b as u64);
            }
            h
        },
        |s: String| Box::new(Text::new(s)),
    );
    let mut app = TestApp::unpinned(VIEWPORT, list);
    let list_id = app.root();
    let stats = app.frame();
    assert_eq!(stats.painted, 4, "1 list + 3 rows");

    // Insert one row in the middle: only the new row should mount.
    app.widget_mut::<List<String>>(list_id).set_items(vec![
        "a".into(),
        "b".into(),
        "x".into(),
        "c".into(),
    ]);
    let stats = app.frame();
    assert_eq!(stats.painted, 5, "1 list + 4 rows");

    // Remove two rows: the surviving slots stay mounted, the rest unmount.
    app.widget_mut::<List<String>>(list_id)
        .set_items(vec!["a".into(), "c".into()]);
    let stats = app.frame();
    assert_eq!(stats.painted, 3, "1 list + 2 rows");
}

#[test]
fn conditional_toggles_its_subtree() {
    let cond =
        Conditional::new(false).show(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
            Box::new(Text::new("hi"))
        }));
    let mut app = TestApp::unpinned(VIEWPORT, cond);
    let id = app.root();
    let stats = app.frame();
    assert_eq!(stats.painted, 1, "hidden conditional is alone");

    app.widget_mut::<Conditional>(id).set_when(true);
    let stats = app.frame();
    assert_eq!(stats.painted, 2, "conditional + its child mounted");

    app.widget_mut::<Conditional>(id).set_when(false);
    let stats = app.frame();
    assert_eq!(stats.painted, 1, "subtree unmounted again");
}

#[test]
fn grid_mounts_rows_and_reconciles_on_diff() {
    let grid = Grid::<String>::new(
        2,
        vec!["a".into(), "b".into(), "c".into()],
        key_of,
        |s: String| Box::new(Text::new(s)),
    );
    let mut app = TestApp::unpinned(VIEWPORT, grid);
    let id = app.root();
    // grid + row(a,b) + row(c) + 3 cells
    assert_eq!(app.frame().painted, 6, "two rows over three cells");

    // Adding one cell grows only the affected row: 2 rows, 4 cells.
    app.widget_mut::<Grid<String>>(id).set_items(vec![
        "a".into(),
        "b".into(),
        "c".into(),
        "d".into(),
    ]);
    assert_eq!(app.frame().painted, 7, "second row gained a cell");

    // Shrinking to a single row unmounts the other row's whole subtree.
    app.widget_mut::<Grid<String>>(id)
        .set_items(vec!["a".into(), "b".into()]);
    assert_eq!(app.frame().painted, 4, "one row + two cells + grid");
}
