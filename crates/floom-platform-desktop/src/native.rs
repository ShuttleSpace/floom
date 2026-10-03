//! The desktop native-view bridge (Phase 16, task 16.2, step 7).
//!
//! A desktop participant is a real child view: an `NSView` added to the
//! window's content view, an `HWND` created as a child of it, an X11
//! subwindow. This module is the half of that bridge which needs no OS — the
//! translation from a [`NativeState`] into a frame the OS can be handed, and
//! the [`NativeViewProvider`] impl around it. The OS half is a handful of
//! calls behind [`NativeViewHost`], with three consequences worth stating:
//!
//! - the arithmetic below is testable without a window, which is where the
//!   bugs in it actually live;
//! - a second backend (Win32, X11) is a trait impl and not a copy of this
//!   file;
//! - what is left to device work is exactly the six methods on the trait, so
//!   the next session knows what it is signing up for.
//!
//! # The origin flip belongs to the backend, and it is wrong first
//!
//! `NativeState::rect` is top-left logical, in window coordinates.
//! [`ViewPlacement`] keeps that convention — top-left, snapped to whole device
//! pixels — and does **not** apply any OS convention. Win32 and X11 are
//! top-left, so for them the placement is the answer; AppKit's default origin is
//! bottom-left, so the AppKit backend converts with [`os_frame`], and nothing
//! else in the chain has to know which of the two it is talking to.
//!
//! Turning the placement into an OS frame is the part that is wrong first, and
//! wrong in a way that survives a demo. The conversion that suggests itself —
//! mirror in *logical* units, then snap to device pixels — is not the same
//! function as snapping and *then* mirroring, because snapping rounds each edge
//! independently. At 2x, a participant at logical `y 0.75..1.25` of a parent 10
//! units tall belongs on device row 2 (so `y_os = 17`), while mirroring first
//! lands it on row 3 mirrored to `y_os = 18`. One pixel, and it is one pixel
//! *of disagreement with the framework*, which is what a seam is: the same box
//! painted by the scene and placed by the OS must be the same pixels, or
//! adjacent content shows a hairline at some (not all) scales — the failure
//! that is invisible on the developer's 2x display and reported by one user at
//! 1.5x.
//!
//! So: **snap once, in [`floom_native::to_device`], and mirror in device
//! space.** [`os_frame`] only ever receives a `DeviceRect`, which is what makes
//! that structural rather than a comment, and `mirror_device` is an involution,
//! which makes the invariant checkable — mirroring an OS frame back must
//! reproduce the very rect the framework snapped.
//!
//! Two halves of the same bug are worth stating separately:
//!
//! - the height to mirror against is the **parent view's**, not the window's.
//!   They are the same number for a participant added straight to the content
//!   view and differ the moment one goes into a container, which is precisely
//!   the case nobody has on day one;
//! - whether a flip is needed at all is a property of the *specific parent
//!   view*, not of the platform: `NSView::isFlipped` is per-view, and a window
//!   whose content view is already flipped wants top-left. The AppKit backend
//!   asks the view rather than assuming.
//!
//! # A crop is a container, for the backends that crop with their own frames
//!
//! `NativeState` separates placement from clipping, so a clipped participant
//! keeps its size and is shown through a window. A backend that crops by
//! clipping a subview — AppKit, and the `Platform` arm of [`ClipSupport`]
//! generally — needs that window to be a *view*: the participant goes inside a
//! container the size of the crop and is offset within it, which is what
//! [`split_placement`] computes. Shrinking the participant's own frame instead
//! would rescale its content, and scrolling it by moving its bounds origin
//! would move the participant's own scroll position — a crop of somebody else's
//! view has to be somebody else's view.

use std::collections::HashMap;

use floom_native::{
    ClipSupport, DeviceRect, InputMode, InputModes, NativeCapabilities, NativeCompositionMode,
    NativeError, NativeSlot, NativeSnapshot, NativeState, NativeViewKind, NativeViewProvider,
    to_device,
};
use floom_platform::{Capability, PlatformCapabilities};
use floom_types::{NativeCapture, Size};

/// A participant's placement, in the framework's own device space.
///
/// Top-left origin, whole device pixels, in the coordinate system of the view
/// the participant is added to. Produced by [`to_view_placement`] and consumed
/// by the backend, which converts for its OS with [`os_frame`] and — for a
/// backend that crops with its own views — splits with [`split_placement`].
///
/// Nothing here is in OS coordinates on purpose: the conversion has a single
/// rounding-sensitive step, and a backend that never needed it (Win32, X11)
/// should not be handed its output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewPlacement {
    /// Where the view goes. Full size, whether or not it is cropped.
    pub frame: DeviceRect,
    /// The part of `frame` to actually show, when an ancestor clips the
    /// participant. `None` means show all of it.
    ///
    /// A crop is *not* a smaller frame: the view keeps its size and the host
    /// shows a window onto it. A provider that honoured a crop by shrinking the
    /// frame would rescale its content — the bug the `clip` channel exists to
    /// prevent.
    pub crop: Option<DeviceRect>,
    /// Whether the view takes the pointer.
    ///
    /// Part of the placement rather than a separate call, because on a backend
    /// that has to do something about it, "does it take the pointer" *is* a
    /// property of how the view is placed: AppKit answers it with `hitTest:`,
    /// so a passthrough participant is not merely not-added to the framework's
    /// hit regions, it is placed in a way the pointer ignores. Folding it in
    /// also means the bridge's memo covers it for free — a separate call would
    /// have to be issued on the *first* placement too, which is the one that is
    /// easiest to miss.
    pub input: InputMode,
}

