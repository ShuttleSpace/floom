//! The native composition ABI (Phase 16, task 16.1).
//!
//! A "participant" is a native view the framework does not paint — an
//! `NSView`, an `HWND`, an Android `SurfaceView`, a KMS plane. This module is
//! only the *contract* between the framework and whoever can actually create
//! one: it names the modes, the state the host pushes down, and the errors a
//! participant can be in. Nothing here touches an OS.
//!
//! Two rules shape the module, and both exist because the failure they prevent
//! is invisible in a headless test:
//!
//! - **Every call takes the desired state, never a delta.** `apply` is
//!   idempotent, so a missed frame self-heals instead of accumulating error.
//! - **The logical→device boundary is crossed exactly once**, in
//!   [`to_device`], from a `scale` the host passes down. Rounding in both the
//!   host and the provider is the classic half-pixel seam.
//!
//! Providers implement [`NativeViewProvider`]; the framework talks to them
//! through [`crate::controller::NativeViewController`], which is what enforces
//! the lifecycle invariants listed in the plan (`destroy` for every `create`, a
//! recycled slot re-bound before any `apply`, no double freeze).

use std::borrow::Cow;
use std::fmt;

use floom_types::{NativeCapture, Rect};

use crate::pool::NativeSlot;

// The nouns a widget, a host and a provider all have to say live in
// `floom-types`: the widget layer sits above this crate and may not name
// anything in it, so "I want a video composited as an overlay" has to be
// spellable below both. Re-exported so callers of this crate keep one import.
//
// `NativeSnapshot` joined them in 16.4, and for a reason 16.1 did not have:
// the frozen texture has to be *painted*, and painting is the widget's job. A
// widget at L6 cannot name a type defined here at L4.
pub use floom_types::{InputMode, NativeCompositionMode, NativeSnapshot, NativeViewKind};

/// Whether a provider can crop a participant to a rect it does not own.
///
/// Cropping is a separate capability from compositing, and declaring it is what
/// keeps a host from having to guess. The failure it prevents is that a
/// participant placed in a clipping ancestor — a row in a scrolling list, a
/// stream under a modal — is either drawn *outside* its box (X11 subwindows and
/// Win32 child `HWND`s are clipped by their parent, but a KMS plane is not) or,
/// worse, silently rescaled to fit, which looks plausible on a map and is
/// obviously wrong on a video.
///
/// Conservative by default: a provider that has not thought about cropping is
/// assumed not to do it, so the host degrades the composition instead of
/// shipping a wrong frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ClipSupport {
    /// The platform clips a participant against its parent for free — an X11
    /// subwindow, a Win32 child `HWND`, an Android view in a `ViewGroup`. The
    /// host still sends the crop, but a provider that ignores it is correct.
    Platform,
    /// The provider must apply the crop itself: Wayland's
    /// `wp_viewport.set_source`, iOS's `layer.contentsRect`, or a texture-mode
    /// participant cropping its source rect before upload.
    Provider,
    /// The provider cannot crop at all: a KMS plane, or a view placed straight
    /// into the window's compositor tree with no crop primitive. A host must
    /// not hand such a provider a rect that needs cropping —
    /// [`NativeCapabilities::negotiate_croppable`] is how it finds a mode whose
    /// pixels it owns instead.
    #[default]
    Unsupported,
}

/// The set of [`InputMode`]s a provider can honour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputModes {
    pub passthrough: bool,
    pub capture: bool,
}

impl InputModes {
    pub const NONE: Self = Self {
        passthrough: false,
        capture: false,
    };
    pub const PASSTHROUGH_ONLY: Self = Self {
        passthrough: true,
        capture: false,
    };
    pub const ALL: Self = Self {
        passthrough: true,
        capture: true,
    };

    pub fn supports(self, mode: InputMode) -> bool {
        match mode {
            InputMode::Passthrough => self.passthrough,
            InputMode::Capture => self.capture,
        }
    }
}

impl Default for InputModes {
    fn default() -> Self {
        Self::PASSTHROUGH_ONLY
    }
}

