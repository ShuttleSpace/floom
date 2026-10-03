//! Translation from Floom's neutral accessibility model to `accesskit`.
//!
//! This module is the single bridge between [`crate::accessibility`]'s
//! platform-neutral [`AccessNode`]/[`AccessTree`] protocol and `accesskit`'s
//! tree types. `accesskit` then drives the native screen reader on each
//! platform (AT-SPI2 on Linux, UI Automation on Windows, NSAccessibility on
//! macOS). Keeping the mapping here means the per-OS backend modules stay tiny
//! and the mapping is unit-testable on any host (it is platform-agnostic).

use accesskit::{Action, Node, NodeId, Rect, Role, Tree, TreeId, TreeUpdate, Toggled};

use super::{AccessAction, AccessNode, AccessNodeId, AccessRole, AccessState, AccessTree};

/// Map a neutral role to the nearest `accesskit` role.
pub(crate) fn role_to_accesskit(role: AccessRole) -> Role {
    use AccessRole::*;
    match role {
        Unknown | None => Role::Unknown,
        Window => Role::Window,
        Application => Role::Application,
        Dialog => Role::Dialog,
        Group => Role::Group,
        Panel => Role::Pane,
        Divider => Role::GenericContainer,
        Text => Role::Label,
        Heading => Role::Heading,
        Paragraph => Role::Paragraph,
        Link => Role::Link,
        Image => Role::Image,
        Button => Role::Button,
        CheckBox => Role::CheckBox,
        RadioButton => Role::RadioButton,
        Switch => Role::Switch,
        Slider => Role::Slider,
        SpinButton => Role::SpinButton,
        ProgressBar => Role::ProgressIndicator,
        TextField => Role::TextInput,
        SearchField => Role::SearchInput,
        List => Role::List,
        ListItem => Role::ListItem,
        Grid => Role::Grid,
        Cell => Role::GridCell,
        Tab => Role::Tab,
        TabList => Role::TabList,
        TabPanel => Role::TabPanel,
        Menu => Role::Menu,
        MenuBar => Role::MenuBar,
        MenuItem => Role::MenuItem,
        ScrollView => Role::ScrollView,
        ScrollBar => Role::ScrollBar,
        Alert => Role::Alert,
        Status => Role::Status,
        Tooltip => Role::Tooltip,
    }
}

/// Map a neutral action to the equivalent `accesskit` action, or `None` when
/// `accesskit` has no corresponding verb (e.g. `Select` has no AT-SPI/UIA
/// action and is expressed via the `selected` state instead).
pub(crate) fn action_to_accesskit(action: AccessAction) -> Option<Action> {
    use AccessAction::*;
    Some(match action {
        Focus => Action::Focus,
        Default => Action::Click,
        Increment => Action::Increment,
        Decrement => Action::Decrement,
        ShowContextMenu => Action::ShowContextMenu,
        ScrollIntoView => Action::ScrollIntoView,
        Expand => Action::Expand,
        Collapse => Action::Collapse,
        SetValue => Action::SetValue,
        // `Select` is not an action in AT-SPI2/UIA; selection is a state.
        Select => return None,
    })
}

/// Build a single `accesskit` node from a neutral node.
pub(crate) fn build_accesskit_node(node: &AccessNode) -> Node {
    let mut a11y = Node::new(role_to_accesskit(node.role));

    if let Some(label) = &node.label {
        a11y.set_label(label.clone());
    }
    if let Some(value) = &node.value {
        a11y.set_value(value.clone());
    }
    if let Some(description) = &node.description {
        a11y.set_description(description.clone());
    }

    a11y.set_children(
        node.children
            .iter()
            .map(|c| NodeId::from(c.0))
            .collect::<Vec<_>>(),
    );

    let s = node.states;
    if s.contains(AccessState::DISABLED) {
        a11y.set_disabled();
    }
    if s.contains(AccessState::HIDDEN) {
        a11y.set_hidden();
    }
    if s.contains(AccessState::SELECTED) {
        a11y.set_selected(true);
    }
    if s.contains(AccessState::EXPANDED) {
        a11y.set_expanded(true);
    } else if s.contains(AccessState::COLLAPSED) {
        a11y.set_expanded(false);
    }
    if s.contains(AccessState::MULTISELECTABLE) {
        a11y.set_multiselectable();
    }
    if s.contains(AccessState::CHECKED) {
        a11y.set_toggled(Toggled::True);
    }

    for action in &node.actions {
        if let Some(a) = action_to_accesskit(*action) {
            a11y.add_action(a);
        }
    }

    if let Some(bounds) = node.bounds {
        a11y.set_bounds(Rect {
            x0: bounds.x0,
            y0: bounds.y0,
            x1: bounds.x1,
            y1: bounds.y1,
        });
    }

    a11y
}

