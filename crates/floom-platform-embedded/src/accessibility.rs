//! Embedded accessibility adapter (no accessibility).
//!
//! Headless/framebuffer targets have no assistive technology to talk to, so the
//! capability set is empty. The adapter still accepts updates so the same
//! publishing path runs everywhere and stays exercised by tests.

use floom_accessibility::{AccessibilityCapabilities, CapabilityLimitedAccessibilityAdapter};

/// Embedded targets expose no accessibility capabilities.
pub fn accessibility_capabilities() -> AccessibilityCapabilities {
    AccessibilityCapabilities::empty()
}

/// Build this backend's accessibility adapter.
pub fn embedded_accessibility() -> CapabilityLimitedAccessibilityAdapter {
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
    fn reports_no_capabilities() {
        let adapter = embedded_accessibility();
        assert!(adapter.capabilities().is_empty());
    }

    #[test]
    fn accepts_updates_without_exposing_them() {
        let mut adapter = embedded_accessibility();
        let node = ElementId::new(5, 1);
        let mut diff = SemanticsDiff::default();
        diff.added.push((
            node,
            SemanticsNode::new(SemanticsRole::StaticText).with_label("temp"),
        ));

        adapter.update(&diff);
        assert_eq!(adapter.node_count(), 1);
        assert!(!adapter.perform_action(node, SemanticsAction::Activate));

        adapter.reset();
        assert_eq!(adapter.node_count(), 0);
    }
}
