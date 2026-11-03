
use std::any::Any;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use floom_accessibility::SemanticsRole;
use floom_core::{
    Child, ChildKey, CreateContext, ElementId, LayoutRequest, OverlayHandle, OverlayKind,
    OverlayRequest, PaintContext, Rect, SemanticsContext, UpdateContext, Widget, WidgetAny,
};

use crate::overlay::Overlay;
use floom_widgets::primitives::SemanticsHint;
use floom_widgets::Grid;

use crate::overlay_bookkeeping::OverlayHost;

// Popup

/// Where a [`Popup`]'s panel sits relative to the control that owns it.
///
/// This is a [`Popup`]'s only real decision. Everything else about it — asking the
/// host, learning the handle, being dismissed from outside, falling back to an
/// in-tree layer when the host refuses — is [`OverlayHost`] and was already
/// there. What was missing was geometry: a popup whose rect *is* its own box covers
/// the control, which is right for a menu pinned to a toolbar button and wrong for
/// every dropdown, because a combobox's list belongs *below* its field.
///
/// Kept as three cases rather than a general rect because a caller that can name an
/// arbitrary rect will eventually name one that puts the panel off-screen, and
/// "below the thing I own" is the property that keeps a dropdown attached to its
/// field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PopupPlacement {
    /// Cover the control's own box. The original behaviour, and the default, so
    /// every existing `Popup` is unchanged.
    Fill,
    /// Directly below the control, as wide as it, `gap` pixels of daylight between.
    ///
    /// The height is explicit because the host lays content out *against* the
    /// requested rect and clips rather than growing — so it is a real bound and not
    /// a hint. A caller with a token for it should pass it; a combobox has
    /// `ComboboxTokens::panel_max_height`, which v1 declared and never read.
    Below { gap: f64, height: f64 },
    /// Directly above the control, for one near the bottom of the window.
    ///
    /// Not chosen automatically: "does it fit below?" needs the viewport and a
    /// content height this widget does not have, and a control that silently flips
    /// sides between frames is worse than one that is consistently underneath.
    Above { gap: f64, height: f64 },
}

impl Default for PopupPlacement {
    fn default() -> Self {
        PopupPlacement::Fill
    }
}

/// The rect a placement asks the host for, given the anchor's box.
///
/// Pure geometry, public and separate from the widget so it can be tested without a
/// host, and so a caller that wants to place a panel itself can ask the same question
/// the widget would have answered.
pub fn popup_placement_rect(placement: PopupPlacement, anchor: Rect) -> Rect {
    match placement {
        PopupPlacement::Fill => anchor,
        PopupPlacement::Below { gap, height } => Rect {
            x0: anchor.x0,
            y0: anchor.y1 + gap,
            x1: anchor.x1,
            y1: anchor.y1 + gap + height,
        },
        PopupPlacement::Above { gap, height } => Rect {
            x0: anchor.x0,
            y0: anchor.y0 - gap - height,
            x1: anchor.x1,
            y1: anchor.y0 - gap,
        },
    }
}

/// A transient, non-modal layer anchored above its siblings.
///
/// [`Dialog`] is the modal sibling: it carries a titled card and owns the
/// keyboard. `Popup` is the lighter primitive for menus, tooltips and inline
/// pickers — a togglable layer whose content fills the control's box, with no
/// scrim of its own. It is non-modal, so a click that lands beside it reaches
/// whatever is underneath, and the host dismisses it on that click. Like every
/// overlay composite it contributes nothing while hidden.
#[derive(Clone)]
pub struct Popup {
    surface: OverlayHost,
    label: Option<String>,
    content: Option<Child>,
    placement: PopupPlacement,
}

impl std::fmt::Debug for Popup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Popup")
            .field("present", &self.surface.present)
            .field("label", &self.label)
            .finish()
    }
}

impl Popup {
    pub fn new() -> Self {
        Self {
            surface: OverlayHost::new(false),
            label: None,
            content: None,
            placement: PopupPlacement::default(),
        }
    }

    /// Where the panel sits relative to this control.
    ///
    /// [`PopupPlacement::Fill`] by default, which is to say: unchanged. A dropdown
    /// wants [`PopupPlacement::Below`].
    pub fn placement(mut self, placement: PopupPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// The placement, for a caller that reads it back.
    pub fn current_placement(&self) -> PopupPlacement {
        self.placement
    }

    /// Sets the popup content (does not show it; call [`Popup::set_present`]).
    pub fn content(mut self, child: Child) -> Self {
        self.content = Some(child);
        self
    }

    /// Sets the content with an explicit key and layout.
    pub fn content_with(
        mut self,
        key: impl Into<ChildKey>,
        layout: LayoutRequest,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.content = Some(Child::new(key, layout, build));
        self
    }

    /// A label published to assistive technology while the popup is shown.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Shows or hides the popup; the next frame reconciles.
    pub fn set_present(&mut self, present: bool) {
        self.surface.set_present(present);
    }

    /// Hides the popup and drops its content.
    pub fn dismiss(&mut self) {
        self.content = None;
        self.surface.set_present(false);
    }

    /// Whether the popup is up.
    pub fn is_present(&self) -> bool {
        self.surface.is_shown()
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        if self.surface.is_shown() {
            SemanticsHint {
                role: SemanticsRole::Group,
                label: self.label.clone(),
                state: Default::default(),
            }
        } else {
            SemanticsHint::decorative()
        }
    }
}

impl Default for Popup {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Popup {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let content = self.content.clone();
        // A popup is anchored to the control that owns it, so unlike a dialog it
        // takes its own box as the rect the content is laid out against — subject to
        // the placement, which is the one thing that decides whether the panel covers
        // the control or hangs off it.
        let placement = self.placement;
        self.surface.reconcile(cx, id, move |bounds, _viewport| {
            let content = content.clone()?;
            Some(OverlayRequest::new(
                OverlayKind::Popup,
                false,
                popup_placement_rect(placement, bounds),
                move |_handle| content.build(),
            ))
        });
    }

    fn children(&self) -> Vec<Child> {
        if !self.surface.takes_children_in_tree() {
            return Vec::new();
        }
        let content = match &self.content {
            Some(content) => content.clone(),
            None => return Vec::new(),
        };
        // No host overlay: fall back to an in-tree layer, as before.
        let surface = Overlay::new().content_with(
            ChildKey::index(0),
            LayoutRequest::new().fill(),
            move || content.build(),
        );
        vec![Child::new(
            ChildKey::index(0),
            LayoutRequest::new().absolute_fill(),
            move || Box::new(surface.clone()),
        )]
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().absolute_fill())
    }

    fn overlay_opened(&mut self, cx: &mut dyn UpdateContext, id: ElementId, handle: OverlayHandle) {
        self.surface.opened(handle);
        cx.mark_dirty(id);
    }

    fn overlay_closed(
        &mut self,
        cx: &mut dyn UpdateContext,
        id: ElementId,
        handle: OverlayHandle,
        _content: Box<dyn WidgetAny>,
    ) {
        if self.surface.closed(handle) {
            cx.mark_dirty(id);
        }
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        floom_widgets::emit_semantics(self.semantics_hint(), cx);
    }
}
