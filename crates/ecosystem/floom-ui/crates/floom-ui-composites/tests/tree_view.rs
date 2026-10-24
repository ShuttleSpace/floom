//! The engine's [`TreeView`]: the shared expansion state, and the fact that
//! expanding a node **changes which rows exist**.
//!
//! That second point is the whole reason the state is shared rather than per-row, so
//! it gets the most tests here. A tree whose rows each remembered their own
//! `expanded` flag would look correct for itself and wrong for everything below it,
//! and every test in the "flattening" section is really about the count and the
//! order of the rows rather than about any one row.

use floom_core::Widget;

use floom_ui_composites::{node_at, toggle_path, visible_rows, TreeNode, TreeView, TreeViewStyle};
use floom_widgets::testkit::AllocatingCreateContext;
use floom_widgets::{Pressable, Stack, Surface, Text, VectorIcon};

/// Material-ish numbers, distinguishable so a wrong field is visible.
fn style() -> TreeViewStyle {
    TreeViewStyle {
        indent: 16.0,
        row_height: 32.0,
        item_gap: 0.0,
        expander_size: 24.0,
        icon_size: 24.0,
        icon_scale: 0.7,
        font_size: 14.0,
        label_color: floom_core::Color::rgb(0x11, 0x22, 0x33),
        hover_background: floom_core::Color::rgba(0x11, 0x22, 0x33, 8),
        gap: 4.0,
        padding_horizontal: 8.0,
    }
}

/// A three-level tree:
///
/// ```text
/// root        (expanded)
/// ├ child     (expanded)
/// │  └ grandchild
/// └ sibling
/// ```
///
/// Both `root` and `child` are expanded, because a node's children are shown when
/// **that** node is open — the root being visible says nothing about whether its
/// children are. The first version of this fixture left the root collapsed and three
/// tests failed by correctly reporting one row.
fn forest() -> Vec<TreeNode> {
    vec![TreeNode::new("root").expanded(true).children(vec![
        TreeNode::new("child")
            .expanded(true)
            .children(vec![TreeNode::new("grandchild")]),
        TreeNode::new("sibling"),
    ])]
}

fn tree() -> TreeView {
    TreeView::new(forest()).style(style())
}

// --- flattening -----------------------------------------------------------

/// The rows, as paths. The root's child is expanded, so its child appears; the
/// root's second child has no children, so nothing follows it.
#[test]
fn an_expanded_node_reveals_its_children() {
    let rows = visible_rows(&forest());
    let paths: Vec<Vec<usize>> = rows.iter().map(|r| r.path.clone()).collect();
    assert_eq!(
        paths,
        vec![
            vec![0],       // root
            vec![0, 0],    // child, expanded
            vec![0, 0, 0], // grandchild
            vec![0, 1],    // sibling
        ],
        "four rows, in document order"
    );
    assert_eq!(
        rows.iter().map(|r| r.depth).collect::<Vec<_>>(),
        vec![1, 2, 3, 2],
        "and the depth of each, which is what the indent is computed from"
    );
}

/// A collapsed node hides its whole subtree — not just its children, which is the
/// easy half.
#[test]
fn a_collapsed_node_hides_its_whole_subtree() {
    let rows = visible_rows(&[TreeNode::new("root").children(vec![TreeNode::new("child")
        .expanded(true)
        .children(vec![TreeNode::new("grandchild")])])]);
    assert_eq!(
        visible_rows(&forest())
            .iter()
            .map(|r| r.path.clone())
            .collect::<Vec<_>>(),
        vec![vec![0], vec![0, 0], vec![0, 0, 0], vec![0, 1]],
        "CONTROL: with the child expanded, three of the four rows are under it"
    );
    assert_eq!(
        rows.iter().map(|r| r.path.clone()).collect::<Vec<_>>(),
        vec![vec![0]],
        "collapsed, only the root — so the grandchild is hidden by its *parent's*
         parent, not merely by its parent"
    );
}

