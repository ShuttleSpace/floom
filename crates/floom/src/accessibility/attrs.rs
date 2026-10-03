//! Per-view accessibility attribute overrides.
//!
//! These are the *declared* attributes a view author sets through the
//! [`crate::ViewId`] builder API. Floom core reads them at layout-commit time
//! (see [`crate::accessibility::bridge`]) and folds them into the neutral
//! [`crate::accessibility::AccessNode`] produced for that view.
//!
//! Everything is additive: a view with no overrides still appears in the tree
//! with [`AccessRole::Unknown`]. Only [`AccessibilityAttrs::hidden`] removes
//! the view (and re-parents its children), mirroring ARIA
//! `role="presentation"` / `role="none"`.

use std::sync::Arc;

use crate::accessibility::{AccessAction, AccessRole};

/// A handler invoked when a screen reader requests a semantic action on a view.
///
/// Floom routes platform accessibility actions (e.g. a VoiceOver/ORCA/Narrator
/// "increment" on a slider) back to the originating view through this hook.
/// Controls that own a value (slider, spinbutton) or a disclosure state (tree
/// row) register one via [`crate::ViewId::accessibility_action`] to make
/// themselves operable from a screen reader.
///
/// The closure receives the requested [`AccessAction`] and, for
/// [`AccessAction::SetValue`], the new value as a string payload (`None`
/// otherwise). It is stored in an `Arc` so the view's [`crate::accessibility::AccessibilityAttrs`]
/// stays cheaply cloneable and the handler survives tree diffs.
pub type AccessibilityActionHandler = Arc<dyn Fn(AccessAction, Option<String>) + 'static>;

/// Accessibility attributes a view author can declare.
///
/// Stored on [`crate::view::ViewState`]. `None` there means "use defaults":
/// the node is still emitted, just with an unknown role and no explicit
/// label/value/description.
#[derive(Default)]
pub struct AccessibilityAttrs {
    /// The semantic role of this view. Defaults to [`AccessRole::Unknown`].
    pub role: AccessRole,
    /// The accessible name (label). When `None`, backends may synthesize one
    /// from text content or fall back to nothing.
    pub label: Option<String>,
    /// The current value (text content, slider value as a string, ...).
    pub value: Option<String>,
    /// A longer description beyond the label.
    pub description: Option<String>,
    /// The actions this node can respond to.
    pub actions: Vec<AccessAction>,
    /// When `true`, the view is decorative: it is excluded from the tree and
    /// its children are re-parented to the nearest exposed ancestor.
    pub hidden: bool,
    /// Optional semantic-action handler (see [`AccessibilityActionHandler`]).
    /// When set, Floom routes semantic [`AccessAction`]s (Increment,
    /// Decrement, Expand, Collapse, SetValue, ScrollIntoView) to this closure
    /// instead of ignoring them. Generic actions (Focus, Default,
    /// ShowContextMenu) are routed through Floom's normal focus/event paths
    /// and do not need this hook.
    pub on_action: Option<AccessibilityActionHandler>,
}

impl Clone for AccessibilityAttrs {
    fn clone(&self) -> Self {
        Self {
            role: self.role,
            label: self.label.clone(),
            value: self.value.clone(),
            description: self.description.clone(),
            actions: self.actions.clone(),
            hidden: self.hidden,
            on_action: self.on_action.clone(),
        }
    }
}

impl std::fmt::Debug for AccessibilityAttrs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccessibilityAttrs")
            .field("role", &self.role)
            .field("label", &self.label)
            .field("value", &self.value)
            .field("description", &self.description)
            .field("actions", &self.actions)
            .field("hidden", &self.hidden)
            .field(
                "on_action",
                &self.on_action.as_ref().map(|_| "<handler>"),
            )
            .finish()
    }
}
