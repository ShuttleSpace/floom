//! The headless inspector's contract.
//!
//! The inspector navigates a live tree the way a devtools panel would: find a
//! node, walk to the root, read its resolved style and layout request, say
//! where it came from in the source, list its semantics, and answer "why did
//! this update?". Every answer is testable without a window.

use std::cell::RefCell;
use std::rc::Rc;

use floom_accessibility::{SemanticsNode, SemanticsRole, SemanticsTree};
use floom_devtools::Inspector;
use floom_hot_reload::{LiveTree, NodeKey};
use floom_inspector::JsonSnapshot;
use floom_legacy_reactive::{RwSignal, SignalUpdate};
use floom_runtime_core::{BindingRegistry, DirtyFlags, DirtySet, ReactiveBinding, SignalSource};
use floom_style::StyleState;
use floom_types::ElementId;

const DOC: &str = r#"
root = stack axis=vertical gap=8 {
    heading = text label="hello" background=#112233
    body = stack axis=horizontal gap=4 width=100 {
        ok = button label="OK"
    }
}
"#;

fn key(name: &str) -> NodeKey {
    NodeKey::new(name)
}

fn mounted() -> LiveTree {
    let document = floom_dsl::compile(DOC).expect("valid document").document;
    let mut tree = LiveTree::new();
    tree.mount(&document);
    tree
}

#[test]
fn navigation_finds_children_and_the_path_to_the_root() {
    let live = mounted();
    let inspector = Inspector::new(&live);

    let children = inspector.children(&key("root")).expect("root has children");
    let names: Vec<&str> = children.iter().map(|k| k.as_str()).collect();
    assert!(names.contains(&"heading"), "{names:?}");
    assert!(names.contains(&"body"), "{names:?}");
    // Document-order children: heading then body.
    assert_eq!(names.first(), Some(&"heading"));

    // Deeper: body's child is ok.
    let body_children = inspector.children(&key("body")).expect("body has children");
    assert_eq!(body_children, vec![key("ok")]);

    // Path to root from the deepest node.
    let path = inspector.path_to_root(&key("ok")).expect("ok is mounted");
    assert_eq!(
        path,
        vec![key("ok"), key("body"), key("root")],
        "the walk goes up one parent at a time"
    );
}

#[test]
fn a_missing_key_navigates_to_nothing() {
    let live = mounted();
    let inspector = Inspector::new(&live);
    assert!(inspector.children(&key("nope")).is_none());
    assert!(inspector.path_to_root(&key("nope")).is_none());
}

#[test]
fn the_property_inspector_resolves_style_and_layout() {
    let live = mounted();
    let inspector = Inspector::new(&live);

    // The heading's background at rest.
    let style = inspector
        .style_of(&key("heading"), StyleState::NORMAL)
        .expect("heading has a style");
    let background = style.background.expect("the DSL set a background");
    assert_eq!(
        (background.r, background.g, background.b),
        (0x11, 0x22, 0x33)
    );

    // The body's width came through the layout table.
    let layout = inspector
        .layout_of(&key("body"))
        .expect("body has a layout");
    assert_eq!(layout.width, Some(100.0));
    assert_eq!(layout.gap, Some(4.0));
    assert_eq!(layout.axis, Some(floom_core::Axis::Horizontal));
}

#[test]
fn kind_filters_find_every_element_of_a_kind() {
    let live = mounted();
    let inspector = Inspector::new(&live);
    let stacks = inspector.find_by_kind(floom_core::ElementKind::Host);
    let names: Vec<&str> = stacks.iter().map(|k| k.as_str()).collect();
    assert!(
        names.contains(&"root") && names.contains(&"body"),
        "both stacks are hosts: {names:?}"
    );
    let texts = inspector.find_by_kind(floom_core::ElementKind::Primitive);
    let names: Vec<&str> = texts.iter().map(|k| k.as_str()).collect();
    assert!(
        names.contains(&"heading") && names.contains(&"ok"),
        "text and button mount as primitives: {names:?}"
    );
}