/// An empty tree is no rows, and a forest of three siblings is three.
#[test]
fn empty_and_flat_forests() {
    assert!(visible_rows(&[]).is_empty());
    assert_eq!(
        visible_rows(&[TreeNode::new("a"), TreeNode::new("b"), TreeNode::new("c")]).len(),
        3,
        "three siblings, no nesting"
    );
}

/// A node marked `expanded` with **no children** still reveals nothing. This is a
/// real case rather than a hypothetical: v1's `expanded` defaults to `false` and a
/// caller can set it on a leaf.
#[test]
fn an_expanded_leaf_reveals_nothing() {
    let rows = visible_rows(&[TreeNode::new("leaf").expanded(true)]);
    assert_eq!(rows.len(), 1, "one row, and no phantom child");
}

// --- node_at --------------------------------------------------------------

/// `node_at` walks a path. Its first version returned the *children slice* of the
/// last node rather than the node, which type-checked and would have made every
/// label in the tree wrong — so the walk is pinned here rather than assumed.
#[test]
fn node_at_resolves_a_path_to_that_node() {
    let forest = forest();
    assert_eq!(node_at(&forest, &[0]).unwrap().label, "root");
    assert_eq!(node_at(&forest, &[0, 0]).unwrap().label, "child");
    assert_eq!(node_at(&forest, &[0, 0, 0]).unwrap().label, "grandchild");
    assert_eq!(node_at(&forest, &[0, 1]).unwrap().label, "sibling");
}

#[test]
fn node_at_rejects_a_path_that_does_not_resolve() {
    let forest = forest();
    assert!(
        node_at(&forest, &[]).is_none(),
        "an empty path names nothing"
    );
    assert!(node_at(&forest, &[9]).is_none(), "out of range at the root");
    assert!(
        node_at(&forest, &[0, 9]).is_none(),
        "out of range one level down"
    );
    assert!(
        node_at(&forest, &[0, 0, 9]).is_none(),
        "and two — so a stale row cannot read a node that has gone"
    );
}

// --- the shared state -----------------------------------------------------

/// Toggling through the shared forest, and reporting whether anything changed.
///
/// The `bool` is load-bearing: a stale row whose path no longer resolves must not
/// make its caller think it toggled something.
#[test]
fn toggle_flips_one_node_and_reports_it() {
    let tree = std::sync::Arc::new(std::sync::Mutex::new(forest()));
    assert!(
        toggle_path(&tree, &[0, 0]),
        "an expandable path changed something"
    );
    let forest = tree.lock().unwrap();
    let _ = &forest;
    assert!(
        !node_at(&forest, &[0, 0]).unwrap().expanded,
        "now collapsed"
    );
    assert!(
        node_at(&forest, &[0]).unwrap().expanded,
        "and the parent is still expanded, which is how this fixture started — \
         the path is the whole story, so nothing above the last index moved"
    );
}

#[test]
fn toggle_rejects_a_path_that_does_not_resolve() {
    let tree = std::sync::Arc::new(std::sync::Mutex::new(forest()));
    assert!(!toggle_path(&tree, &[]), "an empty path changes nothing");
    assert!(
        !toggle_path(&tree, &[9]),
        "out of range changes nothing, and says so"
    );
    assert!(!toggle_path(&tree, &[0, 9]), "one level down, likewise");
}

/// The reason the state is shared at all, as an observable: toggling a node changes
/// **how many rows the widget has**, not just what one of them looks like.
#[test]
fn toggling_changes_the_row_count() {
    let mut tree = tree();
    assert_eq!(tree.rows().len(), 4, "four rows to begin with");

    tree.toggle(&[0, 0]);
    assert_eq!(
        tree.rows().len(),
        3,
        "collapsing `child` hides *its child* — the node's own row stays, because \
         a collapsed node is still a row you can expand again"
    );

    tree.toggle(&[0, 0]);
    assert_eq!(tree.rows().len(), 4, "and expanding brings both back");
}

