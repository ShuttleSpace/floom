//! Overlay vocabulary: what an overlay is for, and the ticket that identifies
//! the one a caller opened.
//!
//! Three crates have to name the same overlay without depending on each other:
//! a control asks its host to open one ([`floem-core`]), the framework crate
//! keeps the z-ordered stack ([`floem-overlay`]), and the host owns the widget
//! subtree. `floem-types` is the one crate all of them already depend on, so
//! the shared nouns live here — the same reason [`crate::ElementId`] does.

/// What an overlay is for.
///
/// The kind is not decoration: it picks the enter/exit animation and tells the
/// manager how the overlay relates to the window it sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverlayKind {
    /// A transient list anchored to the control that opened it. Dismissed by
    /// choosing a row, by Escape, or by the opener closing it; never modal.
    Popup,
    /// A window-sized modal surface that owns the keyboard while it is up.
    Dialog,
    /// A partial-height surface anchored to an edge of the window.
    BottomSheet,
}

/// An opaque ticket for one open overlay.
///
/// Issued by the host when the overlay opens and handed to both the opener and
/// the content, so either can close it later without knowing anything about the
/// stack. The number is only meaningful to the host that issued it — it is not
/// an index into anything, and it is deliberately not constructible from
/// outside the host except through [`OverlayHandle::new`], which the host uses
/// to mint them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OverlayHandle(u64);

impl OverlayHandle {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}
