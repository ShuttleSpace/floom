//! Desktop accessibility adapter.
//!
//! The desktop backend retains the published semantics tree and can dispatch
//! accessibility actions to handlers registered by the app/widget layer.
//!
//! It does **not** advertise a native bridge, because there is none: the
//! concrete OS bridge (AccessKit and friends) is attached by the app layer
//! above this crate, and until one is, claiming that a screen reader can
//! read this tree is a claim nothing backs. The bridge's presence is what
//! sets the flag — see [`DesktopAccessibilityAdapter::with_native_bridge`] —
//! so the two cannot drift apart.

use std::collections::HashMap;

use floom_accessibility::{
    AccessibilityAdapter, AccessibilityCapabilities, AccessibilityFrame, SemanticsAction,
    SemanticsDiff,
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
