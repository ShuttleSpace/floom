// SPDX-License-Identifier: MIT
use floom_accessibility::{
    AccessibilityAdapter, AccessibilityCapabilities, AccessibilityFrame,
    CapabilityLimitedAccessibilityAdapter, SemanticsAction, SemanticsContext, SemanticsNode,
    SemanticsRole, SemanticsState, SemanticsTree,
};
use floom_types::ElementId;

fn id(n: u32) -> ElementId {
    ElementId::new(n, 1)
}

#[test]
fn node_carries_role_label_value_hint_state_and_actions() {
    let node = SemanticsNode::new(SemanticsRole::CheckBox)
        .with_label("Accept terms")
        .with_value("on")
        .with_hint("Toggle to accept")
        .with_state(SemanticsState::CHECKED | SemanticsState::FOCUSABLE)
        .with_action(SemanticsAction::Activate);

    assert_eq!(node.role, SemanticsRole::CheckBox);
    assert_eq!(node.label, "Accept terms");
    assert_eq!(node.value, "on");
    assert_eq!(node.hint, "Toggle to accept");
    assert!(node.state.contains(SemanticsState::CHECKED));
    assert!(node.state.contains(SemanticsState::FOCUSABLE));
    assert_eq!(node.actions, vec![SemanticsAction::Activate]);
}

#[test]
fn default_node_is_not_worth_publishing() {
    let empty = SemanticsNode::default();
    assert!(empty.is_empty());
    assert_eq!(empty.role, SemanticsRole::None);

    let labelled = SemanticsNode::new(SemanticsRole::Button).with_label("Save");
    assert!(!labelled.is_empty());
}

#[test]
fn tree_reports_added_then_updated_then_removed() {
    let mut tree = SemanticsTree::new();
    let button = id(1);

    tree.set_node(
        button,
        SemanticsNode::new(SemanticsRole::Button).with_label("Save"),
    );
    let first = tree.collect_updates();
    assert_eq!(first.added.len(), 1);
    assert_eq!(first.added[0].0, button);
    assert!(first.updated.is_empty());
    assert!(first.removed.is_empty());

    // Same element, new semantics → update, not add.
    tree.set_node(
        button,
        SemanticsNode::new(SemanticsRole::Button).with_label("Save all"),
    );
    let second = tree.collect_updates();
    assert!(second.added.is_empty());
    assert_eq!(second.updated.len(), 1);
    assert_eq!(second.updated[0].1.label, "Save all");

    tree.remove(button);
    let third = tree.collect_updates();
    assert_eq!(third.removed, vec![button]);
    assert!(!tree.contains(button));
}

#[test]
fn runtime_semantics_dirty_routes_into_diff() {
    let mut tree = SemanticsTree::new();
    let field = id(7);
    tree.set_node(
        field,
        SemanticsNode::new(SemanticsRole::TextField).with_label("Name"),
    );
    let _ = tree.collect_updates();

    // The runtime flips `DirtyFlags::SEMANTICS` for the element; a11y layer
    // re-publishes it without any other tree mutation.
    assert!(!tree.has_pending_updates());
    tree.mark_semantics_dirty(field);
    assert!(tree.has_pending_updates());

    let diff = tree.collect_updates();
    assert_eq!(diff.updated.len(), 1);
    assert_eq!(diff.updated[0].0, field);
    assert!(
        diff.added.is_empty(),
        "existing node must not re-appear as added"
    );
}

#[test]
fn marking_unknown_element_dirty_is_a_noop() {
    let mut tree = SemanticsTree::new();
    tree.mark_semantics_dirty(id(99));
    assert!(!tree.has_pending_updates());
    assert!(tree.collect_updates().is_empty());
}

#[test]
fn collect_updates_drains_pending_state() {
    let mut tree = SemanticsTree::new();
    tree.set_node(id(1), SemanticsNode::new(SemanticsRole::StaticText));
    tree.set_node(id(2), SemanticsNode::new(SemanticsRole::StaticText));

    assert!(!tree.collect_updates().is_empty());
    assert!(
        tree.collect_updates().is_empty(),
        "second drain must be empty"
    );
    assert!(!tree.has_pending_updates());
}

#[test]
fn insert_then_remove_before_drain_reports_removal_only() {
    let mut tree = SemanticsTree::new();
    let temp = id(3);
    tree.set_node(temp, SemanticsNode::new(SemanticsRole::ListItem));
    tree.remove(temp);

    let diff = tree.collect_updates();
    assert_eq!(diff.removed, vec![temp]);
    assert!(diff.added.is_empty());
    assert!(diff.updated.is_empty());
    assert_eq!(tree.node_count(), 0);
    assert!(!tree.is_dirty(temp));
}

#[test]
fn context_builder_preserves_untouched_fields() {
    let mut tree = SemanticsTree::new();
    let slider = id(4);

    SemanticsContext::new(&mut tree, slider, SemanticsRole::Slider)
        .label("Volume")
        .add_action(SemanticsAction::Increase)
        .state(SemanticsState::FOCUSABLE)
        .commit();

    // A later update touches only the value; role/actions/state survive.
    SemanticsContext::new(&mut tree, slider, SemanticsRole::Slider)
        .value("40%")
        .commit();

    let node = tree.get(slider).expect("node retained");
    assert_eq!(node.role, SemanticsRole::Slider);
    assert_eq!(node.label, "Volume");
    assert_eq!(node.value, "40%");
    assert_eq!(node.actions, vec![SemanticsAction::Increase]);
    assert!(node.state.contains(SemanticsState::FOCUSABLE));
}

#[test]
fn diff_reports_changed_ids_in_order() {
    let mut tree = SemanticsTree::new();
    tree.set_node(id(1), SemanticsNode::new(SemanticsRole::ListItem));
    tree.set_node(id(2), SemanticsNode::new(SemanticsRole::ListItem));
    let mut diff = tree.collect_updates();

    diff.removed.push(id(9));
    assert_eq!(
        diff.changed_ids(),
        vec![diff.added[0].0, diff.added[1].0, id(9)]
    );
    assert!(!diff.is_empty());
}

#[test]
fn capability_limited_adapter_retains_frame_but_cannot_invoke() {
    let mut adapter = CapabilityLimitedAccessibilityAdapter::new(
        AccessibilityCapabilities::SCREEN_READER | AccessibilityCapabilities::ACTION_INVOKE,
    );
    let button = id(1);
    let mut tree = SemanticsTree::new();
    tree.set_node(
        button,
        SemanticsNode::new(SemanticsRole::Button).with_label("OK"),
    );

    adapter.update(&tree.collect_updates());
    assert_eq!(adapter.node_count(), 1);
    assert_eq!(
        adapter.frame().get(button).map(|n| n.label.as_str()),
        Some("OK")
    );

    // The OS supports actions, but this backend has no bridge wired.
    assert!(
        adapter
            .capabilities()
            .contains(AccessibilityCapabilities::ACTION_INVOKE)
    );
    assert!(
        !adapter
            .capabilities()
            .contains(AccessibilityCapabilities::NATIVE_BRIDGE)
    );
    assert!(!adapter.perform_action(button, SemanticsAction::Activate));

    tree.remove(button);
    adapter.update(&tree.collect_updates());
    assert_eq!(adapter.node_count(), 0);

    adapter.reset();
    assert_eq!(adapter.frame(), &AccessibilityFrame::default());
}
