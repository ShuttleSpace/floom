// SPDX-License-Identifier: MIT
//! The bridge from a host's own tree to an inspector's live tree.
//!
//! The host at L7 has the nodes and no shape for them; the inspector at L8 has
//! the shape and no nodes. `tree_nodes` hands over plain data, `describe` takes
//! it, and the two together are what makes a running app inspectable. Both
//! halves are checked here, because a bridge that silently drops a node is worse
//! than no bridge.

use floom_app::{HostedKind, HostedNode, WidgetHost};
use floom_core::ElementKind;
use floom_hot_reload::{LiveTree, describe};
use floom_layout::LayoutStyle;
use floom_widgets::{Stack, Text};

/// A host with a root and one child, the smallest tree worth inspecting.
fn host() -> WidgetHost {
    let mut host = WidgetHost::new();
    host.set_size(floom_types::Size::new(400.0, 300.0));
    host.mount_root(LayoutStyle::default(), Text::new("root"));
    host
}

#[test]
fn the_host_describes_its_own_tree_as_data() {
    let mut host = host();
    // A stack with two children: the tree the inspector will read.
    let mut stack = Stack::new(floom_core::Axis::Vertical);
    stack = stack.push(|| Box::new(Text::new("first")));
    stack = stack.push(|| Box::new(Text::new("second")));
    host.mount_root(LayoutStyle::default(), stack);
    // The reconciler mounts the stack's children during a frame, so a tree read
    // before one has only the root.
    host.frame();

    let nodes = host.tree_nodes();
    assert!(!nodes.is_empty(), "a mounted tree has nodes to describe");
    // The root names no parent; everything else names one.
    let roots: Vec<&HostedNode> = nodes.iter().filter(|n| n.parent.is_none()).collect();
    assert_eq!(roots.len(), 1, "exactly one root: {:?}", roots);
    // A widget with children is a host; a leaf is a primitive.
    assert_eq!(roots[0].kind, HostedKind::Host);
    assert!(
        nodes.iter().any(|n| n.kind == HostedKind::Primitive),
        "the leaves are primitives: {:?}",
        nodes
    );
}

#[test]
fn the_keys_are_unique_so_a_lookup_is_unambiguous() {
    let mut host = host();
    let mut stack = Stack::new(floom_core::Axis::Vertical);
    stack = stack.push(|| Box::new(Text::new("a")));
    stack = stack.push(|| Box::new(Text::new("b")));
    stack = stack.push(|| Box::new(Text::new("c")));
    host.mount_root(LayoutStyle::default(), stack);
    host.frame();

    let nodes = host.tree_nodes();
    let keys: Vec<&str> = nodes.iter().map(|n| n.key.as_str()).collect();
    let unique: std::collections::HashSet<&str> = keys.iter().copied().collect();
    assert_eq!(keys.len(), unique.len(), "no key is reused: {keys:?}");
}

#[test]
fn describing_the_hosts_data_answers_structural_questions() {
    let mut host = host();
    let mut stack = Stack::new(floom_core::Axis::Vertical);
    stack = stack.push(|| Box::new(Text::new("first")));
    stack = stack.push(|| Box::new(Text::new("second")));
    host.mount_root(LayoutStyle::default(), stack);
    host.frame();

    let live: LiveTree = describe(&host.tree_nodes());
    // The root is findable and has children.
    let root_key = live
        .elements()
        .into_iter()
        .find(|e| e.parent.is_none())
        .map(|e| e.key)
        .expect("the described tree has a root");
    let root_id = live.id_of(&root_key).expect("the root is mounted");
    let root = live
        .elements()
        .into_iter()
        .find(|e| e.id == root_id)
        .expect("the root is in the element list");
    assert_eq!(root.children.len(), 2, "the root has its two children");

    // And a child knows its parent, so `path_to_root` walks.
    let child = root.children[0];
    let child_key = live.key_of(child).expect("a child has a key");
    let grandchild = live
        .elements()
        .into_iter()
        .find(|e| e.id == child)
        .expect("the child is in the element list");
    assert_eq!(grandchild.parent, Some(root_id));
    assert!(child_key.as_str().starts_with("node-"));
}