/// Where a view origin runs on the way to the OS.
///
/// Named for the origin corner, because that is what a backend has to ask about
/// its *parent* view: AppKit's `isFlipped` is a property of the view whose
/// coordinate system frames are expressed in, and a flipped content view makes
/// [`Self::TopLeft`] the right answer on a platform whose default is
/// [`Self::BottomLeft`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// y grows downward, as in the framework's own logical space and in Win32
    /// and X11. No conversion.
    TopLeft,
    /// y grows upward from the parent's bottom edge, as in an unflipped AppKit
    /// view. Needs [`mirror_device`].
    BottomLeft,
}

/// Mirror a device rect about a bottom-left origin.
///
/// `parent_device_height` is the height of the **parent view** in device
/// pixels. Total — an involution on the rects it is given, which is what makes
/// "snap then mirror" verifiable: mirroring an OS frame back reproduces the
/// rect the framework snapped.
///
/// Saturating rather than wrapping: a rect from a scrolled-away participant can
/// legitimately sit outside the parent, and an `i32` overflow here would place
/// a view at a meaningless coordinate instead of an off-screen one.
pub fn mirror_device(rect: DeviceRect, parent_device_height: i32) -> DeviceRect {
    DeviceRect {
        x: rect.x,
        y: parent_device_height.saturating_sub(rect.y.saturating_add(rect.height as i32)),
        width: rect.width,
        height: rect.height,
    }
}

/// The parent view's height in device pixels.
///
/// Separate from [`to_device`] because it is a *scale* of a length, not of a
/// rect: rounding it is what makes the mirror land on whole pixels for a parent
/// whose logical height is fractional.
fn device_parent_height(parent: Size, scale: f64) -> i32 {
    (parent.height * sane_scale(scale)).round() as i32
}

/// A scale a backend handed us, or `1.0` if it is not one.
///
/// The backend owns this number, so unlike [`NativeState::scale`] it arrives
/// from outside the framework. A `NaN` would otherwise become a silent `0` at
/// the `as i32` cast above and place every participant at the top of its parent.
fn sane_scale(scale: f64) -> f64 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

/// Convert a placement rect into the frame a backend's OS expects.
///
/// The **only** place the origin convention is applied, and the only place
/// `scale` is allowed to touch a length rather than a rect. Passed a
/// [`DeviceRect`] rather than a logical one because that is what keeps the snap
/// and the mirror in that order: there is no logical rect in reach here to
/// mirror by mistake.
pub fn os_frame(rect: DeviceRect, parent: Size, scale: f64, origin: Origin) -> DeviceRect {
    match origin {
        Origin::TopLeft => rect,
        Origin::BottomLeft => mirror_device(rect, device_parent_height(parent, scale)),
    }
}

/// The two frames a cropping backend needs: the clipping container and the
/// participant inside it.
///
/// Both in the framework's top-left device space, and `content` is relative to
/// `container`. A backend that crops by clipping a subview sets the container's
/// frame (converted by [`os_frame`]) and then the content's frame, which is a
/// plain subtraction — the container it makes is flipped, so the content's
/// coordinates need no conversion at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewFrames {
    /// The visible window onto the participant, and the view that clips.
    pub container: DeviceRect,
    /// The participant's full size, offset so that the part of it the crop
    /// selects lands inside `container`.
    pub content: DeviceRect,
}

/// Split a placement into the container that clips and the content it clips.
///
/// A crop becomes a container the size of the crop with the participant offset
/// inside it by the difference, which is the same statement as "show that part
/// of it" and is the one an OS can act on. Without a crop the container is the
/// participant's own box and the content fills it, so the cropping path is the
/// only path — there is no second arrangement that only a clipped participant
/// reaches.
pub fn split_placement(placement: ViewPlacement) -> ViewFrames {
    let frame = placement.frame;
    let window = placement.crop.unwrap_or(frame);
    ViewFrames {
        container: window,
        content: DeviceRect {
            x: frame.x.saturating_sub(window.x),
            y: frame.y.saturating_sub(window.y),
            width: frame.width,
            height: frame.height,
        },
    }
}

/// A device rect in points, the unit AppKit frames are expressed in.
///
/// Dividing is safe where rounding would not be: the *device* value is already
/// a whole number of pixels, so a host that multiplies back by the backing scale
/// factor recovers it exactly, while a host that had snapped to whole *points*
/// would have thrown away the sub-point alignment the snap existed to keep. The
/// pair is only meaningful together — see [`os_frame`].
pub fn device_to_points(rect: DeviceRect, scale: f64) -> (f64, f64, f64, f64) {
    let scale = sane_scale(scale);
    (
        rect.x as f64 / scale,
        rect.y as f64 / scale,
        rect.width as f64 / scale,
        rect.height as f64 / scale,
    )
}