#[test]
fn source_mapping_says_where_a_node_was_written() {
    let live = mounted();
    let mut inspector = Inspector::new(&live);
    assert!(
        inspector.source_of(&key("heading")).is_none(),
        "no source until the host provides one"
    );
    inspector.set_source(
        key("heading"),
        floom_inspector::SourceLocation {
            file: "ui/app.dsl".into(),
            line: 3,
            column: 5,
        },
    );
    let source = inspector.source_of(&key("heading")).expect("now mapped");
    assert_eq!(source.line, 3);
    assert_eq!(source.to_string(), "ui/app.dsl:3:5");
}

#[test]
fn the_semantics_viewer_lists_the_tree_the_host_holds() {
    let live = mounted();
    let mut inspector = Inspector::new(&live);
    assert!(
        inspector.semantics().is_empty(),
        "no semantics until the host provides them"
    );

    let ok_id = inspector.id_of(&key("ok")).expect("ok is mounted");
    let mut tree = SemanticsTree::new();
    tree.set_node(
        ok_id,
        SemanticsNode::new(SemanticsRole::Button).with_label("OK"),
    );
    inspector.set_semantics(tree);

    let entries = inspector.semantics();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].role, SemanticsRole::Button);
    assert_eq!(entries[0].label, "OK");
    assert_eq!(entries[0].element, ok_id);
}

#[test]
fn why_did_update_names_the_signal_and_the_ancestors() {
    // The registry and the inspector answer different halves of the question:
    // the registry knows which signal fired, the inspector knows which
    // ancestors the invalidation would propagate to.
    let live = mounted();
    let inspector = Inspector::new(&live);

    let dirty = Rc::new(RefCell::new(DirtySet::new()));
    // A real app registers bindings against the live tree's own arena (one
    // shared Rc). The live tree owns its arena, so the test rebuilds the
    // heading's exact id — same index, generation 1 — in a scratch arena. This
    // is the integration seam on purpose: the registry's report is keyed by
    // element id, the inspector's ancestor walk is keyed by the tree, and the
    // two meet at the id.
    let heading_id = inspector.id_of(&key("heading")).expect("mounted");
    let arena = Rc::new(RefCell::new(floom_runtime_core::ElementArena::new()));
    let el = loop {
        let id = arena.borrow_mut().insert_primitive();
        if id == heading_id {
            break id;
        }
    };
    let text = RwSignal::new("hello".to_string());
    let mut registry = BindingRegistry::new();
    registry.bind_signal_with_source(
        arena.clone(),
        dirty.clone(),
        ReactiveBinding {
            element: el,
            flags: DirtyFlags::PAINT,
        },
        text,
        SignalSource {
            signal: text.id(),
            label: "heading_text".into(),
            property: "label".into(),
        },
    );
    text.set("world".to_string());

    let path = inspector.why_did_update(&registry, heading_id);
    assert_eq!(path.bindings.len(), 1, "the heading has one binding");
    assert_eq!(
        path.bindings[0].source.as_ref().map(|s| s.label.as_str()),
        Some("heading_text"),
        "the report names the signal"
    );
    assert_eq!(path.bindings[0].runs, 2, "initial run + the update");
    // The heading's parent chain, root last: propagate_layout marks ancestors.
    assert_eq!(
        path.ancestors,
        vec![key("root")],
        "only the parent chain, not the whole tree"
    );
}

#[test]
fn the_tree_snapshot_serves_the_protocol() {
    // The wire format the devtools frontend consumes is the protocol's; the
    // inspector's job is to fill it from the live tree, ids and parents
    // included.
    let live = mounted();
    let inspector = Inspector::new(&live);
    let snapshot = inspector.tree_snapshot();
    snapshot
        .validate()
        .expect("the served snapshot is consistent");
    assert_eq!(snapshot.nodes.len(), 4);

    // The root is mounted from the document root, and every parent link is a
    // raw id the snapshot itself contains.
    let root = snapshot
        .nodes
        .iter()
        .find(|n| n.id == snapshot.root)
        .expect("root");
    assert_eq!(root.key.as_deref(), Some("root"));
    assert!(root.parent.is_none());
    assert_eq!(root.children.len(), 2);

    // Round-trip: a frontend can parse what the inspector serves.
    let json = snapshot.to_json().expect("serializes");
    let back = floom_inspector::TreeSnapshot::from_json(&json).expect("parses");
    assert_eq!(back, snapshot);
    let _ = ElementId::INVALID; // keep the import honest if ids change shape
}