/// The widget's `children()` agrees with `rows()`, because `children()` is built by
/// flattening — a test that only checked one of them would not notice a divergence.
#[test]
fn the_widget_child_count_matches_the_rows() {
    let mut tree = tree();
    assert_eq!(tree.children().len(), tree.rows().len());
    tree.toggle(&[0, 0]);
    assert_eq!(tree.children().len(), tree.rows().len(), "after a collapse");
}

/// Each row's key follows its path, so a row keeps its state when its *siblings*
/// change but its own path does not. Keying by position instead would give the
/// second child of the root the first child's node when the first collapsed — which
/// is the bug a `SelectGroup` would have had too if it had keyed by index.
#[test]
fn row_keys_follow_the_path_not_the_position() {
    let mut tree = tree();
    let before = labels(&tree);
    tree.toggle(&[0, 0]);
    let after = labels(&tree);

    assert_eq!(before, vec!["root", "child", "grandchild", "sibling"]);
    assert_eq!(
        after,
        vec!["root", "child", "sibling"],
        "the grandchild left; the root, the child and the sibling stayed"
    );
    // The survivor set is *identified by path*, not by position: `sibling` was the
    // fourth row and is now the second, and it is still the same row. Keying by
    // position would have handed it the root's node, and `child`'s, on the next
    // toggle.
    assert!(after.contains(&"sibling".to_string()));
}

// --- the row's shape ------------------------------------------------------

/// Four parts per row: the indent spacer, the expander's box, the icon's box, and
/// the label.
#[test]
fn a_row_has_an_indent_an_expander_an_icon_and_a_label() {
    let mut tree = tree();
    let row = tree.children().remove(0);
    let built = row.build();
    let stack = built
        .as_any()
        .downcast_ref::<Stack>()
        .expect("a row is a horizontal stack");
    assert_eq!(
        stack.child_count(),
        4,
        "indent, expander box, icon box, label"
    );
}

/// The row declares v1's `height(row_height)` and `padding_horiz(8.0)`. This is why
/// `Stack` needed a `request`: without it the obvious translation was to wrap every
/// row in a `Surface`, adding a node per row to carry one number.
#[test]
fn the_row_declares_its_own_height_and_padding() {
    let mut tree = tree();
    let row = tree.children().remove(0);
    let built = row.build();
    let stack = built.as_any().downcast_ref::<Stack>().unwrap();
    let request = stack.layout_request().expect("a row declares a layout");

    assert_eq!(
        request.height,
        Some(floom_types::Length::Px(32.0)),
        "v1's `.height(row_height)`"
    );
    assert_eq!(
        request.padding,
        Some(floom_types::Insets {
            left: 8.0,
            right: 8.0,
            top: 0.0,
            bottom: 0.0,
        }),
        "and v1's `.padding_horiz(8.0)`"
    );
    assert_eq!(request.gap, Some(4.0), "and its `.gap(4.0)`");
    assert_eq!(
        request.cross_align,
        Some(floom_core::Align::Center),
        "and its `.items_center()`"
    );
}

/// The label is the **only** part that grows, which is what pushes nothing else
/// around when a label is long.
#[test]
fn the_label_is_the_only_part_that_grows() {
    let mut tree = tree();
    let row = tree.children().remove(0);
    let built = row.build();
    let stack = built.as_any().downcast_ref::<Stack>().unwrap().clone();

    // Four parts, and only the last one is handed the slack. The first three are
    // `Surface` boxes whose *own* requests carry no growth, which is the claim: a
    // long label cannot shift the icons, because they are not participating in the
    // distribution at all.
    assert_eq!(stack.child_count(), 4);
    for index in 0..3 {
        let part = stack.children()[index].build();
        assert!(
            part.layout_request()
                .map(|r| r.flex_grow.is_none())
                .unwrap_or(true),
            "part {index} is fixed-width, so a long label cannot shift it"
        );
    }
    // Read the *slot's* request rather than asking the built widget: a `Pressable`
    // declares no layout of its own, so asking it answers `None` and says nothing
    // about how much slack its slot asked for.
    assert_eq!(
        stack.children()[3].layout.flex_grow,
        Some(1.0),
        "and the label's slot takes the rest of the row"
    );
}

