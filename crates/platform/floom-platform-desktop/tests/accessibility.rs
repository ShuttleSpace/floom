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
