//! The hot-reload contract, end to end.
//!
//! The load-bearing claim: a compatible patch preserves the ids of untouched
//! nodes *and everything keyed by them*. Two controls keep the claim honest:
//! renaming every key must NOT preserve (otherwise "preserved" could just
//! mean the diff matched nothing), and a failed patch must leave no trace
//! (otherwise "transactional" would be a word, not a behaviour).

use floem_hot_reload::{LiveTree, NodeKey, PatchOp, UiPatch};
use floem_runtime_core::{DirtyFlags, DirtySet};
use floem_style::StyleState;
use floem_types::Color;

const BEFORE: &str = r#"
root = stack axis=vertical gap=8 {
    heading = text label="hello" background=#112233
    body = stack axis=horizontal gap=4 width=100 {
        ok = button label="OK"
    }
}
"#;

/// One edit later: the gap changed, the heading was recoloured, `body` grew,
/// a sibling arrived after `body`, and `ok` gained a click handler.
const AFTER: &str = r#"
bindings { save: handler }
root = stack axis=vertical gap=12 {
    heading = text label="hello" background=#445566
    body = stack axis=horizontal gap=4 width=200 {
        ok = button label="OK" on:click=save
    }
    extra = text label="new"
}
"#;

/// The same shape as `BEFORE` with every key renamed — the control group.
const RENAMED: &str = r#"
root2 = stack axis=vertical gap=8 {
    heading2 = text label="hello" background=#112233
    body2 = stack axis=horizontal gap=4 width=100 {
        ok2 = button label="OK"
    }
}
"#;

fn compile(source: &str) -> floem_ui_ir::UiDocument {
    floem_dsl::compile(source).expect("valid document").document
}

fn key(name: &str) -> NodeKey {
    NodeKey::new(name)
}

#[test]
fn ids_and_stashed_state_survive_a_compatible_patch() {
    let (before, after) = (compile(BEFORE), compile(AFTER));
    let mut tree = LiveTree::new();
    tree.mount(&before);
    let count_before = tree.element_count();

    // Stash some state the way a host would: a scroll offset on `body`.
    assert!(tree.set_stash(&key("body"), "scroll=42"));
    let body_before = tree.id_of(&key("body")).expect("body mounted");
    let heading_before = tree.id_of(&key("heading")).expect("heading mounted");

    let patch = floem_hot_reload::diff(&before, &after);
    let mut dirty = DirtySet::new();
    let report = tree.apply(&patch, &mut dirty).expect("applies");

    // The untouched node kept its element: same id, same stash.
    assert_eq!(tree.id_of(&key("body")), Some(body_before));
    assert_eq!(tree.stash_of(&key("body")), Some("scroll=42"));
    assert_eq!(
        tree.id_of(&key("heading")),
        Some(heading_before),
        "the heading did not change structurally, so its id must survive"
    );

    // The edit took: the style seat, the layout seat and the new wiring.
    let heading = tree
        .style_of(&key("heading"), StyleState::NORMAL)
        .expect("heading styled");
    assert_eq!(
        heading.background,
        Some(Color::rgba(0x44, 0x55, 0x66, 0xff)),
        "the recolour landed: {:?}",
        heading
    );
    assert_eq!(tree.layout_of(&key("body")).unwrap().width, Some(200.0));
    assert_eq!(tree.events_of(&key("ok")).len(), 1, "click wired");

    // The insert is real.
    assert_eq!(report.mounted, vec![key("extra")]);
    assert!(report.unmounted.is_empty());
    assert_eq!(tree.element_count(), count_before + 1);
}

#[test]
fn the_control_group_renamed_keys_do_not_preserve() {
    // If renaming every key *also* preserved ids, the test above would be
    // proving nothing: preservation has to come from key matching, not from
    // the diff being blind.
    let (before, renamed) = (compile(BEFORE), compile(RENAMED));
    let mut tree = LiveTree::new();
    tree.mount(&before);
    assert!(tree.set_stash(&key("body"), "scroll=42"));
    let root_before = tree.id_of(&key("root")).expect("root mounted");

    let patch = floem_hot_reload::diff(&before, &renamed);
    assert!(
        patch
            .ops
            .iter()
            .any(|op| matches!(op, PatchOp::Remove { key } if key.as_str() == "root")),
        "renaming the root key is a remove-and-insert, not an update"
    );
    let mut dirty = DirtySet::new();
    tree.apply(&patch, &mut dirty).expect("applies");

    let root_after = tree.id_of(&key("root2")).expect("renamed root mounted");
    assert_ne!(root_after, root_before, "a renamed node gets a fresh id");
    assert!(tree.id_of(&key("body")).is_none(), "the old key is gone");
    assert_eq!(
        tree.stash_of(&key("body2")),
        None,
        "state does not follow renames"
    );
}

