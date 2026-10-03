//! iOS accessibility adapter (capability-limited).
//!
//! UIKit exposes a full accessibility API, but this fork has no native bridge
//! wired yet, so `NATIVE_BRIDGE` stays unset and `perform_action` cannot deliver
//! to the OS. The adapter still retains the published tree.

use floem_accessibility::{AccessibilityCapabilities, CapabilityLimitedAccessibilityAdapter};

/// Capabilities iOS reports once a native bridge lands.
pub fn accessibility_capabilities() -> AccessibilityCapabilities {
    AccessibilityCapabilities::SCREEN_READER
        | AccessibilityCapabilities::ACTION_INVOKE
        | AccessibilityCapabilities::VALUE_SET
}

/// Build this backend's accessibility adapter.
pub fn ios_accessibility() -> CapabilityLimitedAccessibilityAdapter {
    CapabilityLimitedAccessibilityAdapter::new(accessibility_capabilities())
}

#[cfg(test)]
mod tests {
    use super::*;
    use floem_accessibility::{
        AccessibilityAdapter, SemanticsAction, SemanticsDiff, SemanticsNode, SemanticsRole,
    };
    use floem_types::ElementId;

    #[test]
    fn reports_capability_profile() {
        let adapter = ios_accessibility();
        let caps = adapter.capabilities();
        assert!(caps.contains(AccessibilityCapabilities::SCREEN_READER));
        assert!(caps.contains(AccessibilityCapabilities::VALUE_SET));
        assert!(!caps.contains(AccessibilityCapabilities::NATIVE_BRIDGE));
    }

    #[test]
    fn retains_tree_but_cannot_invoke_without_bridge() {
        let mut adapter = ios_accessibility();
        let node = ElementId::new(2, 1);
        let mut diff = SemanticsDiff::default();
        diff.added.push((
            node,
            SemanticsNode::new(SemanticsRole::TextField).with_label("Name"),
        ));

        adapter.update(&diff);
        assert_eq!(adapter.node_count(), 1);

        // No native bridge: the action never reaches the OS.
        assert!(!adapter.perform_action(node, SemanticsAction::SetValue("x".into())));
    }
}