/// What one provider can do.
///
/// `modes` is ordered strongest-first *for that provider*; the host never
/// assumes a cross-platform ordering.
///
/// The modes are a `Cow` rather than a `&'static` slice because a provider's
/// capabilities need not be known at compile time. A Rust provider answers
/// from a constant, so it borrows and allocates nothing; a provider reached
/// over the C ABI answers from a list the native side built at runtime, so it
/// owns. The controller asks for the capabilities on every `apply`, so the
/// difference between borrowing and allocating is the difference between a
/// frame that allocates and a frame that does not — and only the C side pays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCapabilities {
    /// Composition modes this provider can do, in the provider's own order of
    /// preference.
    pub modes: Cow<'static, [NativeCompositionMode]>,
    /// Whether [`NativeViewProvider::snapshot`] can freeze a participant into a
    /// texture for a transition.
    pub snapshot: bool,
    /// Which input modes are honoured.
    pub input: InputModes,
    /// Whether participants can join the OS focus chain (and therefore take IME
    /// ownership).
    pub focusable: bool,
    /// Whether a participant can be cropped to a rect it does not own. See
    /// [`ClipSupport`].
    pub clip: ClipSupport,
}

impl NativeCapabilities {
    pub const NONE: Self = Self {
        modes: Cow::Borrowed(&[]),
        snapshot: false,
        input: InputModes::NONE,
        focusable: false,
        clip: ClipSupport::Unsupported,
    };

    pub fn supports(&self, mode: NativeCompositionMode) -> bool {
        self.modes.contains(&mode)
    }

    /// The provider's own first choice, if it has any modes at all.
    pub fn preferred(&self) -> Option<NativeCompositionMode> {
        self.modes.first().copied()
    }

    /// Whether a rect that needs cropping can be handed to this provider at all.
    pub fn can_crop(&self) -> bool {
        !matches!(self.clip, ClipSupport::Unsupported)
    }

    /// Pick the strongest mode both sides accept.
    ///
    /// `preferred` is the *caller's* order (strongest first) — usually
    /// `[Overlay, Texture, Offscreen]` or the reverse, depending on whether the
    /// widget needs to be occludable. A participant that cannot be composited
    /// at all yields `None`, which the host turns into a typed error at mount
    /// time rather than a blank region at paint time.
    pub fn negotiate(&self, preferred: &[NativeCompositionMode]) -> Option<NativeCompositionMode> {
        preferred.iter().copied().find(|m| self.supports(*m))
    }

    /// Pick a mode whose pixels the framework owns, so a crop is free.
    ///
    /// This is the fallback for [`ClipSupport::Unsupported`]: a participant the
    /// platform will not crop for us must be handed to a path where *we* crop,
    /// and the only two such modes are [`NativeCompositionMode::Texture`] and
    /// [`NativeCompositionMode::Offscreen`]. Empty when neither side offers
    /// one, which is the honest answer for a plane-based backend — the host
    /// then hides the participant rather than letting it draw outside its box.
    pub fn negotiate_croppable(
        &self,
        preferred: &[NativeCompositionMode],
    ) -> Option<NativeCompositionMode> {
        preferred
            .iter()
            .copied()
            .filter(|m| m.is_host_composited())
            .find(|m| self.supports(*m))
    }
}

impl Default for NativeCapabilities {
    fn default() -> Self {
        Self::NONE
    }
}

/// Everything the host wants a participant to be, right now.
///
/// Absolute, not a delta: applying the same state twice is a no-op.
///
/// **Placement and clipping are two different things and are carried
/// separately.** `rect` is where the view is; `clip` is how much of it shows.
/// Folding them into one rect — pushing the *intersection* of the node box, its
/// ancestors and the viewport — is the tempting simplification and it is wrong:
/// a provider that honours its rect by fitting its content into it will
/// **rescale** a map or a video instead of cropping it. That bug is invisible on
/// a map (a smaller map looks like a map) and obvious on a video (a squashed
/// face), which is exactly the kind of bug that survives a release.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeState {
    /// Where the participant is, in logical window coordinates. Never
    /// intersected with anything: this is the box the layout gave it.
    pub rect: Rect,
    /// The part of `rect` that may be shown, in the same coordinates.
    ///
    /// `None` means nothing clips this participant — an ancestor box and the
    /// viewport already contain it — which is *not* the same statement as
    /// "clipped to nothing".
    ///
    /// A non-`None` clip is a **crop, not a placement**: show
    /// [`NativeState::visible_rect`] of the view *in place*, at full size. A
    /// provider whose platform crops for it (see [`ClipSupport::Platform`]) may
    /// ignore this; one that does not must crop its source, and one that cannot
    /// (see [`ClipSupport::Unsupported`]) must never be handed a state that
    /// needs it — [`crate::controller::NativeViewController::apply`] refuses.
    pub clip: Option<Rect>,
    /// Device-pixel scale the rect should be snapped with. See [`to_device`].
    pub scale: f64,
    /// `false` when the participant is clipped away entirely or hidden under a
    /// modal. A scrolled-away participant flips to invisible; it is *not* moved
    /// to a rect outside the viewport.
    pub visible: bool,
    /// Stacking order among participants, following tree paint order.
    pub z: u32,
    pub input: InputMode,
    pub mode: NativeCompositionMode,
}

