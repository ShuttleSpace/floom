//! Tree tests: flattening under expand/collapse, keyboard drill-in/out, and
//! the same virtualization contract the grid has.

use floem_core::{ElementId, Key, KeyPhase, NamedKey, Point, Rect, Widget, WidgetAny};
use floem_types::{KeyEvent, Modifiers, PointerEvent};
use floem_virtual::Overscan;
use floem_widgets::testkit::{
    AllocatingCreateContext, SceneRecorder, dispatch, semantics_snapshot,
};
use floem_widgets_data::data_grid::RowPool;
use floem_widgets_data::model::SelectionMode;
use floem_widgets_data::tree::{Tree, TreeNode, TreeNodeId, VecTree};

const HEADER: f64 = 0.0;
const ROW_H: f64 = 20.0;
const WIDTH: f64 = 300.0;
const TEN_ROWS: f64 = HEADER + 10.0 * ROW_H;
/// Horizontal room a disclosure triangle gets, per depth level.
const INDENT: f64 = 16.0;

fn rect(height: f64) -> Rect {
    Rect {
        x0: 0.0,
        y0: 0.0,
        x1: WIDTH,
        y1: height,
    }
}

/// src/
///   main.rs
///   lib.rs
///     inner.rs
/// README.md
fn model() -> VecTree {
    VecTree::new(vec![
        TreeNode::new(TreeNodeId(1), "src")
            .child(
                TreeNode::new(TreeNodeId(2), "main.rs")
                    .child(TreeNode::new(TreeNodeId(4), "deep.rs")),
            )
            .child(TreeNode::new(TreeNodeId(3), "lib.rs")),
        TreeNode::new(TreeNodeId(5), "README.md"),
    ])
}

/// A root with `n` children, all expanded — a flat list of `n + 1` visible rows
/// and therefore a fixture whose *shape* is identical for any `n`. Comparing two
/// of these isolates the node count; comparing differently shaped trees would
/// only show that they differ.
fn wide_model(n: u64) -> VecTree {
    let children: Vec<TreeNode> = (0..n)
        .map(|i| TreeNode::new(TreeNodeId(i + 10), format!("node {i}")))
        .collect();
    let mut model = VecTree::new(vec![
        TreeNode::new(TreeNodeId(1), "root").child_many(children),
    ]);
    model.expand_all();
    model
}

fn tree_with(model: VecTree) -> Tree {
    let mut tree = Tree::new(model)
        .row_height(ROW_H)
        .selection_mode(SelectionMode::Multiple)
        .overscan(Overscan::Items(0));
    tree.set_container(rect(TEN_ROWS));
    tree
}

fn allocate(widget: &dyn Widget) -> ElementId {
    let mut create = AllocatingCreateContext::new();
    widget.create(&mut create)
}

fn key(tree: &mut Tree, id: ElementId, key: Key, modifiers: Modifiers) -> bool {
    let event = KeyEvent {
        phase: KeyPhase::Down,
        key,
        modifiers,
    };
    dispatch(tree, id, rect(TEN_ROWS), event.into())
}

fn named(n: NamedKey) -> Key {
    Key::Named(n)
}

fn labels(tree: &Tree) -> Vec<String> {
    tree.rows().iter().map(|r| r.label.clone()).collect()
}

// --- flattening ------------------------------------------------------------

#[test]
fn a_collapsed_tree_shows_only_its_roots() {
    let tree = tree_with(model());
    assert_eq!(labels(&tree), vec!["src", "README.md"]);
    assert_eq!(tree.rows()[0].depth, 0);
    assert!(tree.rows()[0].has_children);
    assert!(!tree.rows()[0].expanded);
    assert!(!tree.rows()[1].has_children, "a leaf discloses nothing");
}

#[test]
fn expanding_a_node_splices_its_children_in_below_it() {
    let mut tree = tree_with(model());
    assert!(tree.expand(TreeNodeId(1)));
    assert_eq!(labels(&tree), vec!["src", "main.rs", "lib.rs", "README.md"]);
    assert_eq!(tree.rows()[1].depth, 1);
    assert!(!tree.rows()[1].expanded, "children start collapsed");

    // Only the inner node's children are hidden, and expanding it shows them
    // at depth 2 — depth is a property of the visible path, not the node.
    tree.expand(TreeNodeId(2));
    assert_eq!(
        labels(&tree),
        vec!["src", "main.rs", "deep.rs", "lib.rs", "README.md"]
    );
    assert_eq!(tree.rows()[2].depth, 2);
}

#[test]
fn collapsing_hides_the_whole_subtree_including_its_expansions() {
    let mut tree = tree_with(model());
    tree.expand(TreeNodeId(1));
    tree.expand(TreeNodeId(2));
    assert_eq!(tree.rows().len(), 5);

    tree.collapse(TreeNodeId(1));
    assert_eq!(labels(&tree), vec!["src", "README.md"]);
    // The inner node is still marked expanded, so re-opening the parent
    // restores what the user had open rather than resetting the subtree.
    assert!(tree.is_expanded(TreeNodeId(2)));
    tree.expand(TreeNodeId(1));
    assert_eq!(tree.rows().len(), 5);
}

