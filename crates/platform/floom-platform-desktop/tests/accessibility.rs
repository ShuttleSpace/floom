// SPDX-License-Identifier: MIT
use std::cell::Cell;
use std::rc::Rc;

use floom_accessibility::{
    AccessibilityAdapter, AccessibilityCapabilities, SemanticsAction, SemanticsDiff, SemanticsNode,
    SemanticsRole,
};
use floom_platform_desktop::accessibility::DesktopAccessibilityAdapter;
use floom_types::ElementId;

fn id(n: u32) -> ElementId {
    ElementId::new(n, 1)
}

#[test]
fn applies_diff_into_retained_frame() {
    let mut adapter = DesktopAccessibilityAdapter::new();
    let button = id(1);

    let mut diff = SemanticsDiff::default();
    diff.added.push((
        button,
        SemanticsNode::new(SemanticsRole::Button).with_label("Save"),
    ));
    adapter.update(&diff);

    assert_eq!(adapter.node_count(), 1);
    assert_eq!(
        adapter.frame().get(button).map(|n| n.label.as_str()),
        Some("Save")
    );

    // The tree is retained and the actions are dispatchable, but no operating
    // system can read it yet: the adapter claims no native bridge until one
    // is installed.
    assert!(
        !adapter
            .capabilities()
            .contains(AccessibilityCapabilities::NATIVE_BRIDGE)
    );
    assert!(
        adapter
            .capabilities()
            .contains(AccessibilityCapabilities::SCREEN_READER)
    );

    // A caller that installs a bridge says so, and the claim follows.
    let bridged = DesktopAccessibilityAdapter::new().with_native_bridge();
    assert!(
        bridged
            .capabilities()
            .contains(AccessibilityCapabilities::NATIVE_BRIDGE)
    );
}

#[test]
fn performs_only_registered_actions() {
    let mut adapter = DesktopAccessibilityAdapter::new();
    let checkbox = id(2);

    let toggled = Rc::new(Cell::new(false));
    let flag = toggled.clone();
    adapter.register_handler(
        checkbox,
        Box::new(move |action| match action {
            SemanticsAction::Activate => {
                flag.set(true);
                true
            }
            SemanticsAction::SetValue(value) => {
                flag.set(value == "on");
                true
            }
            _ => false,
        }),
    );
    assert!(adapter.has_handler(checkbox));

    assert!(adapter.perform_action(checkbox, SemanticsAction::Activate));
    assert!(toggled.get());

    assert!(adapter.perform_action(checkbox, SemanticsAction::SetValue("off".to_string())));
    assert!(!toggled.get());

    assert!(!adapter.perform_action(checkbox, SemanticsAction::Increase));

    // An unregistered node has no handler.
    assert!(!adapter.perform_action(id(99), SemanticsAction::Activate));
}

#[test]
fn removal_drops_node_and_its_handler() {
    let mut adapter = DesktopAccessibilityAdapter::new();
    let node = id(3);

    adapter.register_handler(node, Box::new(|_| true));
    let mut diff = SemanticsDiff::default();
    diff.added
        .push((node, SemanticsNode::new(SemanticsRole::Button)));
    adapter.update(&diff);
    assert_eq!(adapter.node_count(), 1);

    let mut removal = SemanticsDiff::default();
    removal.removed.push(node);
    adapter.update(&removal);

    assert_eq!(adapter.node_count(), 0);
    assert!(!adapter.perform_action(node, SemanticsAction::Activate));

    adapter.reset();
    assert_eq!(adapter.frame().node_count(), 0);
}

fn root_and_button() -> (ElementId, ElementId) {
    (id(1), id(2))
}

fn published() -> DesktopAccessibilityAdapter {
    let (root, button) = root_and_button();
    let mut adapter = DesktopAccessibilityAdapter::new();
    let mut diff = SemanticsDiff::default();
    diff.added.push((
        root,
        SemanticsNode::new(SemanticsRole::Group)
            .with_bounds(floom_types::Rect {
                x0: 0.0,
                y0: 0.0,
                x1: 200.0,
                y1: 100.0,
            })
            .with_child(button),
    ));
    diff.added.push((
        button,
        SemanticsNode::new(SemanticsRole::Button)
            .with_label("Save")
            .with_bounds(floom_types::Rect {
                x0: 10.0,
                y0: 20.0,
                x1: 110.0,
                y1: 60.0,
            })
            .with_action(SemanticsAction::Activate),
    ));
    adapter.update(&diff);
    adapter
}

#[test]
fn the_frame_becomes_an_accesskit_tree() {
    let adapter = published();
    let (root, button) = root_and_button();
    let update = adapter.tree_update().expect("a published frame has a tree");

    let tree = update.tree.expect("the first update names the tree");
    assert_eq!(
        tree.root,
        floom_platform_desktop::accessibility::accesskit_node_id(root)
    );
    assert_eq!(tree.toolkit_name.as_deref(), Some("floom"));
    assert_eq!(update.nodes.len(), 2);

    let (_, button_node) = update
        .nodes
        .iter()
        .find(|(node, _)| *node == floom_platform_desktop::accessibility::accesskit_node_id(button))
        .expect("the button is in the update");
    assert_eq!(button_node.role(), accesskit::Role::Button);
    assert_eq!(button_node.label(), Some("Save"));
    assert!(button_node.supports_action(accesskit::Action::Click));
    let bounds = button_node.bounds().expect("published bounds travel");
    assert_eq!((bounds.x0, bounds.y1), (10.0, 60.0));
}

#[test]
fn focus_names_the_focused_element_and_defaults_to_the_root() {
    let (root, button) = root_and_button();
    let mut adapter = DesktopAccessibilityAdapter::new();
    let mut diff = SemanticsDiff::default();
    diff.added.push((
        root,
        SemanticsNode::new(SemanticsRole::Group).with_child(button),
    ));
    diff.added.push((
        button,
        SemanticsNode::new(SemanticsRole::Button)
            .with_label("Save")
            .with_state(floom_accessibility::SemanticsState::FOCUSED),
    ));
    adapter.update(&diff);

    let update = adapter.tree_update().expect("tree");
    assert_eq!(
        update.focus,
        floom_platform_desktop::accessibility::accesskit_node_id(button)
    );

    let plain = published().tree_update().expect("tree");
    assert_eq!(
        plain.focus,
        floom_platform_desktop::accessibility::accesskit_node_id(root),
        "with nothing focused the root has the focus, as accesskit requires"
    );
}

#[test]
fn nothing_published_is_no_update() {
    assert!(DesktopAccessibilityAdapter::new().tree_update().is_none());
}

#[test]
fn a_child_that_was_never_published_is_not_linked() {
    let mut adapter = DesktopAccessibilityAdapter::new();
    let mut diff = SemanticsDiff::default();
    diff.added.push((
        id(1),
        SemanticsNode::new(SemanticsRole::Group).with_child(id(9)),
    ));
    adapter.update(&diff);

    let update = adapter.tree_update().expect("tree");
    let (_, node) = update.nodes.first().expect("one node");
    assert_eq!(
        node.children().len(),
        0,
        "naming a child the update does not carry is a protocol violation"
    );
}