impl NativeState {
    /// The participant's rect snapped to whole device pixels.
    pub fn device_rect(self) -> DeviceRect {
        to_device(self.rect, self.scale)
    }

    /// The region of the window this participant may actually cover: `rect`
    /// cropped by `clip`, in place and at the same size.
    pub fn visible_rect(self) -> Rect {
        match self.clip {
            Some(clip) => self.rect.intersect(clip),
            None => self.rect,
        }
    }

    /// Whether honouring this state requires cropping.
    ///
    /// `false` for the common case — a participant nothing clips — and for a
    /// clip that contains it, so a host that only needs to know whether the
    /// provider can cope does not have to re-derive the geometry.
    pub fn needs_crop(self) -> bool {
        match self.clip {
            Some(clip) => !clip.contains_rect(self.rect),
            None => false,
        }
    }

    /// A participant is degenerate when either extent is negative; that is a
    /// host bug (a clip produced an inverted rect), and the controller rejects
    /// it rather than letting the provider clamp it silently to something the
    /// host did not ask for.
    pub fn validate(&self) -> Result<(), NativeError> {
        if self.rect.width() < 0.0 || self.rect.height() < 0.0 {
            return Err(NativeError::InvalidRect(self.rect));
        }
        if let Some(clip) = self.clip {
            if clip.width() < 0.0 || clip.height() < 0.0 {
                return Err(NativeError::InvalidRect(clip));
            }
        }
        if !self.scale.is_finite() || self.scale <= 0.0 {
            return Err(NativeError::InvalidScale(self.scale));
        }
        Ok(())
    }
}

/// A rect in whole device pixels.
///
/// This is the only representation that crosses into a provider. Integers on
/// purpose: a platform cannot place a view at a fractional pixel, and letting
/// each provider invent its own rounding is how a half-pixel seam appears on
/// one platform and not another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DeviceRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl DeviceRect {
    pub const ZERO: Self = Self {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };

    pub fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// Snap a logical rect to whole device pixels.
///
/// Both edges are rounded independently so the *size* is a whole number of
/// pixels: rounding the origin and reusing the logical width is what leaves a
/// one-pixel gap between two adjacent participants at 1.5x.
pub fn to_device(rect: Rect, scale: f64) -> DeviceRect {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let x0 = (rect.x0 * scale).round();
    let y0 = (rect.y0 * scale).round();
    let x1 = (rect.x1 * scale).round();
    let y1 = (rect.y1 * scale).round();
    DeviceRect {
        x: x0 as i32,
        y: y0 as i32,
        width: (x1 - x0).max(0.0) as u32,
        height: (y1 - y0).max(0.0) as u32,
    }
}

