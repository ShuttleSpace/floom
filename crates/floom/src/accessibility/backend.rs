//! Backend contract for accessibility.
//!
//! This mirrors the embedder crate's [`crate::embedder::EmbedderWindow`] /
//! [`crate::embedder::PlatformServices`] split: a Rust trait backends
//! implement, plus a [`MockAccessibility`] for tests and headless use.

use peniko::kurbo::Point;

use super::{AccessAction, AccessNodeId, AccessTree, AccessTreeUpdate};

/// A platform-requested accessibility action routed back to Floom core.
///
/// Produced by a backend's [`AccessibilityBackend::poll_actions`] when an
/// assistive technology (VoiceOver / Orca / Narrator) invokes an action on a
/// node. `WindowState` drains these each frame and dispatches them via
/// [`crate::window::WindowState::handle_accessibility_action`].
pub type PendingAction = (AccessNodeId, AccessAction, Option<String>);

/// A backend that bridges Floom's neutral accessibility tree to a platform
/// screen reader:
///
/// - macOS / iOS: `NSAccessibility` (`NSAccessibilityElement`)
/// - Android: `AccessibilityNodeInfo` / `AccessibilityNodeProvider`
/// - Linux: AT-SPI2 (over D-Bus)
/// - Windows: UI Automation (UIA)
///
/// Floom core produces [`AccessTreeUpdate`]s from view/layout changes and
/// calls [`update_tree`](AccessibilityBackend::update_tree). The platform
/// backend translates them into native a11y nodes and answers queries
/// ([`node_at_point`], [`focused_node`]) and routes user actions back via
/// [`perform_action`].
///
/// The default implementations are no-ops so backends that do not support
/// accessibility (or have not implemented it yet) compile and run safely.
pub trait AccessibilityBackend {
    /// Apply a tree diff produced by Floom core. Default: no-op.
    fn update_tree(&mut self, _update: AccessTreeUpdate) {}

    /// The currently focused node, if any. Default: `None`.
    fn focused_node(&self) -> Option<AccessNodeId> {
        None
    }

    /// Hit-test by window-local logical point; used for touch exploration.
    /// Default: `None`.
    fn node_at_point(&self, _point: Point) -> Option<AccessNodeId> {
        None
    }

    /// Perform an action requested by the platform accessibility API on a node.
    /// Returns `true` if Floom handled it. Default: `false`.
    fn perform_action(
        &mut self,
        _node: AccessNodeId,
        _action: AccessAction,
        _payload: Option<String>,
    ) -> bool {
        false
    }

    /// Return and clear any actions the platform accessibility API requested
    /// since the last call (e.g. a screen reader "press"). Default: empty.
    ///
    /// Backends that route platform → Floom actions override this to drain
    /// their action channel; `WindowState` calls it every frame.
    fn poll_actions(&mut self) -> Vec<PendingAction> {
        Vec::new()
    }

    /// Notify the platform adapter whether the window (and thus its a11y tree)
    /// currently has keyboard focus. Backends that model window focus at the
    /// adapter level (macOS `update_view_focus_state`, Windows
    /// `update_window_focus_state`) override this; the default is a no-op so
    /// platforms without that concept (Linux AT-SPI, mobile) stay correct.
    fn update_focus_state(&mut self, _focused: bool) {}
}

/// An in-memory accessibility backend used for tests and headless operation.
///
/// It keeps the latest [`AccessTree`] and records every
/// [`perform_action`](AccessibilityBackend::perform_action) call, so tests can
/// assert on the a11y contract without a real screen reader.
#[derive(Clone, Debug, Default)]
pub struct MockAccessibility {
    tree: AccessTree,
    performed: Vec<(AccessNodeId, AccessAction, Option<String>)>,
}

impl MockAccessibility {
    /// A fresh mock backend with an empty tree.
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of the current tree.
    pub fn tree(&self) -> &AccessTree {
        &self.tree
    }

    /// All actions requested since construction, in order.
    pub fn performed_actions(&self) -> &[(AccessNodeId, AccessAction, Option<String>)] {
        &self.performed
    }

    /// Clear the recorded action log.
    pub fn clear_actions(&mut self) {
        self.performed.clear();
    }
}

impl AccessibilityBackend for MockAccessibility {
    fn update_tree(&mut self, update: AccessTreeUpdate) {
        self.tree.apply(update);
    }

    fn focused_node(&self) -> Option<AccessNodeId> {
        self.tree.focus
    }

    fn node_at_point(&self, point: Point) -> Option<AccessNodeId> {
        self.tree.node_at_point(point)
    }

    fn perform_action(
        &mut self,
        node: AccessNodeId,
        action: AccessAction,
        payload: Option<String>,
    ) -> bool {
        self.performed.push((node, action, payload.clone()));
        // Accept only actions the node advertises as supported.
        self.tree
            .get(node)
            .map(|n| n.supports(action))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accessibility::{AccessNode, AccessNodeId, AccessRole, AccessTreeUpdate};

    #[test]
    fn mock_applies_tree_and_reports_focus() {
        let mut mock = MockAccessibility::new();
        mock.update_tree(
            AccessTreeUpdate::new()
                .with_root(AccessNodeId(1))
                .with_nodes([
                    AccessNode::new(AccessNodeId(1), AccessRole::Window)
                        .with_children([AccessNodeId(2)]),
                    AccessNode::new(AccessNodeId(2), AccessRole::Button).with_label("OK"),
                ])
                .with_focus(AccessNodeId(2)),
        );
        assert_eq!(mock.focused_node(), Some(AccessNodeId(2)));
        assert_eq!(mock.tree().len(), 2);
    }

    #[test]
    fn mock_perform_action_records_and_validates() {
        let mut mock = MockAccessibility::new();
        mock.update_tree(AccessTreeUpdate::new().with_nodes([AccessNode::new(
            AccessNodeId(2),
            AccessRole::Button,
        )
        .with_actions([AccessAction::Default, AccessAction::Focus])]));

        // Supported action is accepted and recorded.
        assert!(mock.perform_action(AccessNodeId(2), AccessAction::Default, None));
        // Unsupported action is rejected but still recorded.
        assert!(!mock.perform_action(AccessNodeId(2), AccessAction::Increment, None));
        // Action on a missing node is rejected and recorded.
        assert!(!mock.perform_action(AccessNodeId(99), AccessAction::Focus, None));

        let log = mock.performed_actions();
        assert_eq!(log.len(), 3);
        assert_eq!(log[0], (AccessNodeId(2), AccessAction::Default, None));
        assert_eq!(log[1], (AccessNodeId(2), AccessAction::Increment, None));
        assert_eq!(log[2].0, AccessNodeId(99));

        mock.clear_actions();
        assert!(mock.performed_actions().is_empty());
    }

    #[test]
    fn mock_node_at_point_delegates_to_tree() {
        let mut mock = MockAccessibility::new();
        mock.update_tree(AccessTreeUpdate::new().with_nodes([AccessNode::new(
            AccessNodeId(2),
            AccessRole::Button,
        )
        .with_bounds(peniko::kurbo::Rect::new(0.0, 0.0, 50.0, 50.0))]));
        assert_eq!(
            mock.node_at_point(peniko::kurbo::Point::new(10.0, 10.0)),
            Some(AccessNodeId(2))
        );
        assert_eq!(
            mock.node_at_point(peniko::kurbo::Point::new(100.0, 100.0)),
            None
        );
    }
}
