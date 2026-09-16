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

use std::fmt;

use floem_types::{Rect, ResourceId};

use crate::pool::NativeSlot;

/// How a participant is composited relative to the framework's own surface.
///
/// The order of the variants is *not* a strength order. Which of these is
/// "best" is a property of the platform: X11 can raise a subwindow above a
/// surface, Wayland has no subwindows at all, Windows child `HWND`s cannot be
/// composited under D3D without airspace tricks, KMS has real hardware planes.
/// A provider publishes what it can do and the host negotiates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCompositionMode {
    /// A sibling window/view stacked above the framework's surface.
    ///
    /// Cheap and always available, but it composes above the *whole* scene: it
    /// can be occluded by another participant, never by framework-painted
    /// content. See the D2 note in the Phase 16 plan.
    Overlay,
    /// Contents imported as a texture and drawn as a scene resource, so the
    /// participant composites *inside* the scene and can be occluded, clipped
    /// and transformed like anything else.
    Texture,
    /// A real compositor surface or hardware plane handed to the OS
    /// compositor (`SurfaceControl`, `CALayer`, KMS plane).
    Surface,
    /// Rendered offscreen and blitted into the framework's surface.
    Offscreen,
}

/// Whether a participant takes pointer input itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InputMode {
    /// The native view does not want input; the framework routes hits as if the
    /// participant were not there.
    #[default]
    Passthrough,
    /// The native view takes OS input itself. The framework must *subtract*
    /// the participant's rect from its own hit-test area rather than add to it.
    Capture,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeCapabilities {
    /// Composition modes this provider can do, in the provider's own order of
    /// preference.
    pub modes: &'static [NativeCompositionMode],
    /// Whether [`NativeViewProvider::snapshot`] can freeze a participant into a
    /// texture for a transition.
    pub snapshot: bool,
    /// Which input modes are honoured.
    pub input: InputModes,
    /// Whether participants can join the OS focus chain (and therefore take IME
    /// ownership).
    pub focusable: bool,
}

impl NativeCapabilities {
    pub const NONE: Self = Self {
        modes: &[],
        snapshot: false,
        input: InputModes::NONE,
        focusable: false,
    };

    pub fn supports(self, mode: NativeCompositionMode) -> bool {
        self.modes.contains(&mode)
    }

    /// The provider's own first choice, if it has any modes at all.
    pub fn preferred(self) -> Option<NativeCompositionMode> {
        self.modes.first().copied()
    }

    /// Pick the strongest mode both sides accept.
    ///
    /// `preferred` is the *caller's* order (strongest first) — usually
    /// `[Overlay, Texture, Offscreen]` or the reverse, depending on whether the
    /// widget needs to be occludable. A participant that cannot be composited
    /// at all yields `None`, which the host turns into a typed error at mount
    /// time rather than a blank region at paint time.
    pub fn negotiate(self, preferred: &[NativeCompositionMode]) -> Option<NativeCompositionMode> {
        preferred.iter().copied().find(|m| self.supports(*m))
    }
}

impl Default for NativeCapabilities {
    fn default() -> Self {
        Self::NONE
    }
}

/// What kind of native view a participant wants.
///
/// Deliberately a tag rather than a typed handle: the framework names intent
/// without knowing any concrete OS type, and a provider that has no opinion
/// about maps can still answer `Custom("com.example.threed")`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeViewKind {
    Map,
    Video,
    Web,
    /// A native text field. Focusable participants of this kind are the D3 case:
    /// the framework must yield IME ownership while one is composing.
    TextInput,
    Custom(&'static str),
}

/// Everything the host wants a participant to be, right now.
///
/// Absolute, not a delta: applying the same state twice is a no-op.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeState {
    /// Logical rect, already clipped by the host against ancestor clips and the
    /// viewport. Providers never walk a tree to work this out.
    pub rect: Rect,
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

    /// A participant is degenerate when either extent is negative; that is a
    /// host bug (a clip produced an inverted rect), and the controller rejects
    /// it rather than letting the provider clamp it silently to something the
    /// host did not ask for.
    pub fn validate(&self) -> Result<(), NativeError> {
        if self.rect.width() < 0.0 || self.rect.height() < 0.0 {
            return Err(NativeError::InvalidRect(self.rect));
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

/// A participant frozen into a texture, for a transition that moves it.
///
/// The resource is owned by whoever took the snapshot until `restore`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeSnapshot {
    /// Texture resource the renderer can draw as an image node.
    pub resource: ResourceId,
    /// Size of the frozen texture in device pixels. A snapshot taken before a
    /// scale change and restored after it is a stale-size bug.
    pub pixel_width: u32,
    pub pixel_height: u32,
}

/// Why a participant operation did not happen.
#[derive(Debug, Clone, PartialEq)]
pub enum NativeError {
    /// The provider cannot composite in this mode.
    UnsupportedMode(NativeCompositionMode),
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
    /// The provider refused. Carries a static reason because this crosses the
    /// OS boundary where a rich error type is not worth the plumbing.
    Provider(&'static str),
}

impl fmt::Display for NativeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedMode(mode) => write!(f, "provider cannot composite in {mode:?} mode"),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use floem_types::Rect;

    const OVERLAY_ONLY: &[NativeCompositionMode] = &[NativeCompositionMode::Overlay];
    const TEXTURE_FIRST: &[NativeCompositionMode] = &[
        NativeCompositionMode::Texture,
        NativeCompositionMode::Overlay,
    ];

    fn caps(modes: &'static [NativeCompositionMode]) -> NativeCapabilities {
        NativeCapabilities {
            modes,
            snapshot: true,
            input: InputModes::ALL,
            focusable: true,
        }
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
        let s = NativeState {
            rect: Rect {
                x0: 10.0,
                y0: 10.0,
                x1: 5.0,
                y1: 20.0,
            },
            scale: 1.0,
            visible: true,
            z: 0,
            input: InputMode::Passthrough,
            mode: NativeCompositionMode::Overlay,
        };
        assert_eq!(s.validate(), Err(NativeError::InvalidRect(s.rect)));
    }
}