/// Where a participant goes, given the state the host pushed.
///
/// The one crossing from logical to device. No mirroring and no OS convention:
/// both of those are the backend's business, and the snap has to happen exactly
/// once, here, so that [`floom_native::to_device`] is the only rounding in the
/// chain.
pub fn to_view_placement(state: NativeState) -> ViewPlacement {
    // The crop is snapped by the same call as the frame, so it stays a
    // sub-rect of it: `visible_rect` is an intersection of `rect`, and
    // `to_device` is monotone in the edges it touches. A crop computed any
    // other way could fall outside its own frame, which is how a clipped
    // participant ends up showing a slice of the wrong content.
    let frame = state.device_rect();
    let crop = state
        .needs_crop()
        .then(|| to_device(state.visible_rect(), state.scale));
    ViewPlacement {
        frame,
        crop,
        input: state.input,
    }
}

/// The OS calls the bridge needs, so that everything above them can be tested
/// without a window.
///
/// A backend is a trait impl, not a copy of this file: the placement arrives
/// snapped to whole device pixels in the framework's top-left convention, and
/// the backend's own work is [`os_frame`], [`split_placement`],
/// [`device_to_points`] and then the OS call. What is left over is genuinely
/// platform-specific — which view, which selector, which scale factor.
///
/// Capability methods default to **no**: a host that has not thought about
/// snapshotting, focus or cropping is assumed not to do it, and the host
/// degrades the composition rather than shipping a wrong frame. That matches
/// [`ClipSupport`]'s own default and is the reason declaring a capability has
/// to be deliberate.
pub trait NativeViewHost {
    /// Extent of the view participants are added to, in logical units.
    ///
    /// The **parent view's**, not the window's — see the module docs.
    fn parent_extent(&self) -> Size;

    /// Logical-to-device scale, the same number the host puts in
    /// [`NativeState::scale`].
    fn scale(&self) -> f64;

    /// Whether this host crops for the framework. See [`ClipSupport`].
    fn clip_support(&self) -> ClipSupport {
        ClipSupport::Unsupported
    }

    /// Which input modes the native views honour.
    fn input_modes(&self) -> InputModes {
        InputModes::PASSTHROUGH_ONLY
    }

    /// Whether a participant can be frozen into pixels for a transition.
    fn can_snapshot(&self) -> bool {
        false
    }

    /// Whether participants join the OS focus chain (and so take IME
    /// ownership).
    fn can_focus(&self) -> bool {
        false
    }

    /// Create the native view of `kind` as a child of the parent view.
    fn create_view(&mut self, slot: NativeSlot, kind: NativeViewKind) -> Result<(), NativeError>;

    /// Tear the view down. Must tolerate a slot it never created.
    fn destroy_view(&mut self, slot: NativeSlot);

    /// Apply a placement: where the view goes, what part of it shows, and
    /// whether it takes the pointer. Called only when the placement changed.
    fn place_view(&mut self, slot: NativeSlot, placement: ViewPlacement);

    /// Show or hide. Hiding must not be implemented as a move.
    fn set_view_visible(&mut self, slot: NativeSlot, visible: bool);

    /// Give up / take back OS focus for this participant.
    fn set_view_focus(&mut self, slot: NativeSlot, focused: bool);

    /// Freeze the view into a texture. `None` when unsupported.
    fn snapshot_view(&mut self, slot: NativeSlot) -> Option<NativeSnapshot> {
        let _ = slot;
        None
    }

    /// Hand the live view back.
    fn restore_view(&mut self, slot: NativeSlot) {
        let _ = slot;
    }

    /// Drain any snapshots taken since the last call, so the app runner can
    /// hand their pixels to the renderer.
    ///
    /// A host that has not implemented snapshotting (`can_snapshot() == false`)
    /// never has any, so the default is empty and the bridge stays silent. A
    /// host that *can* snapshot but has not been asked to (no `snapshot_view`
    /// call drained yet) also returns empty — the queue only fills when a
    /// participant is actually frozen.
    fn take_captures(&mut self) -> Vec<NativeCapture> {
        let _ = self;
        Vec::new()
    }
}

/// What the bridge last told the OS about one slot.
///
/// The parent extent is part of the key, not just the state, even though the
/// bridge itself no longer looks at it. A resize changes where a participant
/// goes — a top-left-pinned view keeps its rect while its mirrored y moves by
/// the whole delta — and the backend cannot know that on its own, because the
/// only thing that ever prompts it to act is this call. So the parent extent
/// has to be *in* the key for the memo to notice, or the resize is skipped
/// exactly when it matters. Cheaper to compare two numbers than to be wrong
/// once per resize.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Placed {
    state: NativeState,
    /// `None` when the participant was hidden, in which case nothing was
    /// placed and there is no extent to remember.
    parent: Option<Size>,
}

