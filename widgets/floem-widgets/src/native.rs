//! A native view occupying a framework layout box (Phase 16, task 16.2).
//!
//! [`NativeSurface`] is the widget-side half of the native composition seam. It
//! paints nothing — the OS view paints itself — and its whole job is to state,
//! every `update()`, what kind of native view it wants and what it will accept.
//! The host decides: it negotiates a composition mode with its provider, owns
//! the participant's lifetime, and pushes geometry down each frame.
//!
//! The lifetime ownership is the reason this is a *request* and not a
//! constructor. A native view has to outlive the frame that created it, be
//! re-bound when its slot is recycled for another row, and be destroyed exactly
//! once. A widget that owned one directly would leak it on every unmount — and
//! would leak it silently, because a leaked native view looks exactly like a
//! working one until the device runs out of them.

use floem_accessibility::{SemanticsRole, SemanticsState};
use floem_core::{
    CreateContext, ElementId, LayoutRequest, NativeGrant, NativeRequest, PaintContext,
    SemanticsContext, UpdateContext, Widget,
};
use floem_types::{InputMode, NativeCompositionMode, NativeViewKind};

use crate::primitives::{SemanticsHint, emit_semantics};

/// A native view in a framework box.
///
/// Paints nothing and holds no children: everything visible is the OS view the
/// host places over this node's rect.
pub struct NativeSurface {
    request: NativeRequest,
    grant: NativeGrant,
    width: Option<f32>,
    height: Option<f32>,
    label: Option<String>,
    /// Whether this surface can take focus — and therefore take IME ownership
    /// away from the framework while it is focused (decision D4). Native text
    /// fields want this; a video surface does not.
    focusable: bool,
}

impl NativeSurface {
    /// A surface holding `kind`, composited above the scene and capturing
    /// input — the common case for a video or map view.
    pub fn new(kind: NativeViewKind) -> Self {
        Self {
            request: NativeRequest::new(kind),
            grant: NativeGrant::Refused,
            width: None,
            height: None,
            label: None,
            focusable: false,
        }
    }

    /// Composition modes this surface will accept, strongest first.
    pub fn modes(mut self, modes: &'static [NativeCompositionMode]) -> Self {
        self.request = self.request.modes(modes);
        self
    }

    /// Whether the view takes pointer input itself.
    pub fn input(mut self, input: InputMode) -> Self {
        self.request = self.request.input(input);
        self
    }

    /// The recycling identity: two surfaces that are never on screen at once
    /// should share a key space so the second reuses the first's slot.
    pub fn key(mut self, key: u64) -> Self {
        self.request = self.request.key(key);
        self
    }

    /// Explicit intrinsic size. Without it the surface takes whatever its
    /// parent gives it, which for a leaf means collapsing to nothing.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Whether this surface joins the OS focus chain.
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    /// What the host decided. [`NativeGrant::Refused`] means no provider, or no
    /// acceptable mode: read it and draw a fallback, because a surface that
    /// shows nothing is indistinguishable from one that was never asked for.
    pub fn grant(&self) -> NativeGrant {
        self.grant
    }

    pub fn is_granted(&self) -> bool {
        self.grant.is_granted()
    }

    /// The kind this surface asked for.
    pub fn kind(&self) -> NativeViewKind {
        self.request.kind
    }

    /// A `Group` when focusable, decorative otherwise: the host registers
    /// focus from the role, and a native view that cannot be focused should not
    /// appear in the tab order.
    pub fn semantics_hint(&self) -> SemanticsHint {
        if !self.focusable {
            return SemanticsHint::decorative();
        }
        SemanticsHint {
            role: SemanticsRole::Group,
            label: self.label.clone(),
            state: SemanticsState::empty(),
        }
    }
}

impl Widget for NativeSurface {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        // Re-stated every frame on purpose: the host makes it idempotent, and
        // this is how a virtualized row asks for a different slot (a changed
        // `key`) without any explicit teardown.
        let grant = cx.request_native(&self.request);
        if grant != self.grant {
            self.grant = grant;
            cx.mark_dirty(id);
        }
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        if self.width.is_none() && self.height.is_none() {
            return None;
        }
        let mut request = LayoutRequest::new();
        if let Some(width) = self.width {
            request = request.width(width);
        }
        if let Some(height) = self.height {
            request = request.height(height);
        }
        Some(request)
    }

    /// Nothing: the native view paints itself. 16.4 draws a frozen snapshot
    /// here as an image node when the participant is mid-transition.
    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}
