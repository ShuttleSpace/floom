//! The accessibility tree and diffable updates.

use std::collections::HashMap;

use peniko::kurbo::{Point, Rect};

use super::{AccessNode, AccessNodeId};

/// A full or partial update to the accessibility tree.
///
/// Floom core produces these from view/layout changes and hands them to the
/// backend via [`crate::accessibility::AccessibilityBackend::update_tree`].
///
/// - `nodes` are **upserts**: insert a new node or replace an existing one by
///   [`id`](AccessNode::id).
/// - `removed` deletes nodes (and cleans up dangling child references).
/// - `root` / `focus` set the tree root and current focus when `Some`.
#[derive(Clone, Debug, Default)]
pub struct AccessTreeUpdate {
    /// The tree root, if this update (re)establishes it.
    pub root: Option<AccessNodeId>,
    /// Nodes to insert or replace, keyed by id.
    pub nodes: Vec<AccessNode>,
    /// Node ids to remove.
    pub removed: Vec<AccessNodeId>,
    /// The focused node, if this update moves focus.
    pub focus: Option<AccessNodeId>,
}

impl AccessTreeUpdate {
    /// An empty update.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the tree root.
    pub fn with_root(mut self, root: AccessNodeId) -> Self {
        self.root = Some(root);
        self
    }

    /// Add nodes to upsert.
    pub fn with_nodes(mut self, nodes: impl IntoIterator<Item = AccessNode>) -> Self {
        self.nodes.extend(nodes);
        self
    }

    /// Add node ids to remove.
    pub fn with_removed(mut self, removed: impl IntoIterator<Item = AccessNodeId>) -> Self {
        self.removed.extend(removed);
        self
    }

    /// Set the focused node.
    pub fn with_focus(mut self, focus: AccessNodeId) -> Self {
        self.focus = Some(focus);
        self
    }

    /// True when this update carries no changes.
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
            && self.nodes.is_empty()
            && self.removed.is_empty()
            && self.focus.is_none()
    }
}

/// An in-memory accessibility tree.
///
/// This is the neutral store backends keep in sync; it also powers the
/// headless / [`crate::accessibility::MockAccessibility`] backend and the unit
/// tests. Platform backends translate it into native a11y nodes eagerly or on
/// update, depending on the platform API.
#[derive(Clone, Debug, Default)]
pub struct AccessTree {
    /// The tree root, if established.
    pub root: Option<AccessNodeId>,
    /// The currently focused node, if any.
    pub focus: Option<AccessNodeId>,
    nodes: HashMap<AccessNodeId, AccessNode>,
}

impl AccessTree {
    /// An empty tree.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a node by id.
    pub fn get(&self, id: AccessNodeId) -> Option<&AccessNode> {
        self.nodes.get(&id)
    }

    /// Whether a node id is present.
    pub fn contains(&self, id: AccessNodeId) -> bool {
        self.nodes.contains_key(&id)
    }

    /// Iterate over all nodes.
    pub fn iter(&self) -> impl Iterator<Item = &AccessNode> {
        self.nodes.values()
    }

    /// Number of nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the tree has no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The focused node, if any.
    pub fn focused(&self) -> Option<&AccessNode> {
        self.focus.and_then(|id| self.nodes.get(&id))
    }

    /// Apply a diffable update, keeping parent/child references consistent.
    ///
    /// Removal also prunes dangling child references from surviving parents
    /// and clears `root` / `focus` if they pointed at a removed node.
    pub fn apply(&mut self, update: AccessTreeUpdate) {
        if let Some(root) = update.root {
            self.root = Some(root);
        }
        for node in update.nodes {
            self.nodes.insert(node.id, node);
        }
        for id in &update.removed {
            self.nodes.remove(id);
            for parent in self.nodes.values_mut() {
                if parent.children.iter().any(|c| c == id) {
                    parent.children.retain(|c| c != id);
                }
            }
            if self.root == Some(*id) {
                self.root = None;
            }
            if self.focus == Some(*id) {
                self.focus = None;
            }
        }
        if let Some(focus) = update.focus {
            self.focus = Some(focus);
        }
    }

    /// Hit-test: return the **innermost** (smallest-area) node whose bounds
    /// contain `point`, or `None`. Used for touch exploration and
    /// screen-reader focus-by-coordinate.
    pub fn node_at_point(&self, point: Point) -> Option<AccessNodeId> {
        let mut best: Option<(AccessNodeId, f64)> = None;
        for node in self.nodes.values() {
            if let Some(rect) = node.bounds {
                if rect_contains(rect, point) {
                    let area = rect.width() * rect.height();
                    match best {
                        Some((_, best_area)) if best_area <= area => {}
                        _ => best = Some((node.id, area)),
                    }
                }
            }
        }
        best.map(|(id, _)| id)
    }
}

