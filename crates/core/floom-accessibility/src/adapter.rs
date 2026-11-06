// SPDX-License-Identifier: MIT
//! Accessibility adapter ABI: what a platform backend must implement to expose
//! the semantics tree to an OS assistive-technology API.

use std::collections::HashMap;

use bitflags::bitflags;
use floom_types::ElementId;

use crate::semantics::{SemanticsAction, SemanticsDiff, SemanticsNode};

bitflags! {
    /// Accessibility features a backend or its OS integration can provide.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct AccessibilityCapabilities: u32 {
        /// A screen reader can consume the published semantics tree.
        const SCREEN_READER = 1 << 0;
        /// Actions such as [`SemanticsAction::Activate`] can be delivered.
        const ACTION_INVOKE = 1 << 1;
        /// [`SemanticsAction::SetValue`] is supported.
        const VALUE_SET     = 1 << 2;
        /// [`SemanticsAction::ScrollIntoView`] is supported.
        const SCROLL        = 1 << 3;
        /// Live-region announcements are supported.
        const LIVE_REGION   = 1 << 4;
        /// A native OS a11y bridge is actually wired for this backend.
        ///
        /// Backends that only retain the tree (headless/CI stubs) leave this
        /// unset: they accept updates but cannot reach the OS.
        const NATIVE_BRIDGE = 1 << 5;
    }
}

/// Snapshot of the semantics tree as currently known to an adapter.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AccessibilityFrame {
    pub nodes: HashMap<ElementId, SemanticsNode>,
    pub root: Option<ElementId>,
}

impl AccessibilityFrame {
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn get(&self, id: ElementId) -> Option<&SemanticsNode> {
        self.nodes.get(&id)
    }
}

/// Translates semantics diffs into a concrete assistive-technology API.
pub trait AccessibilityAdapter {
    /// Apply the diff produced by [`SemanticsTree::collect_updates`](crate::SemanticsTree::collect_updates).
    fn update(&mut self, diff: &SemanticsDiff);
    fn capabilities(&self) -> AccessibilityCapabilities;
    /// Request an action on a node.
    ///
    /// Returns `true` only when the action reached a handler: a backend without
    /// a native bridge, or a node without a handler, reports `false` so callers
    /// can fall back to synthesized input.
    fn perform_action(&mut self, node: ElementId, action: SemanticsAction) -> bool;
    /// Drop all retained state (used when a window/tree is torn down).
    fn reset(&mut self);
}

/// Adapter for backends that have no native a11y bridge yet.
///
/// It still retains the published tree so tooling and tests can inspect it, but
/// [`AccessibilityAdapter::perform_action`] always reports `false` because no
/// OS delivery path exists. `NATIVE_BRIDGE` is intentionally left to the caller.
#[derive(Debug, Clone)]
pub struct CapabilityLimitedAccessibilityAdapter {
    caps: AccessibilityCapabilities,
    frame: AccessibilityFrame,
}

impl CapabilityLimitedAccessibilityAdapter {
    pub fn new(caps: AccessibilityCapabilities) -> Self {
        Self {
            caps,
            frame: AccessibilityFrame::default(),
        }
    }

    pub fn frame(&self) -> &AccessibilityFrame {
        &self.frame
    }

    pub fn node_count(&self) -> usize {
        self.frame.node_count()
    }
}

impl AccessibilityAdapter for CapabilityLimitedAccessibilityAdapter {
    fn update(&mut self, diff: &SemanticsDiff) {
        for (id, node) in &diff.added {
            self.frame.nodes.insert(*id, node.clone());
        }
        for (id, node) in &diff.updated {
            self.frame.nodes.insert(*id, node.clone());
        }
        for id in &diff.removed {
            self.frame.nodes.remove(id);
        }
    }

    fn capabilities(&self) -> AccessibilityCapabilities {
        self.caps
    }

    fn perform_action(&mut self, _node: ElementId, _action: SemanticsAction) -> bool {
        // No native bridge is wired for this backend; the action cannot be
        // delivered to the OS.
        false
    }

    fn reset(&mut self) {
        self.frame = AccessibilityFrame::default();
    }
}