/// A [`NativeViewProvider`] over a [`NativeViewHost`].
///
/// Deliberately thin. The lifecycle invariants — `destroy` for every `create`,
/// a recycled slot re-bound before anything is applied to it, no double freeze,
/// a rect with a negative extent refused, a crop the host cannot do refused —
/// all live in [`floom_native::NativeViewController`], so this type has exactly
/// two jobs:
///
/// 1. translate a [`NativeState`] into a [`ViewPlacement`];
/// 2. remember the last translation, so repeating a state is free.
///
/// It does not re-check what the controller checked. Two places enforcing one
/// rule is two places to disagree.
#[derive(Debug)]
pub struct DesktopNativeBridge<H: NativeViewHost> {
    host: H,
    caps: NativeCapabilities,
    placed: HashMap<NativeSlot, Placed>,
}

/// A desktop subview's preference order: a real child view is the cheapest
/// thing and always available.
const OVERLAY_ONLY: &[NativeCompositionMode] = &[NativeCompositionMode::Overlay];

/// ...and a frozen snapshot is a texture the framework draws, which is worth
/// offering when the host can produce one — it is the only mode that lets the
/// framework crop on the participant's behalf.
const OVERLAY_THEN_TEXTURE: &[NativeCompositionMode] = &[
    NativeCompositionMode::Overlay,
    NativeCompositionMode::Texture,
];

impl<H: NativeViewHost> DesktopNativeBridge<H> {
    /// Wrap a host, deriving the capabilities from what the host admits to.
    ///
    /// Derived rather than passed in: a provider that publishes a capability it
    /// does not have is worse than one that publishes nothing, because the host
    /// negotiates on the strength of it and stops degrading.
    pub fn over(host: H) -> Self {
        let caps = NativeCapabilities {
            modes: if host.can_snapshot() {
                OVERLAY_THEN_TEXTURE
            } else {
                OVERLAY_ONLY
            },
            snapshot: host.can_snapshot(),
            input: host.input_modes(),
            focusable: host.can_focus(),
            clip: host.clip_support(),
        };
        Self {
            host,
            caps,
            placed: HashMap::new(),
        }
    }

    /// The platform capabilities a session with this bridge installed must
    /// report.
    ///
    /// See [`desktop_capabilities`] for why this is not something the caller
    /// assembles by hand.
    pub fn platform_capabilities(&self) -> PlatformCapabilities {
        desktop_capabilities(true)
    }

    pub fn host(&self) -> &H {
        &self.host
    }

    pub fn host_mut(&mut self) -> &mut H {
        &mut self.host
    }
}

impl<H: NativeViewHost> NativeViewProvider for DesktopNativeBridge<H> {
    fn capabilities(&self) -> NativeCapabilities {
        self.caps
    }

    fn create(&mut self, slot: NativeSlot, kind: NativeViewKind) -> Result<(), NativeError> {
        // No memo to clear here, and that is worth stating because the obvious
        // move is to clear one. A stale entry cannot survive into a create:
        // a slot the pool *recycled* comes back with a bumped generation, which
        // is a different key, and a slot the controller *re-bound* went through
        // `destroy` on the way here, which is where the memo is dropped. The
        // one way to get here with a warm memo and the same key is a second
        // `create` on a bound slot, which the controller rejects.
        self.host.create_view(slot, kind)
    }

    fn destroy(&mut self, slot: NativeSlot) {
        // Load-bearing, not hygiene: `NativeViewController::rebind` destroys
        // and re-creates *the same slot at the same generation* when a
        // participant changes composition mode (a view that cannot crop is
        // moved to a mode the framework composites). Keeping the memo across
        // that would suppress the new view's first placement, and the
        // participant would simply never reappear.
        self.placed.remove(&slot);
        self.host.destroy_view(slot);
    }

    fn apply(&mut self, slot: NativeSlot, state: NativeState) {
        if !state.visible {
            // Hide where it stands; do not move it. The rect in a hidden state
            // is not meaningful — the host sets `visible: false` for a
            // scrolled-away participant rather than inventing a rect outside
            // the viewport — and a hidden view with a nonsensical frame is a
            // rejected `SetWindowPos` on some platforms and a flash of content
            // at the old position on others.
            let key = Placed {
                state,
                parent: None,
            };
            if self.placed.get(&slot) == Some(&key) {
                return;
            }
            self.host.set_view_visible(slot, false);
            self.placed.insert(slot, key);
            return;
        }
        let parent = self.host.parent_extent();
        let key = Placed {
            state,
            parent: Some(parent),
        };
        if self.placed.get(&slot) == Some(&key) {
            return;
        }
        self.host.place_view(slot, to_view_placement(state));
        self.host.set_view_visible(slot, true);
        self.placed.insert(slot, key);
    }

    fn focus(&mut self, slot: NativeSlot) {
        self.host.set_view_focus(slot, true);
    }

    fn blur(&mut self, slot: NativeSlot) {
        self.host.set_view_focus(slot, false);
    }

    fn snapshot(&mut self, slot: NativeSlot) -> Option<NativeSnapshot> {
        self.host.snapshot_view(slot)
    }

    fn restore(&mut self, slot: NativeSlot) {
        self.host.restore_view(slot);
    }

    fn take_captures(&mut self) -> Vec<NativeCapture> {
        self.host.take_captures()
    }
}

