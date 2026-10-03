//! The widget-facing overlay request.
//!
//! A control cannot mount nodes itself — it does not own the tree — so when a
//! `Select` wants its list to escape the parent's clip it asks the host
//! instead. [`OverlayRequest`] is what it asks with: the geometry it wants, the
//! policy (modal or not), and the content, which arrives as a *factory* rather
//! than a finished widget.
//!
//! The factory exists for one reason: the content usually needs its own
//! [`OverlayHandle`] so it can close itself when the user picks a row, and that
//! handle cannot exist until the host has decided to honour the request.
//! Building the content eagerly would leave it permanently unable to dismiss
//! itself — which is precisely how a popup ends up stuck on screen.

use crate::WidgetAny;
use floom_types::{OverlayHandle, OverlayKind, Rect};

/// A request to place widget content above the whole tree.
///
/// Consumed by the host, which mounts the content, positions it at [`bounds`],
/// paints it after every main-tree node, and hit-tests it first.
///
/// [`bounds`]: OverlayRequest::bounds
pub struct OverlayRequest {
    /// What this overlay is for; picks the enter/exit animation.
    pub kind: OverlayKind,
    /// Whether the overlay blocks the tree beneath it, swallowing pointer
    /// events that miss its content.
    pub modal: bool,
    /// Viewport-relative rect the content is positioned at. The content is laid
    /// out *against* this size, so a request that is too small clips the
    /// content rather than growing.
    pub bounds: Rect,
    /// Builds the content, handing it the handle the host issued. Called once,
    /// during the frame that honours the request.
    pub content: Box<dyn FnOnce(OverlayHandle) -> Box<dyn WidgetAny>>,
}

impl std::fmt::Debug for OverlayRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OverlayRequest")
            .field("kind", &self.kind)
            .field("modal", &self.modal)
            .field("bounds", &self.bounds)
            .finish_non_exhaustive()
    }
}

impl OverlayRequest {
    /// A request whose content is built by `content`, receiving its handle.
    pub fn new(
        kind: OverlayKind,
        modal: bool,
        bounds: Rect,
        content: impl FnOnce(OverlayHandle) -> Box<dyn WidgetAny> + 'static,
    ) -> Self {
        Self {
            kind,
            modal,
            bounds,
            content: Box::new(content),
        }
    }

    /// A request whose content ignores its handle — for overlays that only the
    /// opener ever closes, such as a modal dialog whose buttons call back out.
    pub fn detached(
        kind: OverlayKind,
        modal: bool,
        bounds: Rect,
        content: impl WidgetAny + 'static,
    ) -> Self {
        Self::new(kind, modal, bounds, move |_| Box::new(content))
    }
}