#[test]
fn expand_and_collapse_report_whether_anything_changed() {
    let mut tree = tree_with(model());
    assert!(tree.expand(TreeNodeId(1)));
    assert!(!tree.expand(TreeNodeId(1)), "already open");
    assert!(tree.collapse(TreeNodeId(1)));
    assert!(!tree.collapse(TreeNodeId(1)), "already closed");
    // A leaf can never be expanded.
    assert!(!tree.expand(TreeNodeId(5)));
    assert!(!tree.toggle(TreeNodeId(5)));
}

#[test]
fn expand_all_and_collapse_all_cover_the_whole_forest() {
    let mut tree = tree_with(model());
    assert!(tree.expand_all());
    assert_eq!(
        labels(&tree),
        vec!["src", "main.rs", "deep.rs", "lib.rs", "README.md"]
    );
    assert!(tree.collapse_all());
    assert_eq!(labels(&tree), vec!["src", "README.md"]);
    // An already-collapsed tree has nothing to collapse.
    let mut fresh = tree_with(model());
    assert!(!fresh.collapse_all());
}

#[test]
fn toggle_flips_one_node() {
    let mut tree = tree_with(model());
    assert!(tree.toggle(TreeNodeId(1)));
    assert!(tree.is_expanded(TreeNodeId(1)));
    assert!(tree.toggle(TreeNodeId(1)));
    assert!(!tree.is_expanded(TreeNodeId(1)));
}

// --- navigation ------------------------------------------------------------

#[test]
fn right_expands_then_drills_into_the_first_child() {
    let mut tree = tree_with(model());
    let id = allocate(&tree);
    tree.select_row(0);

    // Collapsed node: Right opens it and leaves the cursor where it was.
    assert!(key(
        &mut tree,
        id,
        named(NamedKey::ArrowRight),
        Modifiers::NONE
    ));
    assert!(tree.is_expanded(TreeNodeId(1)));
    assert_eq!(tree.selection().cursor(), Some(TreeNodeId(1)), "stays put");

    // Already open: Right moves to the first child.
    key(&mut tree, id, named(NamedKey::ArrowRight), Modifiers::NONE);
    assert_eq!(tree.selection().cursor(), Some(TreeNodeId(2)));

    // main.rs has a child of its own, so it opens the same way, and then
    // drills in again.
    assert!(key(
        &mut tree,
        id,
        named(NamedKey::ArrowRight),
        Modifiers::NONE
    ));
    assert!(tree.is_expanded(TreeNodeId(2)));
    key(&mut tree, id, named(NamedKey::ArrowRight), Modifiers::NONE);
    assert_eq!(tree.selection().cursor(), Some(TreeNodeId(4)));

    // deep.rs is a leaf: nothing to open, so Right is a no-op.
    assert!(!key(
        &mut tree,
        id,
        named(NamedKey::ArrowRight),
        Modifiers::NONE
    ));
}

#[test]
fn left_collapses_then_moves_to_the_parent() {
    let mut tree = tree_with(model());
    let id = allocate(&tree);
    tree.expand_all();
    tree.select_row(2); // deep.rs, depth 2

    // On a leaf, Left walks up to its parent without collapsing anything.
    assert!(key(
        &mut tree,
        id,
        named(NamedKey::ArrowLeft),
        Modifiers::NONE
    ));
    assert_eq!(tree.selection().cursor(), Some(TreeNodeId(2)));
    assert!(
        tree.is_expanded(TreeNodeId(2)),
        "walking up does not collapse"
    );

    // On the now-current open parent, Left closes it.
    assert!(key(
        &mut tree,
        id,
        named(NamedKey::ArrowLeft),
        Modifiers::NONE
    ));
    assert!(!tree.is_expanded(TreeNodeId(2)));

    // Closed, Left walks up again.
    assert!(key(
        &mut tree,
        id,
        named(NamedKey::ArrowLeft),
        Modifiers::NONE
    ));
    assert_eq!(tree.selection().cursor(), Some(TreeNodeId(1)));

    // An open root collapses rather than climbing out of the tree; once it is
    // closed there is nowhere left to go.
    assert!(key(
        &mut tree,
        id,
        named(NamedKey::ArrowLeft),
        Modifiers::NONE
    ));
    assert!(!tree.is_expanded(TreeNodeId(1)));
    assert!(!key(
        &mut tree,
        id,
        named(NamedKey::ArrowLeft),
        Modifiers::NONE
    ));
}

