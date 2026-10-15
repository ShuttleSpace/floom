//! An absolutely positioned layer over its parent's box.
//!
//! The primitive the dialog, the popup and the bottom sheet are built from. It covers a region outside the flow and takes the click that dismisses it.
//!
//! It lived in `floom-widgets/src/compose.rs` beside the layout vocabulary
//! (`Stack`, `List`, `Grid`, `Conditional`, `Crossfade`). The two are not the
//! same kind of thing: a `Stack` arranges children along an axis, which is
//! arithmetic; this decides *what a control looks like*, which is a product
//! decision. So it moved up a layer with the tree, the counter and the select,
//! and the dependency runs one way -- this crate composes the engine's
//! vocabulary rather than the other way round.

use std::any::Any;
use std::sync::atomic::{Ordering};
use std::sync::{Arc};

use floom_accessibility::SemanticsRole;
use floom_core::{
    Child, ChildKey, CreateContext, ElementId, LayoutRequest,
    PaintContext, SemanticsContext, UpdateContext, Widget, WidgetAny,
};

use floom_widgets::primitives::{SemanticsHint};
use floom_widgets::{Grid};

use crate::overlay_bookkeeping::OverlayHost;

// ---------------------------------------------------------------------------
// Overlay
// ---------------------------------------------------------------------------

/// An absolutely positioned layer that covers its parent's whole box.
///
/// Use it for modals, drawers and toasts: the child escapes the parent's flow
/// and spans the containing block. Toggling the content mounts/unmounts just
/// the layer, leaving the rest of the tree untouched.
///
/// This is a *layout* primitive, not a host overlay: the layer is clipped by
/// whatever contains it. [`Dialog`], [`Popup`] and [`BottomSheet`] are the
/// composites that hand their surface to the host instead (and fall back to
/// building on this layer when the host has no overlays), which is the
/// difference between "inside this box" and "above everything".
#[derive(Clone)]
pub struct Overlay {
    present: bool,
    last_present: bool,
    child: Option<Child>,
}

impl std::fmt::Debug for Overlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Overlay")
            .field("present", &self.present)
            .finish()
    }
}

impl Overlay {
    pub fn new() -> Self {
        Self {
            present: false,
            last_present: false,
            child: None,
        }
    }

    /// Puts `child` on the layer and marks it present.
    pub fn content(mut self, child: Child) -> Self {
        self.child = Some(child);
        self.present = true;
        self
    }

    /// Puts `build` on the layer with an explicit key and layout.
    pub fn content_with(
        mut self,
        key: impl Into<ChildKey>,
        layout: LayoutRequest,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.child = Some(Child::new(key, layout, build));
        self.present = true;
        self
    }

    /// Replaces (or clears) the layer content; `None` dismisses it.
    pub fn set_content(&mut self, child: Option<Child>) {
        self.present = child.is_some();
        self.child = child;
    }

    /// Dismisses the layer.
    pub fn dismiss(&mut self) {
        self.child = None;
        self.present = false;
    }

    /// Whether the layer is up.
    pub fn is_present(&self) -> bool {
        self.present
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Default for Overlay {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Overlay {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        if self.present != self.last_present {
            cx.mark_children_dirty(id);
            self.last_present = self.present;
        }
    }

    fn children(&self) -> Vec<Child> {
        match &self.child {
            Some(child) if self.present => vec![child.clone()],
            _ => Vec::new(),
        }
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().absolute_fill())
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        floom_widgets::emit_semantics(self.semantics_hint(), cx);
    }
}