#[test]
fn a_failing_patch_leaves_no_trace() {
    let before = compile(BEFORE);
    let mut tree = LiveTree::new();
    tree.mount(&before);
    let snapshot_before = tree.snapshot();

    // Op 0 succeeds, op 1 fails: the style must be back to #112233 and the
    // whole structural fingerprint must match.
    let patch = UiPatch::new(vec![
        PatchOp::SetStyle {
            key: key("heading"),
            props: floem_style::StyleProps {
                background: Some(Color::rgba(1, 2, 3, 255)),
                ..Default::default()
            },
        },
        PatchOp::Insert {
            key: key("stray"),
            parent: Some(key("nowhere")),
            index: 0,
            kind: floem_ui_ir::NodeKind::Text,
            props: vec![],
        },
    ]);
    let mut dirty = DirtySet::new();
    let error = tree.apply(&patch, &mut dirty).expect_err("rolled back");
    assert_eq!(error.failed_at, 1, "the first op applied cleanly");
    assert!(error.reason.contains("nowhere"), "{:?}", error.reason);

    assert_eq!(
        tree.snapshot(),
        snapshot_before,
        "a rolled-back patch must leave the tree byte-identical"
    );
    let heading = tree.style_of(&key("heading"), StyleState::NORMAL).unwrap();
    assert_eq!(
        heading.background,
        Some(Color::rgba(0x11, 0x22, 0x33, 0xff)),
        "even the successful op's effect is undone"
    );
}

#[test]
fn style_and_layout_changes_reach_the_dirty_set() {
    let (before, after) = (compile(BEFORE), compile(AFTER));
    let mut tree = LiveTree::new();
    tree.mount(&before);
    let root_id = tree.id_of(&key("root")).unwrap();
    let body_id = tree.id_of(&key("body")).unwrap();
    let heading_id = tree.id_of(&key("heading")).unwrap();

    let patch = floem_hot_reload::diff(&before, &after);
    let mut dirty = DirtySet::new();
    tree.apply(&patch, &mut dirty).expect("applies");

    let pending = dirty.drain_pending();
    assert!(pending.contains(&heading_id), "recoloured node is dirty");
    assert!(
        dirty
            .get(heading_id)
            .contains(DirtyFlags::STYLE | DirtyFlags::PAINT),
        "style changes mark style+paint: {:?}",
        dirty.get(heading_id)
    );
    assert!(
        dirty.get(body_id).contains(DirtyFlags::LAYOUT),
        "a layout change marks the node"
    );
    assert!(
        dirty.get(root_id).contains(DirtyFlags::LAYOUT),
        "and propagates to the parent"
    );
}

#[test]
fn removing_a_node_takes_its_subtree_with_it() {
    let before = compile(BEFORE);
    // `body` (and its child `ok`) is gone.
    let after = compile(
        r#"
root = stack axis=vertical gap=8 {
    heading = text label="hello" background=#112233
}
"#,
    );
    let mut tree = LiveTree::new();
    tree.mount(&before);
    let count_before = tree.element_count();
    let root_id = tree.id_of(&key("root")).unwrap();

    let patch = floem_hot_reload::diff(&before, &after);
    let mut dirty = DirtySet::new();
    let report = tree.apply(&patch, &mut dirty).expect("applies");

    assert_eq!(
        report.unmounted,
        vec![key("body")],
        "only the subtree root is named"
    );
    assert_eq!(
        tree.element_count(),
        count_before - 2,
        "body and its child are both gone"
    );
    assert!(tree.id_of(&key("body")).is_none());
    assert!(tree.id_of(&key("ok")).is_none());
    assert!(
        dirty.get(root_id).contains(DirtyFlags::CHILDREN),
        "the parent learns its child list changed"
    );
}