#[test]
fn up_and_down_walk_the_visible_rows_only() {
    let mut tree = tree_with(model());
    let id = allocate(&tree);
    key(&mut tree, id, named(NamedKey::ArrowDown), Modifiers::NONE);
    assert_eq!(tree.selection().cursor(), Some(TreeNodeId(1)));
    key(&mut tree, id, named(NamedKey::ArrowDown), Modifiers::NONE);
    assert_eq!(
        tree.selection().cursor(),
        Some(TreeNodeId(5)),
        "a collapsed subtree is skipped"
    );

    tree.expand_all();
    tree.select_row(1);
    key(&mut tree, id, named(NamedKey::ArrowDown), Modifiers::NONE);
    assert_eq!(tree.selection().cursor(), Some(TreeNodeId(4)), "deep.rs");
}

// --- pointer ---------------------------------------------------------------

#[test]
fn clicking_the_disclosure_toggles_and_clicking_the_label_selects() {
    let mut tree = tree_with(model());
    let id = allocate(&tree);

    // x = 4 is inside depth 0's disclosure column.
    assert!(dispatch(
        &mut tree,
        id,
        rect(TEN_ROWS),
        PointerEvent::primary_down(Point::new(4.0, 10.0)).into()
    ));
    assert!(tree.is_expanded(TreeNodeId(1)));
    assert_eq!(
        tree.selection().cursor(),
        None,
        "the toggle is not a selection"
    );

    // x = 40 is in the label area.
    assert!(dispatch(
        &mut tree,
        id,
        rect(TEN_ROWS),
        PointerEvent::primary_down(Point::new(40.0, 10.0)).into()
    ));
    assert_eq!(tree.selection().cursor(), Some(TreeNodeId(1)));
    assert!(tree.is_expanded(TreeNodeId(1)), "selecting does not toggle");
}

#[test]
fn a_nested_row_indents_its_disclosure_column() {
    let mut tree = tree_with(model());
    tree.expand_all();
    // Row 1 is depth 1, so its triangle sits one indent further right.
    let b = tree.row_disclosure_bounds(rect(TEN_ROWS), 1).unwrap();
    assert_eq!(b.x0, INDENT);
    assert_eq!(b.x1, 2.0 * INDENT);
    assert_eq!(b.y0, ROW_H);
    assert_eq!(b.y1, 2.0 * ROW_H);
}

// --- virtualization --------------------------------------------------------

#[test]
fn the_visible_window_and_the_row_pool_stay_bounded_on_a_huge_tree() {
    // One root with 100k children: a flat list as far as the window is
    // concerned, so the same contract as the grid has to hold.
    let mut tree = tree_with(wide_model(100_000));
    assert_eq!(tree.rows().len(), 100_001);
    assert_eq!(
        tree.visible_range(),
        0..10,
        "the window is a box, not a list"
    );

    let mut pool = RowPool::new();
    tree.sync_rows(&mut pool);
    for step in 0..1_000 {
        tree.scroll_to(f64::from(step) * 20.0);
        tree.sync_rows(&mut pool);
    }
    assert!(
        pool.high_water() <= 10,
        "pool grew to {} — rows leak",
        pool.high_water()
    );
}

#[test]
fn paint_cost_is_flat_in_the_node_count() {
    // Two trees of the same shape, ten times the node count apart. Both windows
    // are the same ten rows, so the emitted ops must be identical — that is the
    // whole claim of "virtualized", and a differently shaped small tree could
    // not make it.
    let small = tree_with(wide_model(10_000));
    let big = tree_with(wide_model(100_000));
    assert_eq!(small.rows().len(), 10_001);
    assert_eq!(big.rows().len(), 100_001);

    let id = allocate(&small);
    let mut small_rec = SceneRecorder::new(rect(TEN_ROWS));
    small.paint(&mut small_rec, id);
    small_rec.finish();

    let big_id = allocate(&big);
    let mut big_rec = SceneRecorder::new(rect(TEN_ROWS));
    big.paint(&mut big_rec, big_id);
    big_rec.finish();

    assert_eq!(
        small_rec.ops().len(),
        big_rec.ops().len(),
        "paint ops grew with the node count"
    );
    assert!(big_rec.ops().len() < 100);
}

// --- semantics -------------------------------------------------------------

#[test]
fn semantics_report_the_visible_and_expanded_counts() {
    let mut tree = tree_with(model());
    let id = allocate(&tree);
    let label = semantics_snapshot(&tree, id, rect(TEN_ROWS));
    assert!(label.contains("2 rows"), "got {label:?}");

    tree.expand_all();
    let label = semantics_snapshot(&tree, id, rect(TEN_ROWS));
    assert!(label.contains("5 rows"), "got {label:?}");
    // Only id 1 and id 2 have children, so only those two can be expanded.
    assert!(label.contains("2 expanded"), "got {label:?}");
}

#[test]
fn a_tree_is_type_erasable_like_every_other_widget() {
    let tree = tree_with(model());
    let erased: &dyn WidgetAny = &tree;
    assert!(erased.as_any().downcast_ref::<Tree>().is_some());
}
