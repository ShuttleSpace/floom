//! Desktop accessibility adapter.
//!
//! The desktop backend retains the published semantics tree and can dispatch
//! accessibility actions to handlers registered by the app/widget layer.
//! Desktop is the reference target, so it advertises the full capability set
//! including a native bridge; the concrete OS bridge (AccessKit and friends) is
//! attached by the app layer above this crate.

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
                | AccessibilityCapabilities::LIVE_REGION
                | AccessibilityCapabilities::NATIVE_BRIDGE,
            frame: AccessibilityFrame::default(),
            handlers: HashMap::new(),
        }
    }
}

impl DesktopAccessibilityAdapter {
    pub fn new() -> Self {
        Self::default()
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
