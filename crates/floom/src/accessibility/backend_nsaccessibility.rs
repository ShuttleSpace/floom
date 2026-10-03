//! macOS accessibility backend (via `accesskit_macos`).
//!
//! Bridges Floom's neutral [`AccessTreeUpdate`] to VoiceOver / the macOS
//! Accessibility API. Rather than speaking `NSAccessibility` directly, we feed
//! the tree to `accesskit_macos::Adapter`, which owns the `NSView` subclassing
//! and `NSAccessibility` plumbing. Actions requested by VoiceOver arrive via
//! the [`ActionHandler`] we install and are forwarded to Floom core through a
//! channel (drained by `WindowState`).
//!
//! Wiring happens in `WindowHandle::new` (desktop only). The backend keeps the
//! neutral [`AccessTree`] as a cache so `focused_node` / `node_at_point` work
//! without re-querying the native tree.

use std::sync::mpsc;

use accesskit::{ActionData, ActionHandler, ActionRequest};
use accesskit_macos::Adapter;

use crate::accessibility::translate::build_tree_update;
use crate::accessibility::{
    AccessAction, AccessNodeId, AccessTree, AccessTreeUpdate, AccessibilityBackend, PendingAction,
};

/// Forwards VoiceOver action requests into a channel the `WindowState` drains.
struct MacOSActionHandler {
    tx: mpsc::Sender<PendingAction>,
}

impl ActionHandler for MacOSActionHandler {
    fn do_action(&mut self, request: ActionRequest) {
        let node = AccessNodeId(request.target_node.0);
        // Reverse-map the accesskit action to our neutral action.
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

/// Bridges Floom's neutral accessibility tree to macOS VoiceOver via accesskit.
pub(crate) struct MacOSAccessibilityBackend {
    adapter: Adapter,
    /// Neutral cache powering `focused_node` / `node_at_point`.
    tree: AccessTree,
    /// Receiver side of the action channel (see [`MacOSActionHandler`]).
    action_rx: mpsc::Receiver<PendingAction>,
}

impl MacOSAccessibilityBackend {
    /// Build the backend from the raw content-view pointer.
    ///
    /// `ns_view` must be a valid, live `NSView`. Returns `None` when it is null
    /// (e.g. headless) — no bridge is then active.
    ///
    /// # Safety
    /// Mirrors `accesskit_macos::Adapter::new`: must be called on the main
    /// thread with a valid `NSView`.
    pub(crate) fn new(ns_view: *mut std::ffi::c_void) -> Option<Self> {
        if ns_view.is_null() {
            return None;
        }
        let (tx, rx) = mpsc::channel();
        // SAFETY: caller guarantees `ns_view` is a valid, live `NSView` on the
        // main thread (we are invoked from `WindowHandle::new`).
        let adapter = unsafe { Adapter::new(ns_view, true, MacOSActionHandler { tx }) };
        Some(Self {
            adapter,
            tree: AccessTree::new(),
            action_rx: rx,
        })
    }

    /// Translate the cached neutral tree to an `accesskit` update and push it.
    fn push(&mut self) {
        let update = build_tree_update(&self.tree);
        if let Some(events) = self.adapter.update_if_active(move || update) {
            events.raise();
        }
    }
}

impl AccessibilityBackend for MacOSAccessibilityBackend {
    fn update_tree(&mut self, update: AccessTreeUpdate) {
        self.tree.apply(update);
        self.push();
    }

    fn focused_node(&self) -> Option<AccessNodeId> {
        self.tree.focus
    }

    fn node_at_point(&self, point: peniko::kurbo::Point) -> Option<AccessNodeId> {
        self.tree.node_at_point(point)
    }

    fn poll_actions(&mut self) -> Vec<PendingAction> {
        self.action_rx.try_recv().into_iter().collect()
    }

    fn update_focus_state(&mut self, focused: bool) {
        // Keep VoiceOver's notion of window focus in sync with Floom's. The
        // adapter was created with `is_view_focused = true`; this corrects it
        // whenever the window gains or loses keyboard focus.
        if let Some(events) = self.adapter.update_view_focus_state(focused) {
            events.raise();
        }
    }
}