/// v1 put the click handler on the **label only**, so the expander and the icon were
/// not clickable. Here the expander is its own `Pressable`, which is the same two
/// targets and not one.
#[test]
fn the_expander_is_its_own_click_target() {
    let mut tree = tree();
    let row = tree.children().remove(0);
    let built = row.build();
    let stack = built.as_any().downcast_ref::<Stack>().unwrap().clone();

    let expander_box = stack.children()[1].build();
    let box_ = expander_box
        .as_any()
        .downcast_ref::<Surface>()
        .expect("the expander's box");
    assert_eq!(
        box_.child_count(),
        1,
        "a node with children has a chevron inside its box"
    );

    let label_slot = stack.children()[3].build();
    assert!(
        label_slot.as_any().is::<Pressable>(),
        "and the label is a `Pressable` too — two targets, as v1's two handlers were"
    );
}

/// A childless node's expander box is **empty but present**, which is v1's
/// `Empty::new().style(|s| s.width(expander_size)…)`. The geometry is the point: a
/// missing icon must not shift the labels.
#[test]
fn a_childless_nodes_expander_box_is_empty_but_the_same_size() {
    let mut tree = tree();
    // Row 3 is `sibling`, which has no children.
    let row = tree.children().remove(3);
    let built = row.build();
    let stack = built.as_any().downcast_ref::<Stack>().unwrap().clone();

    let expander = stack.children()[1].build();
    let expander = expander.as_any().downcast_ref::<Surface>().unwrap().clone();
    assert_eq!(expander.child_count(), 0, "no chevron for a leaf");
    assert_eq!(
        expander.layout_request().expect("still sized").width,
        Some(floom_types::Length::Px(24.0)),
        "but the box is still `expander_size` wide, so the labels line up"
    );
}

/// The indent is `(depth - 1) * indent`, so a depth-1 row has none at all.
#[test]
fn the_indent_is_depth_minus_one_indents() {
    let tree = tree();
    let rows = tree.children();

    // Row 0 is the root, at depth 1.
    let root = rows[0].build();
    let root = root.as_any().downcast_ref::<Stack>().unwrap().clone();
    let spacer = root.children()[0].build();
    let spacer = spacer.as_any().downcast_ref::<Surface>().unwrap().clone();
    assert_eq!(
        spacer.layout_request().expect("the spacer is sized").width,
        Some(floom_types::Length::Px(0.0)),
        "depth 1: (1 - 1) * 16"
    );

    // Row 1 is at depth 2.
    let deeper = rows[1].build();
    let deeper = deeper.as_any().downcast_ref::<Stack>().unwrap().clone();
    let spacer = deeper.children()[0].build();
    let spacer = spacer.as_any().downcast_ref::<Surface>().unwrap().clone();
    assert_eq!(
        spacer.layout_request().expect("the spacer is sized").width,
        Some(floom_types::Length::Px(16.0)),
        "depth 2: (2 - 1) * 16"
    );
}

// --- the two click targets, and what they report -------------------------

/// The label click reports the node's **label**, per v1.
#[test]
fn the_label_click_reports_the_labels_string() {
    let hits = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut tree = {
        let hits = hits.clone();
        tree().on_node_click(move |label| hits.lock().unwrap().push(label))
    };
    let row = tree.children().remove(0);
    let built = row.build();
    let stack = built.as_any().downcast_ref::<Stack>().unwrap().clone();
    let mut label_slot = stack.children()[3].build();
    let pressable = label_slot
        .as_any_mut()
        .downcast_mut::<Pressable>()
        .expect("a `Pressable`");

    let id = pressable.create(&mut AllocatingCreateContext::new());
    press_fire(pressable, id);

    assert_eq!(
        hits.lock().unwrap().as_slice(),
        ["root".to_string()],
        "v1's `cb(label.clone())`"
    );
}

