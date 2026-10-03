//! Linux AT-SPI2 accessibility backend (via `accesskit_unix`).
//!
//! Bridges Floom's neutral [`AccessTreeUpdate`] to AT-SPI2 over D-Bus. We feed
//! the tree to `accesskit_unix::Adapter`, which owns the `org.a11y.atspi`
//! object server and the session-bus registration. The initial tree is served
//! lazily via the [`accesskit::ActivationHandler`] from a shared neutral-tree
//! cache; subsequent updates are pushed through [`Adapter::update_if_active`].
//!
//! Actions requested by Orca arrive via the [`accesskit::ActionHandler`] and
//! are forwarded to Floom core through a channel (drained by `WindowState`).

use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use accesskit::{ActionData, ActionHandler, ActionRequest, NodeId, TreeUpdate};
use accesskit_unix::Adapter;

use crate::accessibility::translate::build_tree_update;
use crate::accessibility::{
    AccessAction, AccessNodeId, AccessTree, AccessTreeUpdate, AccessibilityBackend, PendingAction,
};

/// Shared neutral-tree cache, readable by the activation handler (which runs
/// on accesskit's bus thread) and writable by the backend (main thread).
type SharedTree = Arc<Mutex<AccessTree>>;

/// Serves the initial tree when a screen reader first connects.
struct LinuxActivationHandler {
    tree: SharedTree,
}

impl accesskit::ActivationHandler for LinuxActivationHandler {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        let tree = self.tree.lock().ok()?;
        Some(build_tree_update(&tree))
    }
}

/// Routes Orca action requests into the action channel.
struct LinuxActionHandler {
    tx: mpsc::Sender<PendingAction>,
}

impl ActionHandler for LinuxActionHandler {
    fn do_action(&mut self, request: ActionRequest) {
        let node = AccessNodeId(request.target_node.0);
        let action = match request.action {
            accesskit::Action::Focus => AccessAction::Focus,
            accesskit::Action::Click => AccessAction::Default,
            accesskit::Action::Increment => AccessAction::Increment,
            accesskit::Action::Decrement => AccessAction::Decrement,
            accesskit::Action::ShowContextMenu => AccessAction::ShowContextMenu,
            accesskit::Action::ScrollIntoView => AccessAction::ScrollIntoView,
            accesskit::Action::Expand => AccessAction::Expand,
            accesskit::Action::Collapse => AccessAction::Collapse,
            accesskit::Action::SetValue => AccessAction::SetValue,
            _ => return,
        };
        let payload = match request.data {
            Some(ActionData::Value(v)) => Some(v.into()),
            _ => None,
        };
        let _ = self.tx.send((node, action, payload));
    }
}

/// No-op deactivation handler.
struct LinuxDeactivationHandler;

impl accesskit::DeactivationHandler for LinuxDeactivationHandler {
    fn deactivate(&mut self) {}
}

/// Bridges Floom's neutral accessibility tree to Linux AT-SPI2 via accesskit.
pub(crate) struct LinuxAccessibilityBackend {
    adapter: Adapter,
    tree: SharedTree,
    action_rx: mpsc::Receiver<PendingAction>,
}

impl LinuxAccessibilityBackend {
    /// Build the AT-SPI2 adapter. Returns `None` only on allocation failure
    /// (the adapter owns the D-Bus connection itself).
    pub(crate) fn new() -> Option<Self> {
        let tree: SharedTree = Arc::new(Mutex::new(AccessTree::new()));
        let (tx, rx) = mpsc::channel();
        let adapter = Adapter::new(
            LinuxActivationHandler { tree: tree.clone() },
            LinuxActionHandler { tx },
            LinuxDeactivationHandler,
        );
        Some(Self {
            adapter,
            tree,
            action_rx: rx,
        })
    }

    fn push(&mut self) {
        let tree = match self.tree.lock() {
            Ok(tree) => tree,
            Err(_) => return,
        };
        let update = build_tree_update(&tree);
        self.adapter.update_if_active(move || update);
    }
}

impl AccessibilityBackend for LinuxAccessibilityBackend {
    fn update_tree(&mut self, update: AccessTreeUpdate) {
        if let Ok(mut tree) = self.tree.lock() {
            tree.apply(update);
        }
        self.push();
    }

    fn focused_node(&self) -> Option<AccessNodeId> {
        self.tree.lock().ok().and_then(|t| t.focus)
    }

    fn node_at_point(&self, point: peniko::kurbo::Point) -> Option<AccessNodeId> {
        self.tree.lock().ok().and_then(|t| t.node_at_point(point))
    }

    fn poll_actions(&mut self) -> Vec<PendingAction> {
        self.action_rx.try_recv().into_iter().collect()
    }
}
