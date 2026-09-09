//! HarmonyOS accessibility adapter (capability-limited).
//!
//! HarmonyOS is an independent platform implementation, not an Android alias, so
//! it keeps its own capability profile. No native bridge is wired in this fork;
//! `NATIVE_BRIDGE` stays unset and the adapter only retains the tree.

use floem_accessibility::{AccessibilityCapabilities, CapabilityLimitedAccessibilityAdapter};

/// Capabilities HarmonyOS reports once a native bridge lands.
pub fn accessibility_capabilities() -> AccessibilityCapabilities {
    AccessibilityCapabilities::SCREEN_READER | AccessibilityCapabilities::ACTION_INVOKE
}

/// Build this backend's accessibility adapter.
pub fn harmony_accessibility() -> CapabilityLimitedAccessibilityAdapter {
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
        let adapter = harmony_accessibility();
        let caps = adapter.capabilities();
        assert!(caps.contains(AccessibilityCapabilities::SCREEN_READER));
        assert!(!caps.contains(AccessibilityCapabilities::NATIVE_BRIDGE));
    }

    #[test]
    fn independent_of_android_profile() {
        // Harmony is not an alias of Android: the adapters are separate values
        // built from this crate's own profile.
        let harmony = harmony_accessibility();
        let android = floem_accessibility::CapabilityLimitedAccessibilityAdapter::new(
            floem_accessibility::AccessibilityCapabilities::SCREEN_READER,
        );
        assert_ne!(harmony.capabilities(), android.capabilities());
    }

    #[test]
    fn retains_tree_but_cannot_invoke_without_bridge() {
        let mut adapter = harmony_accessibility();
        let node = ElementId::new(3, 1);
        let mut diff = SemanticsDiff::default();
        diff.added.push((
            node,
            SemanticsNode::new(SemanticsRole::Switch).with_label("Wi-Fi"),
        ));

        adapter.update(&diff);
        assert_eq!(adapter.node_count(), 1);
        assert!(!adapter.perform_action(node, SemanticsAction::Activate));
    }
}
