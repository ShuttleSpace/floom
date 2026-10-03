//! Web accessibility adapter (capability-limited).
//!
//! The web backend can expose semantics through the DOM, so it advertises the
//! richest capability profile of the non-desktop backends. The DOM bridge itself
//! is not wired in this fork, so `NATIVE_BRIDGE` stays unset; the adapter only
//! retains the published tree.

use floom_accessibility::{AccessibilityCapabilities, CapabilityLimitedAccessibilityAdapter};

/// Capabilities the web backend reports once the DOM bridge lands.
pub fn accessibility_capabilities() -> AccessibilityCapabilities {
    AccessibilityCapabilities::SCREEN_READER
        | AccessibilityCapabilities::ACTION_INVOKE
        | AccessibilityCapabilities::VALUE_SET
        | AccessibilityCapabilities::LIVE_REGION
}

/// Build this backend's accessibility adapter.
pub fn web_accessibility() -> CapabilityLimitedAccessibilityAdapter {
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
        let adapter = web_accessibility();
        let caps = adapter.capabilities();
        assert!(caps.contains(AccessibilityCapabilities::LIVE_REGION));
        assert!(caps.contains(AccessibilityCapabilities::VALUE_SET));
        assert!(!caps.contains(AccessibilityCapabilities::NATIVE_BRIDGE));
    }

    #[test]
    fn retains_tree_but_cannot_invoke_without_bridge() {
        let mut adapter = web_accessibility();
        let node = ElementId::new(4, 1);
        let mut diff = SemanticsDiff::default();
        diff.added.push((
            node,
            SemanticsNode::new(SemanticsRole::Link).with_label("Docs"),
        ));

        adapter.update(&diff);
        assert_eq!(adapter.node_count(), 1);
        assert!(!adapter.perform_action(node, SemanticsAction::Activate));
    }
}