/// The desktop platform's capabilities for a session that did or did not
/// install a native-view bridge.
///
/// One function rather than a `desktop_default` plus a flag set by hand at the
/// call site, because the flag is a **promise**: advertising
/// [`Capability::NATIVE_VIEW`] with no provider makes every `NativeSurface`
/// mount fail with "no mode this participant accepts", and installing a
/// provider without the flag makes widgets refuse to try at all. Deriving the
/// flag from the bridge's presence is the only arrangement in which the two
/// cannot drift — so the OS half of step 7 flips the argument, and nothing else
/// in this crate has to be touched when it lands.
pub fn desktop_capabilities(native_view: bool) -> PlatformCapabilities {
    let mut caps = PlatformCapabilities::desktop_default();
    if native_view {
        caps.caps |= Capability::NATIVE_VIEW;
    }
    caps
}

#[cfg(test)]
mod tests {
    use super::*;
    use floom_native::NativeViewId;
    use floom_types::{InputMode, Rect};

    fn slot(index: u32) -> NativeSlot {
        NativeSlot {
            id: NativeViewId::new(index, 1),
            generation: 1,
        }
    }

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
        Rect { x0, y0, x1, y1 }
    }

    fn state(rect: Rect, scale: f64) -> NativeState {
        NativeState {
            rect,
            clip: None,
            scale,
            visible: true,
            z: 0,
            input: InputMode::Passthrough,
            mode: NativeCompositionMode::Overlay,
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    enum Call {
        Create(NativeSlot, NativeViewKind),
        Destroy(NativeSlot),
        Place(NativeSlot, ViewPlacement),
        Visible(NativeSlot, bool),
        Focus(NativeSlot),
        Blur(NativeSlot),
    }

    #[derive(Debug)]
    struct RecordingHost {
        parent: Size,
        scale: f64,
        clip: ClipSupport,
        snapshot: bool,
        calls: Vec<Call>,
    }

    impl RecordingHost {
        fn new(parent: Size, scale: f64) -> Self {
            Self {
                parent,
                scale,
                clip: ClipSupport::Platform,
                snapshot: false,
                calls: Vec::new(),
            }
        }

        /// Every placement the bridge asked for, in order.
        fn places(&self) -> Vec<ViewPlacement> {
            self.calls
                .iter()
                .filter_map(|c| match c {
                    Call::Place(_, placement) => Some(*placement),
                    _ => None,
                })
                .collect()
        }
    }

    impl NativeViewHost for RecordingHost {
        fn parent_extent(&self) -> Size {
            self.parent
        }

        fn scale(&self) -> f64 {
            self.scale
        }

        fn clip_support(&self) -> ClipSupport {
            self.clip
        }

        fn can_snapshot(&self) -> bool {
            self.snapshot
        }

        fn create_view(
            &mut self,
            slot: NativeSlot,
            kind: NativeViewKind,
        ) -> Result<(), NativeError> {
            self.calls.push(Call::Create(slot, kind));
            Ok(())
        }

        fn destroy_view(&mut self, slot: NativeSlot) {
            self.calls.push(Call::Destroy(slot));
        }

        fn place_view(&mut self, slot: NativeSlot, placement: ViewPlacement) {
            self.calls.push(Call::Place(slot, placement));
        }

        fn set_view_visible(&mut self, slot: NativeSlot, visible: bool) {
            self.calls.push(Call::Visible(slot, visible));
        }

        fn set_view_focus(&mut self, slot: NativeSlot, focused: bool) {
            self.calls.push(if focused {
                Call::Focus(slot)
            } else {
                Call::Blur(slot)
            });
        }
    }

    /// The case that separates "snap then mirror" from "mirror then snap".
    ///
    /// Logical `y 0.75..1.25` at 2x snaps to device rows `2..3`, whose mirror in
    /// a 10-unit-tall parent (20 device rows) is `y_os = 17`. Mirroring in
    /// logical units first gives `8.75..9.25`, which snaps to rows `18..19`,
    /// `y_os = 18` — one pixel away from where the framework painted the same
    /// box.
    ///
    /// Two statements, because the split is the fix: the placement is the
    /// framework's own box, and only the OS frame is mirrored. [`os_frame`]
    /// takes a [`DeviceRect`], so the wrong order is unreachable from it — the
    /// test is what says the structure is right, not what makes it right.
    #[test]
    fn the_mirror_happens_after_the_snap() {
        let placement = to_view_placement(state(rect(0.0, 0.75, 10.0, 1.25), 2.0));
        assert_eq!(
            placement.frame,
            DeviceRect {
                x: 0,
                y: 2,
                width: 20,
                height: 1
            },
            "the placement is the box the framework snapped"
        );
        assert_eq!(
            os_frame(
                placement.frame,
                Size::new(100.0, 10.0),
                2.0,
                Origin::BottomLeft
            ),
            DeviceRect {
                x: 0,
                y: 17,
                width: 20,
                height: 1
            },
            "mirroring before the snap would place this at y_os = 18"
        );
    }

    /// A backend whose origin is already top-left converts nothing.
    ///
    /// The point of the assertion is that the conversion is a *choice* the
    /// backend makes and not something the shared layer did on its way past: a
    /// Win32 or X11 backend that places [`ViewPlacement::frame`] unread is
    /// correct, and it would not be if the framework had mirrored it for them.
    #[test]
    fn a_top_left_host_gets_the_framework_box_unchanged() {
        let placement = to_view_placement(state(rect(0.0, 1.0, 5.0, 2.0), 2.0));

        for parent in [Size::new(100.0, 4.0), Size::new(100.0, 40.0)] {
            assert_eq!(
                os_frame(placement.frame, parent, 2.0, Origin::TopLeft),
                placement.frame,
                "a top-left host mirrors nothing, whatever the parent extent is"
            );
        }
    }

    /// The invariant behind the test above, at every scale a desktop actually
    /// runs at: mirroring an OS frame back must reproduce the rect the framework
    /// snapped. If it does not, the OS view and the scene disagree about which
    /// pixels the participant occupies, and the difference shows up as a
    /// hairline between it and the content next to it.
    ///
    /// The second half is the same statement about two participants: stacked
    /// boxes must stay stacked, with no row claimed twice and none skipped.
    /// Adjacency surviving the mirror is not obvious — the mirror moves each
    /// box's *top* edge by its own height as well — so it is asserted rather
    /// than assumed.
    #[test]
    fn an_os_frame_round_trips_and_neighbours_stay_adjacent() {
        let parent = Size::new(100.0, 3.0);
        for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
            let above = state(rect(0.0, 0.5, 10.0, 1.5), scale);
            let below = state(rect(0.0, 1.5, 10.0, 2.5), scale);
            let parent_height = device_parent_height(parent, scale);

            let a = os_frame(
                to_view_placement(above).frame,
                parent,
                scale,
                Origin::BottomLeft,
            );
            let b = os_frame(
                to_view_placement(below).frame,
                parent,
                scale,
                Origin::BottomLeft,
            );

            assert_eq!(
                mirror_device(a, parent_height),
                above.device_rect(),
                "at {scale}x the OS frame does not mirror back to the snapped rect"
            );
            assert_eq!(
                mirror_device(b, parent_height),
                below.device_rect(),
                "at {scale}x the OS frame does not mirror back to the snapped rect"
            );
            assert_eq!(
                a.y,
                b.y + b.height as i32,
                "at {scale}x the two neighbours are not adjacent (gap or overlap)"
            );
        }
    }

    /// The height to mirror against is the parent view's, not the window's.
    ///
    /// The window is 10 logical units tall; the group the participant was added
    /// to is 4. Mirroring against the window would put it at `y_os = 8` instead
    /// of `2` — and the two are the same number for a participant added straight
    /// to the content view, which is why the difference is easy to miss.
    #[test]
    fn the_flip_is_measured_against_the_parent_view_not_the_window() {
        let placement = to_view_placement(state(rect(0.0, 1.0, 5.0, 2.0), 1.0));
        assert_eq!(placement.frame.y, 1);

        let against_parent = os_frame(
            placement.frame,
            Size::new(100.0, 4.0),
            1.0,
            Origin::BottomLeft,
        );
        assert_eq!(against_parent.y, 2);
        assert_eq!(against_parent.height, 1);

        let against_window = os_frame(
            placement.frame,
            Size::new(100.0, 10.0),
            1.0,
            Origin::BottomLeft,
        );
        assert_eq!(
            against_window.y, 8,
            "the wrong parent extent is what this number would be"
        );
    }

    /// A crop becomes a container and an offset, never a smaller participant.
    ///
    /// The distinction is the whole `clip` channel: a backend handed the crop as
    /// a frame would fit the participant to it and **rescale** its content. So
    /// the assertion is that the content keeps its full size and is pushed out
    /// of the container by exactly the crop's origin — which is the same
    /// statement as "show that part of it", in the form an OS can act on.
    #[test]
    fn a_crop_becomes_a_container_and_an_offset_not_a_smaller_view() {
        let mut clipped = state(rect(0.0, 1.0, 10.0, 3.0), 1.0);
        clipped.clip = Some(rect(2.0, 1.5, 8.0, 2.5));

        let placement = to_view_placement(clipped);
        let frames = split_placement(placement);
        let crop = placement.crop.unwrap();

        assert_eq!(
            frames.container, crop,
            "the container is the visible window onto the participant"
        );
        assert_eq!(
            frames.content.width, placement.frame.width,
            "a crop must not resize the participant"
        );
        assert_eq!(frames.content.height, placement.frame.height);
        assert_eq!(
            frames.content.x,
            placement.frame.x - crop.x,
            "the participant slides by the difference, not to zero"
        );
        assert_eq!(frames.content.y, placement.frame.y - crop.y);
    }

    /// No crop is not a special case: it is the crop that happens to be the
    /// whole participant.
    ///
    /// Worth pinning because the tempting implementation creates the container
    /// only when there is a crop, which leaves the clipping path as the one that
    /// only clipped participants reach — the path nobody exercises until the
    /// day it is wrong.
    #[test]
    fn an_uncropped_participant_goes_through_the_same_split() {
        let placement = to_view_placement(state(rect(3.0, 4.0, 8.0, 9.0), 1.0));
        assert_eq!(placement.crop, None);

        let frames = split_placement(placement);

        assert_eq!(frames.container, placement.frame);
        assert_eq!(
            frames.content,
            DeviceRect {
                x: 0,
                y: 0,
                width: 5,
                height: 5
            },
            "the participant fills its own container, at the container's origin"
        );
    }

    /// Points are what AppKit frames are in, and the division is why the snap
    /// survives the trip.
    ///
    /// Snapping to whole *points* would have thrown away the sub-point alignment
    /// that [`to_device`] exists to keep; dividing a whole number of *pixels*
    /// and letting the host multiply back by the backing scale factor recovers
    /// the same pixel. The assertion is that round trip, not the division.
    #[test]
    fn device_pixels_convert_to_points_and_back_to_the_same_pixel() {
        for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
            let rect = DeviceRect {
                x: 3,
                y: 17,
                width: 20,
                height: 1,
            };
            let (x, y, width, height) = device_to_points(rect, scale);

            assert_eq!((x * scale).round() as i32, rect.x, "at {scale}x");
            assert_eq!((y * scale).round() as i32, rect.y, "at {scale}x");
            assert_eq!((width * scale).round() as u32, rect.width, "at {scale}x");
            assert_eq!((height * scale).round() as u32, rect.height, "at {scale}x");
        }
    }

    #[test]
    fn repeating_a_state_does_not_touch_the_os() {
        let mut bridge = DesktopNativeBridge::over(RecordingHost::new(Size::new(100.0, 10.0), 1.0));
        let slot = slot(0);
        let one = state(rect(0.0, 1.0, 10.0, 3.0), 1.0);

        bridge.apply(slot, one);
        bridge.apply(slot, one);
        bridge.apply(slot, one);

        assert_eq!(bridge.host().places().len(), 1);
    }

    /// The memo keys on the parent extent even though the bridge no longer
    /// computes anything from it.
    ///
    /// The backend's *output* does depend on it — that is the mirror — and this
    /// call is the only thing that ever makes the backend look again, so a key
    /// that dropped the extent would skip the re-placement exactly when it
    /// matters. The placement handed over is byte-identical in both cases, which
    /// is why the assertion is on the number of calls and not on their contents.
    #[test]
    fn a_resize_re_places_a_participant_whose_rect_did_not_change() {
        let mut bridge = DesktopNativeBridge::over(RecordingHost::new(Size::new(100.0, 4.0), 1.0));
        let slot = slot(0);
        // Pinned to the top of the parent: the state never changes, but what the
        // backend has to draw from it does.
        let pinned = state(rect(0.0, 0.0, 10.0, 1.0), 1.0);

        bridge.apply(slot, pinned);
        bridge.host_mut().parent = Size::new(100.0, 8.0);
        bridge.apply(slot, pinned);

        let places = bridge.host().places();
        assert_eq!(places.len(), 2, "the resize was swallowed by the memo");
        assert_eq!(
            places[0], places[1],
            "the placement is the same; the OS frame the backend makes from it is not"
        );
    }

    #[test]
    fn a_hidden_participant_is_hidden_where_it_stands_not_moved() {
        let mut bridge = DesktopNativeBridge::over(RecordingHost::new(Size::new(100.0, 10.0), 1.0));
        let slot = slot(0);
        let mut hidden = state(rect(0.0, 1.0, 10.0, 3.0), 1.0);
        hidden.visible = false;

        bridge.apply(slot, hidden);

        assert_eq!(bridge.host().calls, vec![Call::Visible(slot, false)]);
    }

    #[test]
    fn a_crop_is_carried_with_the_frame_and_stays_inside_it() {
        let mut clipped = state(rect(0.0, 1.0, 10.0, 3.0), 1.0);
        clipped.clip = Some(rect(2.0, 1.5, 8.0, 2.5));

        let placement = to_view_placement(clipped);

        assert_eq!(
            placement.frame,
            DeviceRect {
                x: 0,
                y: 1,
                width: 10,
                height: 2
            }
        );
        assert_eq!(
            placement.crop,
            Some(DeviceRect {
                x: 2,
                y: 2,
                width: 6,
                height: 1
            })
        );
        let crop = placement.crop.unwrap();
        assert!(
            crop.x >= placement.frame.x
                && crop.y >= placement.frame.y
                && crop.x + crop.width as i32 <= placement.frame.x + placement.frame.width as i32
                && crop.y + crop.height as i32 <= placement.frame.y + placement.frame.height as i32,
            "the crop escapes its own frame"
        );
    }

    /// The input mode travels with the placement, because on a backend that has
    /// to do something about it the two are one decision.
    ///
    /// AppKit answers "does this take the pointer" with `hitTest:`, which is
    /// part of placing the view — so a backend that received the placement and
    /// not the mode could only guess. Pinned here rather than trusted, because
    /// the failure is one-directional and quiet: every participant would take
    /// the pointer, including the ones that exist precisely so they do not.
    #[test]
    fn the_input_mode_is_carried_into_the_placement() {
        let mut capturing = state(rect(0.0, 1.0, 10.0, 3.0), 1.0);
        capturing.input = InputMode::Capture;
        assert_eq!(
            to_view_placement(capturing).input,
            InputMode::Capture,
            "a participant that takes input must be placed as one"
        );

        let passthrough = state(rect(0.0, 1.0, 10.0, 3.0), 1.0);
        assert_eq!(
            to_view_placement(passthrough).input,
            InputMode::Passthrough,
            "and one that does not must not be"
        );
    }

    /// A crop shows a window onto the participant, so the participant has to
    /// cover the whole window.
    ///
    /// Stated separately from the offsets because it is the property that makes
    /// them right: the container is the crop's box, and the participant is
    /// placed relative to it, so if the participant did not cover it the visible
    /// result would be a blank edge *inside* the crop — which looks like a
    /// layout bug somewhere else entirely.
    #[test]
    fn the_cropped_participant_covers_the_whole_container() {
        let mut clipped = state(rect(0.0, 1.0, 10.0, 3.0), 1.0);
        clipped.clip = Some(rect(2.0, 1.5, 8.0, 2.5));

        let frames = split_placement(to_view_placement(clipped));

        assert!(
            frames.content.x <= 0 && frames.content.y <= 0,
            "the participant starts at or before the window's origin"
        );
        assert!(
            frames.content.x + frames.content.width as i32 >= frames.container.width as i32
                && frames.content.y + frames.content.height as i32
                    >= frames.container.height as i32,
            "and reaches past its far edge, so no part of the window is empty"
        );
    }

    #[test]
    fn a_clip_that_contains_the_participant_is_not_sent_as_a_crop() {
        let mut clipped = state(rect(0.0, 1.0, 10.0, 3.0), 1.0);
        clipped.clip = Some(rect(-1.0, 0.0, 11.0, 4.0));

        let placement = to_view_placement(clipped);

        assert_eq!(
            placement.crop, None,
            "a clip that does not bite is not a crop"
        );
    }

    #[test]
    fn the_capabilities_are_derived_from_the_host() {
        let plain = DesktopNativeBridge::over(RecordingHost::new(Size::new(100.0, 10.0), 1.0));
        assert_eq!(plain.capabilities().clip, ClipSupport::Platform);
        assert_eq!(plain.capabilities().modes, OVERLAY_ONLY);
        assert!(!plain.capabilities().snapshot);
        assert!(
            !plain.capabilities().focusable,
            "an undeclared capability must default to no, not yes"
        );

        let mut host = RecordingHost::new(Size::new(100.0, 10.0), 1.0);
        host.snapshot = true;
        host.clip = ClipSupport::Unsupported;
        let rich = DesktopNativeBridge::over(host);
        assert_eq!(rich.capabilities().clip, ClipSupport::Unsupported);
        assert_eq!(rich.capabilities().modes, OVERLAY_THEN_TEXTURE);
        assert!(rich.capabilities().snapshot);
    }

    /// The flag and the provider are one decision, not two.
    #[test]
    fn the_native_view_flag_is_exactly_the_bridge() {
        assert!(
            !desktop_capabilities(false).supports(Capability::NATIVE_VIEW),
            "a session with no bridge must not promise one"
        );
        assert!(
            desktop_capabilities(true).supports(Capability::NATIVE_VIEW),
            "a session with a bridge must advertise it"
        );
        assert_eq!(
            desktop_capabilities(false),
            PlatformCapabilities::desktop_default(),
            "the bridge is the only thing this flag may be keyed on"
        );

        let bridge = DesktopNativeBridge::over(RecordingHost::new(Size::new(100.0, 10.0), 1.0));
        assert!(
            bridge
                .platform_capabilities()
                .supports(Capability::NATIVE_VIEW)
        );
    }

    /// The shape of `NativeViewController::rebind`: same slot, same generation,
    /// a new view on it. The memo has to be dropped by `destroy` or the
    /// replacement is never placed — and because the state is unchanged, the
    /// suppression is total: the participant does not appear at all.
    #[test]
    fn a_re_bound_slot_is_placed_again() {
        let mut bridge = DesktopNativeBridge::over(RecordingHost::new(Size::new(100.0, 10.0), 1.0));
        let slot = slot(0);
        let one = state(rect(0.0, 1.0, 10.0, 3.0), 1.0);

        bridge.create(slot, NativeViewKind::Map).unwrap();
        bridge.apply(slot, one);
        // `rebind`'s two halves, on the slot it was given.
        bridge.destroy(slot);
        bridge.create(slot, NativeViewKind::Map).unwrap();
        bridge.apply(slot, one);

        assert_eq!(
            bridge.host().places().len(),
            2,
            "a re-bound slot must be placed again, not assumed to be where its predecessor was"
        );
    }

    #[test]
    fn focus_and_blur_reach_the_host() {
        // Exclusivity and the `focusable` gate belong to the controller; the
        // bridge's job is to not drop the call on the floor.
        let mut bridge = DesktopNativeBridge::over(RecordingHost::new(Size::new(100.0, 10.0), 1.0));
        let slot = slot(0);

        bridge.focus(slot);
        bridge.blur(slot);

        assert_eq!(
            bridge.host().calls,
            vec![Call::Focus(slot), Call::Blur(slot)]
        );
    }
}