/// Why a participant operation did not happen.
#[derive(Debug, Clone, PartialEq)]
pub enum NativeError {
    /// The provider cannot composite in this mode.
    UnsupportedMode(NativeCompositionMode),
    /// A state that needs cropping was handed to a provider that declared it
    /// cannot crop ([`ClipSupport::Unsupported`]).
    ///
    /// Refused rather than applied: the provider would draw its content outside
    /// the box the host gave it, over framework content it should be under.
    /// The host is expected to have re-negotiated into a mode it can crop (see
    /// [`NativeCapabilities::negotiate_croppable`]) or hidden the participant
    /// before this point, so this error means a host bug, not a platform limit.
    UnsupportedClip(Rect),
    /// No mode the participant would accept is offered by the provider at all
    /// (or the participant asked for nothing). Surfaced at bind time, not as a
    /// blank region at paint time — this is Wayland's shape (D5).
    NoSupportedMode,
    /// The provider cannot do this at all (e.g. `focus` on a provider that
    /// reports `focusable: false`).
    Unsupported(&'static str),
    /// `create` on a slot that is already bound. Two creates on one slot leak a
    /// native view the framework can no longer name.
    AlreadyBound(NativeSlot),
    /// An operation on a slot the provider was never told to create — most
    /// importantly, a *recycled* slot whose generation was bumped and which
    /// must therefore be re-bound before anything is applied to it.
    NotBound(NativeSlot),
    /// A rect with a negative extent.
    InvalidRect(Rect),
    /// A non-finite or non-positive scale.
    InvalidScale(f64),
    /// `snapshot` on a slot that is already frozen. Freezing twice would strand
    /// the first snapshot's texture.
    AlreadyFrozen(NativeSlot),
    /// `restore` on a slot that is not frozen.
    NotFrozen(NativeSlot),
    /// The provider refused.
    ///
    /// A `Cow` rather than a `&'static str` because the reason is often the
    /// provider's own: a native implementation that refuses to create a
    /// participant says why, and a static string would throw that away. A
    /// Rust provider still hands over a borrowed literal.
    Provider(Cow<'static, str>),
}

impl fmt::Display for NativeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedMode(mode) => write!(f, "provider cannot composite in {mode:?} mode"),
            Self::UnsupportedClip(clip) => write!(
                f,
                "provider cannot crop, so it cannot be given a rect clipped to {:?}",
                clip
            ),
            Self::NoSupportedMode => {
                write!(f, "provider offers no mode this participant accepts")
            }
            Self::Unsupported(what) => write!(f, "provider cannot {what}"),
            Self::AlreadyBound(slot) => write!(f, "slot {slot:?} is already bound"),
            Self::NotBound(slot) => write!(f, "slot {slot:?} is not bound"),
            Self::InvalidRect(rect) => write!(
                f,
                "rect {:?} has a negative extent ({}x{})",
                rect,
                rect.width(),
                rect.height()
            ),
            Self::InvalidScale(scale) => write!(f, "scale {scale} is not a positive finite number"),
            Self::AlreadyFrozen(slot) => write!(f, "slot {slot:?} is already frozen"),
            Self::NotFrozen(slot) => write!(f, "slot {slot:?} is not frozen"),
            Self::Provider(reason) => write!(f, "provider failed: {reason}"),
        }
    }
}

impl std::error::Error for NativeError {}

/// What a platform backend implements to make native views exist.
///
/// Object-safe on purpose: the host holds one `Box<dyn NativeViewProvider>`
/// selected at runtime. `&mut self` throughout because creating and moving
/// native views is not reentrant.
///
/// Providers are *not* expected to enforce the lifecycle invariants — that is
/// [`crate::controller::NativeViewController`]'s job, so that every platform
/// backend gets them for free and a backend cannot quietly opt out.
pub trait NativeViewProvider {
    /// What this provider can do. Queried at bind time; a cheap capability
    /// query beats a per-platform fork in the host.
    fn capabilities(&self) -> NativeCapabilities;

    /// Bind a concrete native handle to `slot`.
    ///
    /// `slot.generation` is what tells the provider whether this is a fresh
    /// slot or a recycled one that must be re-bound to a new handle.
    fn create(&mut self, slot: NativeSlot, kind: NativeViewKind) -> Result<(), NativeError>;

    /// Tear the native view down. The provider must tolerate being called for a
    /// slot it never created — teardown during an error path is not always
    /// symmetric in practice.
    fn destroy(&mut self, slot: NativeSlot);

    /// Move/resize/re-stack/show-hide. Called with the desired state, so
    /// repeating the last state must be free.
    fn apply(&mut self, slot: NativeSlot, state: NativeState);

    /// Hand OS focus to the participant. Only meaningful when
    /// `capabilities().focusable`.
    fn focus(&mut self, slot: NativeSlot);