/// Inclusive bounds check for a logical-space rectangle.
fn rect_contains(rect: Rect, point: Point) -> bool {
    point.x >= rect.x0 && point.x <= rect.x1 && point.y >= rect.y0 && point.y <= rect.y1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accessibility::{AccessAction, AccessRole, AccessState};

    fn node(id: u64, role: AccessRole) -> AccessNode {
        AccessNode::new(AccessNodeId(id), role)
    }

    #[test]
    fn apply_inserts_nodes_and_root() {
        let mut tree = AccessTree::new();
        tree.apply(
            AccessTreeUpdate::new()
                .with_root(AccessNodeId(1))
                .with_nodes([
                    node(1, AccessRole::Window).with_children([AccessNodeId(2), AccessNodeId(3)]),
                    node(2, AccessRole::Button).with_label("OK"),
                    node(3, AccessRole::Button).with_label("Cancel"),
                ]),
        );
        assert_eq!(tree.root, Some(AccessNodeId(1)));
        assert_eq!(tree.len(), 3);
        assert!(tree.contains(AccessNodeId(2)));
        let root = tree.get(AccessNodeId(1)).unwrap();
        assert_eq!(root.children, vec![AccessNodeId(2), AccessNodeId(3)]);
    }

    #[test]
    fn apply_updates_existing_node() {
        let mut tree = AccessTree::new();
        tree.apply(AccessTreeUpdate::new().with_nodes([node(2, AccessRole::Button).with_label("OK")]));
        tree.apply(
            AccessTreeUpdate::new()
                .with_nodes([node(2, AccessRole::Button).with_label("Done").with_states(AccessState::DISABLED)]),
        );
        let n = tree.get(AccessNodeId(2)).unwrap();
        assert_eq!(n.label.as_deref(), Some("Done"));
        assert!(n.states.contains(AccessState::DISABLED));
    }

    #[test]
    fn apply_removes_node_and_prunes_children() {
        let mut tree = AccessTree::new();
        tree.apply(
            AccessTreeUpdate::new()
                .with_root(AccessNodeId(1))
                .with_nodes([
                    node(1, AccessRole::Window).with_children([AccessNodeId(2)]),
                    node(2, AccessRole::Button),
                ]),
        );
        tree.apply(AccessTreeUpdate::new().with_removed([AccessNodeId(2)]));
        assert!(!tree.contains(AccessNodeId(2)));
        assert!(tree.get(AccessNodeId(1)).unwrap().children.is_empty());
    }

    #[test]
    fn apply_removing_focus_clears_focus() {
        let mut tree = AccessTree::new();
        tree.apply(
            AccessTreeUpdate::new()
                .with_nodes([node(2, AccessRole::Button)])
                .with_focus(AccessNodeId(2)),
        );
        assert_eq!(tree.focus, Some(AccessNodeId(2)));
        tree.apply(AccessTreeUpdate::new().with_removed([AccessNodeId(2)]));
        assert_eq!(tree.focus, None);
        assert_eq!(tree.focused(), None);
    }

    #[test]
    fn node_at_point_picks_innermost() {
        let mut tree = AccessTree::new();
        tree.apply(AccessTreeUpdate::new().with_nodes([
            node(1, AccessRole::Window).with_bounds(Rect::new(0.0, 0.0, 100.0, 100.0)),
            node(2, AccessRole::Button).with_bounds(Rect::new(10.0, 10.0, 40.0, 40.0)),
        ]));
        // A point inside both should resolve to the smaller (inner) node.
        assert_eq!(tree.node_at_point(Point::new(20.0, 20.0)), Some(AccessNodeId(2)));
        // A point outside the button but inside the window resolves to the window.
        assert_eq!(tree.node_at_point(Point::new(80.0, 80.0)), Some(AccessNodeId(1)));
        // A point outside everything resolves to nothing.
        assert_eq!(tree.node_at_point(Point::new(200.0, 200.0)), None);
    }

    #[test]
    fn empty_update_is_empty() {
        assert!(AccessTreeUpdate::new().is_empty());
        assert!(!AccessTreeUpdate::new().with_focus(AccessNodeId(5)).is_empty());
    }

    #[test]
    fn node_supports_action() {
        let n = node(2, AccessRole::Button)
            .with_actions([AccessAction::Default, AccessAction::Focus]);
        assert!(n.supports(AccessAction::Default));
        assert!(!n.supports(AccessAction::Increment));
    }
}
