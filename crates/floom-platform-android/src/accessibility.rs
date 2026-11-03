//! Android accessibility adapter (capability-limited).
//!
//! Android's accessibility service can consume a semantics tree, but this fork
//! has no native bridge wired yet, so `NATIVE_BRIDGE` stays unset and
//! `perform_action` cannot deliver to the OS. The adapter still retains the
//! published tree so tooling and tests can inspect it.

use floom_accessibility::{AccessibilityCapabilities, CapabilityLimitedAccessibilityAdapter};

/// Capabilities Android reports once a native bridge lands.
pub fn accessibility_capabilities() -> AccessibilityCapabilities {
    AccessibilityCapabilities::SCREEN_READER | AccessibilityCapabilities::ACTION_INVOKE
}

/// Build this backend's accessibility adapter.
pub fn android_accessibility() -> CapabilityLimitedAccessibilityAdapter {
    CapabilityLimitedAccessibilityAdapter::new(accessibility_capabilities())
}

#[cfg(test)]
mod tests {
    use super::*;
    use floom_accessibility::{
        AccessibilityAdapter, SemanticsAction, SemanticsDiff, SemanticsNode, SemanticsRole,
    };
    use floom_types::ElementId;

    #[test]
    fn reports_capability_profile() {
        let adapter = android_accessibility();
        let caps = adapter.capabilities();
        assert!(caps.contains(AccessibilityCapabilities::SCREEN_READER));
        assert!(caps.contains(AccessibilityCapabilities::ACTION_INVOKE));
        assert!(!caps.contains(AccessibilityCapabilities::NATIVE_BRIDGE));
    }

    #[test]
    fn retains_tree_but_cannot_invoke_without_bridge() {
        let mut adapter = android_accessibility();
        let node = ElementId::new(1, 1);
        let mut diff = SemanticsDiff::default();
        diff.added.push((
            node,
            SemanticsNode::new(SemanticsRole::Button).with_label("OK"),
        ));

        adapter.update(&diff);
        assert_eq!(adapter.node_count(), 1);
        assert_eq!(
            adapter.frame().get(node).map(|n| n.label.as_str()),
            Some("OK")
        );

        // No native bridge: the action never reaches the OS.
        assert!(!adapter.perform_action(node, SemanticsAction::Activate));
    }
}