    /// Take focus back. The framework resumes its own focus and IME ownership.
    fn blur(&mut self, slot: NativeSlot);

    /// Freeze the participant into a texture for the duration of a transition.
    /// `None` when the provider cannot snapshot.
    fn snapshot(&mut self, slot: NativeSlot) -> Option<NativeSnapshot>;

    /// Undo [`Self::snapshot`], handing the live view back.
    fn restore(&mut self, slot: NativeSlot);

    /// Drain any snapshots taken since the last call, so the app runner can
    /// hand their pixels to the renderer.
    ///
    /// Default empty: a provider whose host cannot snapshot (the common and
    /// honest case) has nothing to drain. Only a bridge wrapping a snapshotting
    /// host overrides this to forward to it.
    fn take_captures(&mut self) -> Vec<NativeCapture> {
        let _ = self;
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use floom_types::Rect;

    const OVERLAY_ONLY: &[NativeCompositionMode] = &[NativeCompositionMode::Overlay];
    const TEXTURE_FIRST: &[NativeCompositionMode] = &[
        NativeCompositionMode::Texture,
        NativeCompositionMode::Overlay,
    ];

    fn caps(modes: &'static [NativeCompositionMode]) -> NativeCapabilities {
        NativeCapabilities {
            modes: Cow::Borrowed(modes),
            snapshot: true,
            input: InputModes::ALL,
            focusable: true,
            clip: ClipSupport::Provider,
        }
    }

    fn state(rect: Rect) -> NativeState {
        NativeState {
            rect,
            clip: None,
            scale: 1.0,
            visible: true,
            z: 0,
            input: InputMode::Passthrough,
            mode: NativeCompositionMode::Overlay,
        }
    }

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
        Rect { x0, y0, x1, y1 }
    }

    #[test]
    fn negotiation_takes_the_callers_strongest_accepted_mode() {
        let c = caps(TEXTURE_FIRST);
        assert_eq!(
            c.negotiate(&[
                NativeCompositionMode::Overlay,
                NativeCompositionMode::Texture
            ]),
            Some(NativeCompositionMode::Overlay),
            "provider can do both, so the caller's first choice wins"
        );
    }

    #[test]
    fn negotiation_falls_back_past_modes_the_provider_lacks() {
        let c = caps(OVERLAY_ONLY);
        assert_eq!(
            c.negotiate(&[
                NativeCompositionMode::Surface,
                NativeCompositionMode::Texture,
                NativeCompositionMode::Overlay,
            ]),
            Some(NativeCompositionMode::Overlay)
        );
    }

    #[test]
    fn a_provider_with_no_modes_negotiates_to_nothing() {
        // Wayland has no subwindows: the host must see `None` and fall back,
        // not silently pick Overlay. This is device case D5's headless half.
        assert_eq!(
            NativeCapabilities::NONE.negotiate(&[NativeCompositionMode::Overlay]),
            None
        );
        assert_eq!(NativeCapabilities::NONE.preferred(), None);
    }

    #[test]
    fn both_edges_are_rounded_so_the_size_is_whole_pixels() {
        // 1.5x is where origin-only rounding shows up as a seam (D1).
        let r = Rect {
            x0: 10.0,
            y0: 10.0,
            x1: 20.0,
            y1: 20.0,
        };
        let d = to_device(r, 1.5);
        assert_eq!(
            d,
            DeviceRect {
                x: 15,
                y: 15,
                width: 15,
                height: 15
            }
        );
    }

    #[test]
    fn adjacent_participants_do_not_gap_or_overlap() {
        let scale = 1.5;
        let a = to_device(
            Rect {
                x0: 0.0,
                y0: 0.0,
                x1: 10.0,
                y1: 10.0,
            },
            scale,
        );
        let b = to_device(
            Rect {
                x0: 10.0,
                y0: 0.0,
                x1: 20.0,
                y1: 10.0,
            },
            scale,
        );
        assert_eq!(a.x + a.width as i32, b.x, "no gap, no overlap at 1.5x");
    }