/// Build a full `accesskit::TreeUpdate` from the accumulated neutral tree.
///
/// `accesskit` applies the update incrementally, but we always emit the full
/// tree so the call sites do not need to track which nodes changed. The `tree`
/// field (root + toolkit metadata) is required on the initial update and is
/// harmless to repeat.
pub(crate) fn build_tree_update(tree: &AccessTree) -> TreeUpdate {
    let root = tree.root.unwrap_or(AccessNodeId::ROOT);
    let focus = tree.focus.unwrap_or(root);

    let nodes = tree
        .iter()
        .map(|node| (NodeId::from(node.id.0), build_accesskit_node(node)))
        .collect();

    TreeUpdate {
        nodes,
        tree: Some(Tree::new(NodeId::from(root.0))),
        tree_id: TreeId::ROOT,
        focus: NodeId::from(focus.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accessibility::{AccessNode, AccessState, AccessTreeUpdate};

    #[test]
    fn role_roundtrip_known() {
        // A handful of representative mappings.
        assert_eq!(role_to_accesskit(AccessRole::Button), Role::Button);
        assert_eq!(role_to_accesskit(AccessRole::Window), Role::Window);
        assert_eq!(role_to_accesskit(AccessRole::Unknown), Role::Unknown);
        assert_eq!(role_to_accesskit(AccessRole::None), Role::Unknown);
        assert_eq!(role_to_accesskit(AccessRole::Panel), Role::Pane);
        assert_eq!(role_to_accesskit(AccessRole::Divider), Role::GenericContainer);
        assert_eq!(role_to_accesskit(AccessRole::ProgressBar), Role::ProgressIndicator);
    }

    #[test]
    fn action_select_has_no_accesskit_equivalent() {
        assert_eq!(action_to_accesskit(AccessAction::Select), None);
        assert_eq!(action_to_accesskit(AccessAction::Default), Some(Action::Click));
        assert_eq!(action_to_accesskit(AccessAction::Focus), Some(Action::Focus));
        assert_eq!(
            action_to_accesskit(AccessAction::Increment),
            Some(Action::Increment)
        );
    }

    #[test]
    fn build_node_maps_states_and_bounds() {
        let node = AccessNode::new(AccessNodeId(7), AccessRole::Button)
            .with_label("OK")
            .with_states(AccessState::DISABLED | AccessState::FOCUSED)
            .with_actions([AccessAction::Default, AccessAction::Focus])
            .with_bounds(peniko::kurbo::Rect::new(1.0, 2.0, 30.0, 40.0));
        // `FOCUSED` is not a per-node state in accesskit (it is the tree-level
        // `focus`); `DISABLED` is. Building must not panic and the tree update
        // must carry the node + root + focus.
        let mut tree = AccessTree::new();
        tree.apply(
            AccessTreeUpdate::new()
                .with_root(AccessNodeId(7))
                .with_nodes([node]),
        );
        let update = build_tree_update(&tree);
        assert_eq!(update.focus, NodeId::from(7));
        assert_eq!(update.tree_id, TreeId::ROOT);
        assert!(update.nodes.iter().any(|(id, _)| *id == NodeId::from(7)));
        assert!(update.tree.is_some());
    }
}