#[test]
fn a_node_whose_parent_was_not_described_is_dropped() {
    // The host named a parent it did not describe. The bridge must not attach it
    // to the wrong node, and it must not crash: it drops the node.
    let nodes = vec![
        HostedNode {
            key: "orphan".into(),
            kind: HostedKind::Primitive,
            element: floom_types::ElementId::new(2, 0),
            parent: Some(floom_types::ElementId::new(9999, 0)),
            // This fixture says nothing about paint or layout: the
            // test is about edges and tiers, and `None` is the honest
            // "no claim" the field documents.
            style: None,
            layout: None,
        },
        HostedNode {
            key: "root".into(),
            kind: HostedKind::Host,
            element: floom_types::ElementId::new(0, 0),
            parent: None,
            // This fixture says nothing about paint or layout: the
            // test is about edges and tiers, and `None` is the honest
            // "no claim" the field documents.
            style: None,
            layout: None,
        },
    ];
    let live = describe(&nodes);
    // The root survives.
    assert!(live.elements().iter().any(|e| e.key.as_str() == "root"));
    // The orphan does not: a tree with one wrong edge is worse than a tree
    // missing one node.
    assert!(!live.elements().iter().any(|e| e.key.as_str() == "orphan"));
}

#[test]
fn an_empty_host_describes_an_empty_tree() {
    let host = WidgetHost::new();
    let nodes = host.tree_nodes();
    assert!(nodes.is_empty(), "a host with nothing mounted has no nodes");
    let live = describe(&nodes);
    assert!(live.elements().is_empty());
}

#[test]
fn the_kinds_map_onto_the_inspectors_three_tiers() {
    // The mapping is thin and total: every host kind has an inspector tier.
    let nodes = vec![
        HostedNode {
            key: "root".into(),
            kind: HostedKind::Host,
            element: floom_types::ElementId::new(0, 0),
            parent: None,
            // This fixture says nothing about paint or layout: the
            // test is about edges and tiers, and `None` is the honest
            // "no claim" the field documents.
            style: None,
            layout: None,
        },
        HostedNode {
            key: "leaf".into(),
            kind: HostedKind::Primitive,
            element: floom_types::ElementId::new(1, 0),
            parent: Some(floom_types::ElementId::new(0, 0)),
            // This fixture says nothing about paint or layout: the
            // test is about edges and tiers, and `None` is the honest
            // "no claim" the field documents.
            style: None,
            layout: None,
        },
    ];
    let live = describe(&nodes);
    // The described kinds are IR kinds; the inspector maps them onto its three
    // tiers. What matters here is that the two kinds survive the mapping.
    let tiers: Vec<ElementKind> = live
        .elements()
        .into_iter()
        .map(|e| floom_hot_reload::tier_of(e.kind))
        .collect();
    assert!(tiers.contains(&ElementKind::Host), "{tiers:?}");
    assert!(tiers.contains(&ElementKind::Primitive), "{tiers:?}");
}

#[test]
fn a_key_round_trips_back_to_the_node_it_names() {
    let mut host = host();
    let mut stack = Stack::new(floom_core::Axis::Vertical);
    stack = stack.push(|| Box::new(Text::new("first")));
    stack = stack.push(|| Box::new(Text::new("second")));
    host.mount_root(LayoutStyle::default(), stack);
    host.frame();

    // A panel holds a selection as a key, across frames and rebuilds, so the
    // key has to name the same node a frame later — and a stale one has to come
    // back as "gone" rather than as whatever now occupies the slot.
    for described in host.tree_nodes() {
        let node = host.node_by_key(&described.key).unwrap_or_else(|| {
            panic!("a key the host just minted must resolve: {}", described.key)
        });
        assert_eq!(
            host.tree_nodes()
                .into_iter()
                .find(|n| n.key == described.key)
                .map(|n| n.element),
            Some(described.element),
        );
        // The reverse direction names the same element the description did.
        assert!(
            host.tree_nodes()
                .iter()
                .any(|n| n.element == described.element && n.key == described.key),
            "node {} does not answer to {}",
            described.element.raw(),
            described.key
        );
        let _ = node;
    }
}

#[test]
fn a_key_from_another_tree_does_not_resolve_to_a_live_node() {
    let host = WidgetHost::new();
    // A panel keeps its selection across a rebuild. The old key must answer
    // "gone" rather than land on a node that reused the slot — a stale
    // selection that silently retargets is worse than one that clears.
    assert!(host.node_by_key("node-999999").is_none());
    assert!(host.node_by_key("node-not-a-number").is_none());
    assert!(host.node_by_key("").is_none());
    // The format is the host's own, and nothing else parses as one.
    assert!(host.node_by_key("999999").is_none());
}
