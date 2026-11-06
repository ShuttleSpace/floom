use std::any::Any;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use floom_accessibility::SemanticsRole;
use floom_core::{
    Child, ChildKey, ElementId, LayoutRequest, OverlayHandle, OverlayRequest, Rect, UpdateContext,
};

use crate::overlay::Overlay;
use floom_widgets::primitives::Text;
use floom_widgets::Grid;

// Overlay-backed surface plumbing

/// Bookkeeping shared by the composites whose surface belongs *above* the tree.
///
/// These widgets are driven by state — an app flips `present` — so there is no
/// event to react to and the request has to be made from [`Widget::update`].
/// Two outcomes are possible. The host mounts the surface on its overlay stack,
/// which is the normal case and the only one that escapes the parent's clip. Or
/// the host has no overlay support and refuses, in which case the composite
/// falls back to mounting the same surface in-tree — what a headless embedder
/// gets, and what these widgets did before overlays existed.
///
/// ## Why this is public, and why a composite holds it as a field
///
/// It was `pub(crate)` while only [`Popup`] and [`Dialog`] used it, which was
/// correct then: those two *are* the composite, so the bookkeeping lived inside
/// them. It is public now because a composite which is *about* something else —
/// a combobox with a dropdown, a select with a list — has to own the same
/// bookkeeping, and it cannot.
///
/// The reason it cannot is a host fact worth writing down: **unmounting a node
/// does not close the overlays it opened.** `OverlaySlot` is only ever removed by
/// [`Widget::update`]'s `close_overlay` path, so an overlay whose owner is torn
/// down stays on the stack. That rules out the obvious shape — the combobox
/// returns a `Popup` as one of its `children`, and rebuilds it with `present`
/// set — because a child whose presence flag changes has to change key to take
/// effect, and changing key remounts, and remounting orphans the panel.
///
/// So the owner keeps the host as a *field*, flips `present` on it, and forwards
/// [`Widget::overlay_opened`] / [`Widget::overlay_closed`]. Nothing about that is
/// specialised to a combobox, which is why it is a public type rather than code
/// copied into each component.
///
/// The forwarding is the sharp edge, and it has already cost this port once:
/// see `floom_widgets::popup`'s module note on an inherent method shadowing a
/// `Widget` trait method. A composite that defines `overlay_opened` and forgets to
/// forward compiles, runs, opens its panel, and never learns the handle — so it
/// can never close it, and nothing errors.
#[derive(Clone)]
pub struct OverlayHost {
    /// Whether the surface should be up. A widget sets it; the host does not
    /// decide, which is what lets the same host drive a dialog and a sheet.
    pub(crate) present: bool,
    /// The previous value of `present`, so the first `update` can tell a change
    /// from an initial state.
    pub(crate) last_present: bool,
    /// The ticket the host issued, while the surface sits on its overlay stack.
    pub(crate) handle: Option<OverlayHandle>,
    /// True while the surface is mounted in-tree rather than on the stack.
    in_tree: bool,
}

impl OverlayHost {
    pub fn new(present: bool) -> Self {
        Self {
            present,
            // "Not yet seen": the first `update` always acts on whatever the
            // widget currently is. Starting it at `!present` instead would lose
            // a transition made before the first update — a nested composite is
            // only mounted by the reconciler *after* the update pass that would
            // have run it, so `set_present(true)` in between would be swallowed
            // and the surface would never open.
            last_present: false,
            handle: None,
            in_tree: true,
        }
    }

    /// Whether the surface should be visible at all.
    pub fn is_shown(&self) -> bool {
        self.present
    }

    /// Whether the surface is currently mounted as in-tree children, i.e. the
    /// host refused the overlay (or never had one).
    pub fn takes_children_in_tree(&self) -> bool {
        self.present && self.in_tree
    }

    pub fn set_present(&mut self, present: bool) {
        self.present = present;
    }

    /// Opens or closes this surface's host overlay so it matches `present`.
    ///
    /// `build` runs only while the surface is coming up, and turns the node's
    /// own box plus the host viewport into the request to place — or `None` when
    /// there is nothing to put in an overlay after all (a `Popup` with no
    /// content), in which case nothing is mounted anywhere.
    pub fn reconcile(
        &mut self,
        cx: &mut dyn UpdateContext,
        id: ElementId,
        build: impl FnOnce(Rect, Rect) -> Option<OverlayRequest>,
    ) {
        if self.present == self.last_present {
            return;
        }
        let opening = self.present;
        self.last_present = self.present;
        if opening {
            let opened = match build(cx.bounds(), cx.viewport()) {
                Some(request) => cx.open_overlay(request),
                None => false,
            };
            self.in_tree = !opened;
        } else {
            if let Some(handle) = self.handle.take() {
                cx.close_overlay(handle);
            }
            self.in_tree = false;
        }
        // Both outcomes change which subtree exists: taking the host overlay
        // means the in-tree children must go, and a refusal means the in-tree
        // children must come back.
        cx.mark_children_dirty(id);
        cx.mark_dirty(id);
    }

    /// The in-tree layer to mount while there is no host overlay, as a child's
    /// `children()` contribution. Empty in the normal case.
    ///
    /// This is the half of the fallback that used to be written out inside every
    /// composite, and it is here because a composite which is about something else
    /// — a combobox's dropdown — needs the same two lines and would otherwise copy
    /// them: *the fallback covers the control*, because the containing block is the
    /// control's own box.
    ///
    /// That covering is a real limitation and is stated rather than hidden: a
    /// dropdown that should hang **below** its field falls back to covering it, on a
    /// host with no overlay support. The alternative — offsetting within the parent —
    /// is not expressible in [`LayoutRequest`], which offers [`LayoutRequest::
    /// absolute_fill`] and nothing for "below my own bottom edge". A hostless embedder
    /// gets a dropdown on top of its field, which is wrong but visible; the thing it
    /// must not do is show nothing.
    pub fn fallback_children(&self, content: Option<Child>, key: ChildKey) -> Vec<Child> {
        if !self.takes_children_in_tree() {
            return Vec::new();
        }
        let Some(content) = content else {
            return Vec::new();
        };
        let surface =
            Overlay::new().content_with(key, LayoutRequest::new().fill(), move || content.build());
        vec![Child::new(
            key,
            LayoutRequest::new().absolute_fill(),
            move || Box::new(surface.clone()),
        )]
    }

    /// Records the handle the host issued for this surface's overlay.
    pub fn opened(&mut self, handle: OverlayHandle) {
        self.handle = Some(handle);
    }

    /// The overlay carrying this surface came down — because the widget asked it
    /// to, or because the host dismissed it. Reports whether this surface was
    /// the one that closed.
    ///
    /// `present` is cleared as well: a dismissal the widget did not ask for
    /// (clicking outside a popup) has to leave the widget's own state matching
    /// what the user sees, or the next `update` would believe the surface is
    /// still up and never reopen it.
    pub fn closed(&mut self, handle: OverlayHandle) -> bool {
        if self.handle != Some(handle) {
            return false;
        }
        self.handle = None;
        self.present = false;
        self.last_present = false;
        true
    }
}

/// The fallback content for a body that was never supplied.
///
/// Shared by the dialog and the bottom sheet: both let a caller omit the body
/// entirely, and both want "something is there but it is empty" rather than
/// nothing, so the surface keeps its size.
pub(crate) fn empty_body() -> Child {
    Child::new(ChildKey::index(1), LayoutRequest::new(), || {
        Box::new(Text::new("(empty)"))
    })
}