    #[test]
    fn a_degenerate_scale_falls_back_to_one_rather_than_producing_nonsense() {
        let r = Rect {
            x0: 1.0,
            y0: 2.0,
            x1: 3.0,
            y1: 4.0,
        };
        assert_eq!(to_device(r, 0.0), to_device(r, 1.0));
        assert_eq!(to_device(r, f64::NAN), to_device(r, 1.0));
    }

    #[test]
    fn an_inverted_rect_is_rejected_by_the_state_not_silently_clamped() {
        let s = state(rect(10.0, 10.0, 5.0, 20.0));
        assert_eq!(s.validate(), Err(NativeError::InvalidRect(s.rect)));
    }

    #[test]
    fn an_inverted_clip_is_rejected_too() {
        let mut s = state(rect(0.0, 0.0, 10.0, 10.0));
        s.clip = Some(rect(8.0, 0.0, 2.0, 10.0));
        assert_eq!(
            s.validate(),
            Err(NativeError::InvalidRect(rect(8.0, 0.0, 2.0, 10.0)))
        );
    }

    // --- placement versus clipping ------------------------------------------

    #[test]
    fn a_clip_crops_in_place_rather_than_moving_the_view() {
        let mut s = state(rect(0.0, 0.0, 100.0, 50.0));
        s.clip = Some(rect(0.0, 0.0, 100.0, 20.0));
        assert_eq!(
            s.rect,
            rect(0.0, 0.0, 100.0, 50.0),
            "the placement is untouched: a provider that fit its content to the \
             visible rect would squash 50px of video into 20px"
        );
        assert_eq!(
            s.visible_rect(),
            rect(0.0, 0.0, 100.0, 20.0),
            "the crop is reported separately, as a region of the same view"
        );
        assert!(s.needs_crop());
    }

    #[test]
    fn no_clip_is_not_the_same_as_clipped_to_nothing() {
        let mut unclipped = state(rect(0.0, 0.0, 100.0, 50.0));
        unclipped.clip = None;
        let clipped_away = state(Rect::ZERO);

        assert!(!unclipped.needs_crop());
        assert_eq!(unclipped.visible_rect(), unclipped.rect);
        assert!(clipped_away.visible_rect().is_empty());
    }

    #[test]
    fn a_clip_that_contains_the_view_needs_no_cropping() {
        let mut s = state(rect(10.0, 10.0, 20.0, 20.0));
        s.clip = Some(rect(0.0, 0.0, 100.0, 100.0));
        assert!(
            !s.needs_crop(),
            "a clip that does not bite must not force a provider downgrade"
        );
        assert_eq!(s.visible_rect(), s.rect);
    }

    #[test]
    fn croppable_negotiation_only_considers_modes_the_framework_composites() {
        let c = caps(&[
            NativeCompositionMode::Overlay,
            NativeCompositionMode::Texture,
            NativeCompositionMode::Offscreen,
        ]);
        assert_eq!(
            c.negotiate_croppable(&[
                NativeCompositionMode::Overlay,
                NativeCompositionMode::Surface,
                NativeCompositionMode::Offscreen,
            ]),
            Some(NativeCompositionMode::Offscreen),
            "Overlay and Surface are shown by the OS, so neither can be cropped \
             by the framework; Offscreen is"
        );
        assert_eq!(
            NativeCapabilities {
                clip: ClipSupport::Unsupported,
                ..c
            }
            .negotiate_croppable(&[NativeCompositionMode::Overlay]),
            None,
            "and when the caller will accept nothing the framework composites, \
             a host that cannot crop has no correct mode to offer"
        );
    }

    #[test]
    fn a_provider_that_has_not_declared_cropping_is_assumed_unable() {
        let declared = NativeCapabilities {
            modes: Cow::Borrowed(OVERLAY_ONLY),
            snapshot: false,
            input: InputModes::ALL,
            focusable: false,
            clip: ClipSupport::Unsupported,
        };
        assert!(!declared.can_crop());
        assert!(
            NativeCapabilities {
                clip: ClipSupport::Platform,
                ..declared
            }
            .can_crop()
        );
        assert_eq!(
            NativeCapabilities::default().clip,
            ClipSupport::Unsupported,
            "the default must fail closed: a provider that has not thought about \
             cropping must not be handed a rect that needs it"
        );
    }
}