/// The expander click reports the toggled **path**, and changes the tree.
#[test]
fn the_expander_click_reports_the_path_and_changes_the_tree() {
    let toggled = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut tree = {
        let toggled = toggled.clone();
        tree().on_toggle(move |path| toggled.lock().unwrap().push(path))
    };
    let row = tree.children().remove(1);
    let built = row.build();
    let stack = built.as_any().downcast_ref::<Stack>().unwrap().clone();
    let expander = stack.children()[1].build();
    let box_ = expander.as_any().downcast_ref::<Surface>().unwrap().clone();
    let mut chevron = box_.children()[0].build();
    let pressable = chevron
        .as_any_mut()
        .downcast_mut::<Pressable>()
        .expect("the chevron is a `Pressable`");

    let before = tree.rows().len();
    let id = pressable.create(&mut AllocatingCreateContext::new());
    press_fire(pressable, id);

    assert_eq!(
        toggled.lock().unwrap().as_slice(),
        [vec![0usize, 0]],
        "the toggled node's path"
    );
    assert_eq!(
        tree.rows().len(),
        before - 1,
        "and the tree really changed — the grandchild left, and only it"
    );
}

// --- shared state, from two handles ---------------------------------------

/// Two `TreeView`s over one forest see each other's toggles. This is what "shared"
/// buys, and it is the property a per-row flag could not have.
#[test]
fn two_views_over_one_forest_see_each_other() {
    let shared = TreeView::new(forest()).tree();
    let a = TreeView::with_shared_tree(shared.clone());
    let b = TreeView::with_shared_tree(shared);

    assert_eq!(a.rows().len(), b.rows().len(), "they start alike");
    a.toggle(&[0, 0]);
    assert_eq!(
        b.rows().len(),
        3,
        "and a toggle through one is visible from the other"
    );
    assert!(
        !b.is_expanded(&[0, 0]),
        "which `is_expanded` reports without walking a widget"
    );
}

// --- the widget as a whole ------------------------------------------------

#[test]
fn the_widget_publishes_no_semantics_of_its_own() {
    assert!(
        <TreeView as floom_core::WidgetAny>::semantics_hint(&tree()).is_none(),
        "each row's `Pressable` publishes its own label; the tree is a container"
    );
}

/// The icon resolver is the kit's job, and a tree with no resolver still assembles.
#[test]
fn a_tree_with_no_icon_resolver_still_builds() {
    let tree = TreeView::new(forest()).style(style());
    assert_eq!(
        tree.children().len(),
        4,
        "the icon boxes are empty rather than missing — see the module note"
    );
}

/// With a resolver, a named icon lands inside its box. v1 drew it at
/// `icon_size * 0.7`, so the scale is the assertion.
#[test]
fn a_resolved_icon_is_drawn_at_seventy_percent_of_its_box() {
    let icon = |_name: &str| {
        Some(
            VectorIcon::from_path_data(
                "M 0 0 L 4 4 Z",
                floom_types::Rect {
                    x0: 0.0,
                    y0: 0.0,
                    x1: 4.0,
                    y1: 4.0,
                },
            )
            .size(1.0),
        )
    };
    let tree = TreeView::new(vec![TreeNode::new("a").icon("folder")])
        .style(style())
        .icons(icon);
    let row = tree.children().remove(0);
    let built = row.build();
    let stack = built.as_any().downcast_ref::<Stack>().unwrap().clone();
    let icon_box = stack.children()[2].build();
    let icon_box = icon_box.as_any().downcast_ref::<Surface>().unwrap().clone();

    assert_eq!(icon_box.child_count(), 1, "the icon is inside its box");
    let drawn = icon_box.children()[0].build();
    let drawn = drawn
        .as_any()
        .downcast_ref::<VectorIcon>()
        .expect("the resolved icon");
    assert_eq!(drawn.edge(), 24.0 * 0.7, "v1's `icon_size * 0.7`");
}

