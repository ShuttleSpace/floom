// SPDX-License-Identifier: MIT
//! Desktop accessibility adapter.
//!
//! The desktop backend retains the published semantics tree and can dispatch
//! accessibility actions to handlers registered by the app/widget layer.
//!
//! [`DesktopAccessibilityAdapter::tree_update`] turns the retained frame into
//! the `accesskit::TreeUpdate` an OS adapter consumes; that part is pure and
//! tested here. Installing an OS adapter still needs a window handle, so the
//! adapter does not claim [`AccessibilityCapabilities::NATIVE_BRIDGE`] on its
//! own — the caller that installs the bridge passes it to
//! [`DesktopAccessibilityAdapter::with_native_bridge`], and the claim follows
//! the bridge's presence so the two cannot drift apart.

use std::collections::HashMap;

use accesskit::{Action, Node, NodeId, Role, Tree, TreeId, TreeUpdate};
use floom_accessibility::{
    AccessibilityAdapter, AccessibilityCapabilities, AccessibilityFrame, SemanticsAction,
    SemanticsDiff, SemanticsRole, SemanticsState,
};
use floom_types::ElementId;

/// Full desktop a11y adapter: retained frame plus per-node action handlers.
pub struct DesktopAccessibilityAdapter {
    caps: AccessibilityCapabilities,
    frame: AccessibilityFrame,
    handlers: HashMap<ElementId, Box<dyn Fn(SemanticsAction) -> bool>>,
}

impl Default for DesktopAccessibilityAdapter {
    fn default() -> Self {
        Self {
            caps: AccessibilityCapabilities::SCREEN_READER
                | AccessibilityCapabilities::ACTION_INVOKE
                | AccessibilityCapabilities::VALUE_SET
                | AccessibilityCapabilities::SCROLL
                | AccessibilityCapabilities::LIVE_REGION,
            frame: AccessibilityFrame::default(),
            handlers: HashMap::new(),
        }
    }
}

