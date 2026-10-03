//! The neutral accessibility node model.

use peniko::kurbo::Rect;

use super::{AccessAction, AccessRole, AccessState};

/// Protocol-level identifier for an accessibility node.
///
/// Backends assign these when they build their native a11y tree. They must be
/// stable across [`crate::accessibility::AccessTreeUpdate`]s for the same
/// logical UI element so the platform tree can be patched incrementally.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AccessNodeId(pub u64);

impl AccessNodeId {
    /// Conventional id for the tree root. Core may use any stable id, but `0`
    /// is the recommended root when one is not otherwise assigned.
    pub const ROOT: AccessNodeId = AccessNodeId(0);
}

/// A single node in the accessibility tree.
///
/// This is the platform-neutral representation Floem core produces from its
/// view/layout state. Each backend maps it to its native a11y node type
/// (`NSAccessibilityElement`, `AccessibilityNodeInfo`, an AT-SPI accessible, a
/// UIA provider element, ...).
#[derive(Clone, Debug, PartialEq)]
pub struct AccessNode {
    /// Stable node id.
    pub id: AccessNodeId,
    /// What kind of thing this node is.
    pub role: AccessRole,
    /// Accessible name (label). For editable text this is usually empty and
    /// [`value`](AccessNode::value) carries the content instead.
    pub label: Option<String>,
    /// The current value (text content, slider value as a string, ...).
    pub value: Option<String>,
    /// A longer description beyond the label.
    pub description: Option<String>,
    /// Boolean states (focused, disabled, selected, ...).
    pub states: AccessState,
    /// Actions this node can respond to. Drives which verbs the screen reader
    /// offers the user.
    pub actions: Vec<AccessAction>,
    /// Bounding box in **logical pixels** relative to the window, if known.
    pub bounds: Option<Rect>,
    /// Child node ids, in paint / z-order.
    pub children: Vec<AccessNodeId>,
}

impl AccessNode {
    /// Create a node with the given id and role; everything else is empty.
    pub fn new(id: AccessNodeId, role: AccessRole) -> Self {
        Self {
            id,
            role,
            label: None,
            value: None,
            description: None,
            states: AccessState::empty(),
            actions: Vec::new(),
            bounds: None,
            children: Vec::new(),
        }
    }

    /// Builder: set the accessible name.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Builder: set the value.
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Builder: set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Builder: set the states.
    pub fn with_states(mut self, states: AccessState) -> Self {
        self.states = states;
        self
    }

    /// Builder: set the supported actions.
    pub fn with_actions(mut self, actions: impl IntoIterator<Item = AccessAction>) -> Self {
        self.actions = actions.into_iter().collect();
        self
    }

    /// Builder: set the bounding box.
    pub fn with_bounds(mut self, bounds: Rect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    /// Builder: set the children.
    pub fn with_children(mut self, children: impl IntoIterator<Item = AccessNodeId>) -> Self {
        self.children = children.into_iter().collect();
        self
    }

    /// Whether `action` is advertised as supported by this node.
    pub fn supports(&self, action: AccessAction) -> bool {
        self.actions.contains(&action)
    }
}
