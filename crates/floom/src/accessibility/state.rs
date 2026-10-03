//! Accessibility node states and the actions a node can perform.

use bitflags::bitflags;

bitflags! {
    /// Boolean states of an accessibility node.
    ///
    /// A subset of WAI-ARIA states (`aria-disabled`, `aria-selected`, ...) and
    /// the equivalent native attributes on each platform. Backends translate
    /// these into the platform-specific state representation.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct AccessState: u32 {
        /// The node currently has keyboard focus.
        const FOCUSED         = 1 << 0;
        /// The node is disabled and cannot be interacted with.
        const DISABLED        = 1 << 1;
        /// The node is not exposed to the accessibility tree.
        const HIDDEN          = 1 << 2;
        /// The node is selected (list item, tab, ...).
        const SELECTED        = 1 << 3;
        /// The node is checked (checkbox / radio / switch).
        const CHECKED         = 1 << 4;
        /// The node is expanded (collapsible container).
        const EXPANDED        = 1 << 5;
        /// The node is collapsed (collapsible container).
        const COLLAPSED       = 1 << 6;
        /// The node's value cannot be edited.
        const READ_ONLY       = 1 << 7;
        /// The node requires a value before it is valid.
        const REQUIRED        = 1 << 8;
        /// The node's value is invalid.
        const INVALID         = 1 << 9;
        /// The node (typically a link) has been visited.
        const VISITED         = 1 << 10;
        /// The node is a modal dialog blocking the rest of the UI.
        const MODAL           = 1 << 11;
        /// The node allows multiple selection.
        const MULTISELECTABLE = 1 << 12;
        /// The node's orientation is vertical.
        const VERTICAL        = 1 << 13;
        /// The node's orientation is horizontal.
        const HORIZONTAL      = 1 << 14;
        /// The node is the default control within its group.
        const DEFAULT         = 1 << 15;
    }
}

impl Default for AccessState {
    fn default() -> Self {
        AccessState::empty()
    }
}

/// An action a screen reader (or other assistive technology) can request on a
/// node.
///
/// The platform accessibility API delivers these to Floom; Floom performs them
/// via [`crate::accessibility::AccessibilityBackend::perform_action`] and
/// returns whether it handled the request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AccessAction {
    /// Move keyboard focus to the node.
    Focus,
    /// Activate the node's default action (e.g. click a button).
    Default,
    /// Increment a value (slider, spinbutton, stepper).
    Increment,
    /// Decrement a value.
    Decrement,
    /// Show the node's context menu.
    ShowContextMenu,
    /// Scroll the node into the visible viewport.
    ScrollIntoView,
    /// Expand a collapsible node.
    Expand,
    /// Collapse an expandable node.
    Collapse,
    /// Select the node.
    Select,
    /// Set the node's value (e.g. the text of a text field). The new value is
    /// passed as the action payload and interpreted by the node.
    SetValue,
}
