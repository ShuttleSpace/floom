// SPDX-License-Identifier: MIT
//! The widget-facing request for a native view (Phase 16, task 16.2).
//!
//! A control that wants a map, a video or a web view in its box cannot create
//! one — it does not own a window — so it asks the host, exactly as a `Select`
//! asks for an overlay. [`NativeRequest`] is what it asks with.
//!
//! The host owns the participant's lifetime, and that is the whole reason this
//! is a request rather than a constructor: a native view must outlive the frame
//! that created it, be re-bound when its slot is recycled for another row, and
//! be destroyed exactly once. A widget that owned one directly would leak it
//! on every unmount.

use floom_types::{InputMode, NativeCompositionMode, NativeViewKind};

/// A request to put a native view in this node's box.
///
/// Consumed by the host, which negotiates the composition mode with its
/// provider, tracks the participant, and pushes geometry down each frame.
///
/// Asking is idempotent: a widget re-states its request every `update()`, and a
/// host that has already granted it does nothing the second time. That is what
/// lets a virtualized row change its `key` and get a different slot without any
/// explicit teardown.
pub struct NativeRequest {
    /// What kind of view this is. Providers read it as intent, not as a type
    /// they must know.
    pub kind: NativeViewKind,
    /// Composition modes this participant will accept, strongest first. The
    /// host picks the first one its provider offers.
    pub modes: &'static [NativeCompositionMode],
    /// Whether the view takes pointer input itself.
    pub input: InputMode,
    /// Stable identity used to recycle slots.
    ///
    /// Two participants that are never on screen at once should share a key
    /// space so the second can reuse the first's slot — a virtualized list
    /// passes its row key, and scrolling then recycles a handful of native
    /// views instead of creating one per row. The default (`0`) means "this
    /// participant is its own identity", which is right for anything that is
    /// not in a list.
    pub key: u64,
}

impl std::fmt::Debug for NativeRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeRequest")
            .field("kind", &self.kind)
            .field("modes", &self.modes)
            .field("input", &self.input)
            .field("key", &self.key)
            .finish()
    }
}

impl NativeRequest {
    /// A request for `kind`, composited above the scene and capturing input —
    /// the common case for a video or map view.
    pub fn new(kind: NativeViewKind) -> Self {
        Self {
            kind,
            modes: &[NativeCompositionMode::Overlay],
            input: InputMode::Capture,
            key: 0,
        }
    }

    /// Modes this participant will accept, strongest first.
    pub fn modes(mut self, modes: &'static [NativeCompositionMode]) -> Self {
        self.modes = modes;
        self
    }

    /// Whether the view takes pointer input itself.
    pub fn input(mut self, input: InputMode) -> Self {
        self.input = input;
        self
    }

    /// The recycling identity. See [`NativeRequest::key`].
    pub fn key(mut self, key: u64) -> Self {
        self.key = key;
        self
    }
}

/// The fallback a participant gets when the host cannot composite natively.
///
/// Returned by [`UpdateContext::request_native`]: a widget that needs a real
/// native view must decide what to do without one, and "nothing" is a decision
/// worth making explicitly rather than by omission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeGrant {
    /// No provider, or no mode this participant accepts.
    Refused,
    /// The host will composite this participant, in this mode.
    Granted(NativeCompositionMode),
}

impl NativeGrant {
    pub fn is_granted(self) -> bool {
        matches!(self, Self::Granted(_))
    }

    pub fn mode(self) -> Option<NativeCompositionMode> {
        match self {
            Self::Granted(mode) => Some(mode),
            Self::Refused => None,
        }
    }
}