impl DesktopAccessibilityAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    /// An adapter whose semantics tree reaches the operating system.
    ///
    /// The caller passes the bridge it installed, and the adapter claims
    /// [`AccessibilityCapabilities::NATIVE_BRIDGE`] because of it. Deriving
    /// the claim from the bridge's presence is the arrangement in which the
    /// two cannot drift: the default adapter has no bridge and does not claim
    /// one.
    pub fn with_native_bridge(mut self) -> Self {
        self.caps |= AccessibilityCapabilities::NATIVE_BRIDGE;
        self
    }

    pub fn frame(&self) -> &AccessibilityFrame {
        &self.frame
    }

    pub fn node_count(&self) -> usize {
        self.frame.node_count()
    }

    /// Register the handler that receives accessibility actions for `node`.
    ///
    /// A node without a handler reports `false` from
    /// [`AccessibilityAdapter::perform_action`], letting the caller fall back to
    /// synthesized input.
    pub fn register_handler(
        &mut self,
        node: ElementId,
        handler: Box<dyn Fn(SemanticsAction) -> bool>,
    ) {
        self.handlers.insert(node, handler);
    }

    pub fn has_handler(&self, node: ElementId) -> bool {
        self.handlers.contains_key(&node)
    }

    /// The node an `accesskit` update starts from.
    ///
    /// The frame's own root when the tree knows it; otherwise the node nothing
    /// claims as a child — the diffs the runtime hands over carry nodes rather
    /// than a root, and an update that names no root is not a tree.
    fn root(&self) -> Option<ElementId> {
        if let Some(root) = self.frame.root {
            return Some(root);
        }
        let claimed: std::collections::HashSet<ElementId> = self
            .frame
            .nodes
            .values()
            .flat_map(|node| node.children.iter().copied())
            .filter(|child| self.frame.nodes.contains_key(child))
            .collect();
        self.frame
            .nodes
            .keys()
            .filter(|id| !claimed.contains(id))
            .min_by_key(|id| id.raw())
            .copied()
    }

    /// The frame as an `accesskit` tree update, or `None` when nothing is published.
    ///
    /// Children come from each node's `children`, so a node whose children were
    /// never published is not linked — an `accesskit` update that names a child
    /// it does not carry is a protocol violation, not a partial tree.
    pub fn tree_update(&self) -> Option<TreeUpdate> {
        let root = self.root()?;
        let mut nodes = Vec::with_capacity(self.frame.nodes.len());
        let mut focus = node_id(root);
        for (id, published) in &self.frame.nodes {
            let mut node = Node::new(accesskit_role(published.role));
            if !published.label.is_empty() {
                node.set_label(published.label.as_str());
            }
            if !published.value.is_empty() {
                node.set_value(published.value.as_str());
            }
            if !published.hint.is_empty() {
                node.set_description(published.hint.as_str());
            }
            node.set_bounds(accesskit::Rect {
                x0: published.bounds.x0,
                y0: published.bounds.y0,
                x1: published.bounds.x1,
                y1: published.bounds.y1,
            });
            if published.state.contains(SemanticsState::DISABLED) {
                node.set_disabled();
            }
            if published
                .state
                .intersects(SemanticsState::SELECTED | SemanticsState::CHECKED)
            {
                node.set_selected(true);
            }
            if published.state.contains(SemanticsState::FOCUSED) {
                focus = node_id(*id);
            }
            if published.actions.contains(&SemanticsAction::Activate) {
                node.add_action(Action::Click);
            }
            if published
                .actions
                .iter()
                .any(|action| matches!(action, SemanticsAction::SetValue(_)))
            {
                node.add_action(Action::SetValue);
            }
            node.set_children(
                published
                    .children
                    .iter()
                    .filter(|child| self.frame.nodes.contains_key(child))
                    .map(|child| node_id(*child))
                    .collect::<Vec<_>>(),
            );
            nodes.push((node_id(*id), node));
        }
        Some(TreeUpdate {
            nodes,
            tree: Some(Tree {
                root: node_id(root),
                toolkit_name: Some("floom".to_string()),
                toolkit_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            tree_id: TreeId::ROOT,
            focus,
        })
    }
}

/// The `accesskit` role for a semantics role.
///
/// Roles without a closer match fall back to `GenericContainer` rather than
/// being dropped: a node the screen reader cannot place is worse than one it
/// names imprecisely.
pub fn accesskit_role(role: SemanticsRole) -> Role {
    use floom_accessibility::SemanticsRole as S;
    match role {
        S::None => Role::GenericContainer,
        S::Button => Role::Button,
        S::CheckBox => Role::CheckBox,
        S::Radio => Role::RadioButton,
        S::Switch => Role::Switch,
        S::Slider => Role::Slider,
        S::TextField => Role::TextInput,
        S::TextView => Role::MultilineTextInput,
        S::Image => Role::Image,
        S::ImageButton => Role::Button,
        S::Link => Role::Link,
        S::Heading => Role::Heading,
        S::StaticText => Role::Label,
        S::List => Role::List,
        S::ListItem => Role::ListItem,
        S::Group => Role::Group,
        S::Scrollable => Role::ScrollView,
        S::ProgressBar => Role::ProgressIndicator,
        S::Tab => Role::Tab,
        S::TabBar => Role::TabList,
        S::Dialog => Role::Dialog,
        S::MenuItem => Role::MenuItem,
        S::Custom(_) => Role::GenericContainer,
    }
}

fn node_id(id: ElementId) -> NodeId {
    NodeId(id.raw())
}

/// The `NodeId` an `accesskit` platform adapter would see for `element`.
///
/// Exposed so the caller that installs a bridge can map the OS's node ids back
/// to elements when it forwards actions.
pub fn accesskit_node_id(element: ElementId) -> NodeId {
    node_id(element)
}

impl AccessibilityAdapter for DesktopAccessibilityAdapter {
    fn update(&mut self, diff: &SemanticsDiff) {
        for (id, node) in &diff.added {
            self.frame.nodes.insert(*id, node.clone());
        }
        for (id, node) in &diff.updated {
            self.frame.nodes.insert(*id, node.clone());
        }
        for id in &diff.removed {
            self.frame.nodes.remove(id);
            self.handlers.remove(id);
        }
    }

    fn capabilities(&self) -> AccessibilityCapabilities {
        self.caps
    }

    fn perform_action(&mut self, node: ElementId, action: SemanticsAction) -> bool {
        match self.handlers.get(&node) {
            Some(handler) => handler(action),
            None => false,
        }
    }

    fn reset(&mut self) {
        self.frame = AccessibilityFrame::default();
        self.handlers.clear();
    }
}