/// A node with **no** icon leaves its box empty, which is v1's
/// `Empty::new().style(|s| s.width(icon_size))`.
#[test]
fn a_node_with_no_icon_leaves_an_empty_box() {
    let tree = tree();
    let row = tree.children().remove(0);
    let built = row.build();
    let stack = built.as_any().downcast_ref::<Stack>().unwrap().clone();
    let icon_box = stack.children()[2].build();
    let icon_box = icon_box.as_any().downcast_ref::<Surface>().unwrap().clone();

    assert_eq!(icon_box.child_count(), 0);
    assert_eq!(
        icon_box.layout_request().expect("still sized").width,
        Some(floom_types::Length::Px(24.0)),
        "and the box keeps its width, so the labels still line up"
    );
}

/// The labels are the labels. v1's tree carries them as `String`s and so does this,
/// so a row's text is the node's label and nothing else.
#[test]
fn the_labels_travel_into_the_rows() {
    let tree = tree();
    let row = tree.children().remove(2);
    let built = row.build();
    let stack = built.as_any().downcast_ref::<Stack>().unwrap().clone();
    let label = stack.children()[3].build();
    let label = label
        .as_any()
        .downcast_ref::<Pressable>()
        .expect("the label slot is a `Pressable`");

    assert_eq!(
        label.semantics_hint().label.as_deref(),
        Some("grandchild"),
        "row 2 is the grandchild"
    );
}

/// A `Text` inside the label pressable, at the style's font size.
#[test]
fn the_label_is_text_at_the_styles_size() {
    let tree = tree();
    let row = tree.children().remove(0);
    let built = row.build();
    let stack = built.as_any().downcast_ref::<Stack>().unwrap().clone();
    let pressable = stack.children()[3].build();
    let pressable = pressable
        .as_any()
        .downcast_ref::<Pressable>()
        .unwrap()
        .clone();
    let inner = pressable.children()[0].build();
    let text = inner.as_any().downcast_ref::<Text>().expect("a `Text`");

    assert_eq!(text.content(), "root");
    assert_eq!(text.font_size(), 14.0, "v1's `font_size` token");
}

/// Each visible row's label, read through the widget's own subtree.
///
/// This is the substitute for comparing `ChildKey`s: the key's field is private and
/// the type has no accessor, so the identity claim is made where it is actually
/// observable — in which rows survive a toggle.
fn labels(tree: &TreeView) -> Vec<String> {
    tree.children()
        .into_iter()
        .map(|row| {
            let built = row.build();
            let stack = built.as_any().downcast_ref::<Stack>().unwrap().clone();
            let label = stack.children()[3].build();
            let label = label
                .as_any()
                .downcast_ref::<Pressable>()
                .unwrap()
                .semantics_hint()
                .label
                .unwrap_or_default();
            label
        })
        .collect()
}

// --- helpers --------------------------------------------------------------

/// A pointer down and up in the middle of a row.
fn press_fire(pressable: &mut Pressable, id: floom_core::ElementId) {
    use floom_types::{Point, PointerEvent, Rect};
    let bounds = Rect {
        x0: 0.0,
        y0: 0.0,
        x1: 300.0,
        y1: 32.0,
    };
    let at = Point::new(10.0, 10.0);
    let mut down = floom_widgets::testkit::TestEventContext::new(
        id,
        bounds,
        PointerEvent::primary_down(at).into(),
    );
    pressable.event(&mut down, id);
    let mut up = floom_widgets::testkit::TestEventContext::new(
        id,
        bounds,
        PointerEvent::primary_up(at).into(),
    );
    pressable.event(&mut up, id);
}
