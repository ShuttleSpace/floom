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
//! # The origin flip is the part that is wrong first
//!
//! `NativeState::rect` is top-left logical, in window coordinates. AppKit's
//! default origin is **bottom-left** and `NSView::isFlipped` is `false`, so a
//! frame handed to `setFrame:` measures y up from the *parent's* bottom edge.
//! (Win32 and X11 are top-left; they can flip nothing and place
//! [`ViewPlacement::frame`] as it comes.)
//!
//! The conversion that suggests itself — mirror in logical units, then snap to
//! device pixels — is wrong, and wrong in a way that survives a demo. Snapping
//! rounds each edge independently, so mirroring *around* the snap and snapping
//! *after* the mirror are not the same function: at 2x, a participant at
//! logical `y 0.75..1.25` of a parent 10 units tall belongs on device row 2
//! (so `y_os = 17`), while mirroring first lands it on row 3 mirrored to
//! `y_os = 18`. One pixel, and it is one pixel *of disagreement with the
//! framework*, which is what a seam is: the same box painted by the scene and
//! placed by the OS must be the same pixels, or adjacent content shows a
//! hairline at some (not all) scales — the failure that is invisible on the
//! developer's 2x display and reported by one user at 1.5x.
//!
//! So: **snap once, in [`floem_native::to_device`], and mirror in device
//! space.** `mirror_device` is an involution, which makes the invariant
//! checkable — mirroring a placement back must reproduce the very rect the
//! framework snapped, and `the_mirror_happens_after_the_snap` asserts exactly
//! that.
//!
//! The other half of the same bug: the height to mirror against is the
//! **parent view's**, not the window's. They are the same number for a
//! participant added straight to the content view and differ the moment one
//! goes into a container, which is precisely the case nobody has on day one.

use std::collections::HashMap;

use floem_native::{
    ClipSupport, DeviceRect, InputModes, NativeCapabilities, NativeCompositionMode, NativeError,
    NativeSlot, NativeSnapshot, NativeState, NativeViewKind, NativeViewProvider, to_device,
};
use floem_platform::{Capability, PlatformCapabilities};
use floem_types::Size;

/// A participant's placement in the host's own device space.
///
/// Produced by [`to_view_placement`] and consumed by the OS half. `frame` is
/// already mirrored, if the host's origin is bottom-left, and already snapped
/// to whole device pixels, so the OS call is a copy and not a computation.
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
}

/// Mirror a device rect about a bottom-left origin.
///
/// `parent_device_height` is the height of the **parent view** in device
/// pixels. Total — an involution on the rects it is given, which is what makes
/// "snap then mirror" verifiable: mirroring a placement back reproduces the
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
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    (parent.height * scale).round() as i32
}

/// Where a participant goes, given the state the host pushed and the view it is
/// added to.
///
/// Both conversions happen here and nowhere else — the host and the provider
/// see only whole device pixels, so there is exactly one rounding in the chain
/// (`to_device`'s own module docs say why that matters).
pub fn to_view_placement(state: NativeState, parent: Size) -> ViewPlacement {
    let parent_height = device_parent_height(parent, state.scale);
    let frame = mirror_device(state.device_rect(), parent_height);
    // The crop is snapped and mirrored by the same two calls as the frame, so
    // it stays a sub-rect of it: `visible_rect` is an intersection of `rect`,
    // and both `to_device` and `mirror_device` are monotone in the edges they
    // touch. A crop computed any other way could fall outside its own frame,
    // which is how a clipped participant ends up showing a slice of the wrong
    // content.
    let crop = state
        .needs_crop()
        .then(|| mirror_device(to_device(state.visible_rect(), state.scale), parent_height));
    ViewPlacement { frame, crop }
}

/// The OS calls the bridge needs, so that everything above them can be tested
/// without a window.
///
/// Every method is a *copy*, not a computation: the frame arrives mirrored and
/// snapped. An implementation translates #`ViewPlacement` into whatever the OS
/// calls it (`setFrame:`, `SetWindowPos`, `XConfigureWindow`) and nothing more.
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

    /// Apply a placement. Called only when the placement changed.
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
}

/// What the bridge last told the OS about one slot.
///
/// The parent extent is part of the key, not just the state. A window resize
/// changes where a participant goes even when the state it was given did not
/// change — a top-left-pinned view keeps its rect while its mirrored y moves by
/// the whole delta — so a memo keyed on the state alone would skip exactly the
/// re-placement the resize made necessary. Cheaper to compare two numbers than
/// to be wrong once per resize.
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
/// all live in [`floem_native::NativeViewController`], so this type has exactly
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

    /// Unwrap the host, dropping the placement memo.
    pub fn into_host(self) -> H {
        self.host
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
        self.host.place_view(slot, to_view_placement(state, parent));
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
    use floem_native::NativeViewId;
    use floem_types::{InputMode, Rect};

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
    #[test]
    fn the_mirror_happens_after_the_snap() {
        let placement = to_view_placement(
            state(rect(0.0, 0.75, 10.0, 1.25), 2.0),
            Size::new(100.0, 10.0),
        );
        assert_eq!(
            placement.frame,
            DeviceRect {
                x: 0,
                y: 17,
                width: 20,
                height: 1
            },
            "mirroring before the snap would place this at y_os = 18"
        );
    }

    /// The invariant behind the test above, at every scale a desktop actually
    /// runs at: mirroring a placement back must reproduce the rect the
    /// framework snapped. If it does not, the OS view and the scene disagree
    /// about which pixels the participant occupies, and the difference shows up
    /// as a hairline between it and the content next to it.
    ///
    /// The second half is the same statement about two participants: stacked
    /// boxes must stay stacked, with no row claimed twice and none skipped.
    #[test]
    fn a_placement_round_trips_and_neighbours_stay_adjacent() {
        let parent = Size::new(100.0, 3.0);
        for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
            let above = state(rect(0.0, 0.5, 10.0, 1.5), scale);
            let below = state(rect(0.0, 1.5, 10.0, 2.5), scale);
            let parent_height = device_parent_height(parent, scale);

            let a = to_view_placement(above, parent);
            let b = to_view_placement(below, parent);

            assert_eq!(
                mirror_device(a.frame, parent_height),
                above.device_rect(),
                "at {scale}x the placement does not mirror back to the snapped rect"
            );
            assert_eq!(
                mirror_device(b.frame, parent_height),
                below.device_rect(),
                "at {scale}x the placement does not mirror back to the snapped rect"
            );
            assert_eq!(
                a.frame.y,
                b.frame.y + b.frame.height as i32,
                "at {scale}x the two neighbours are not adjacent (gap or overlap)"
            );
        }
    }

    #[test]
    fn a_participant_is_placed_against_the_parent_view_not_the_window() {
        // The window is 10 logical units tall; the group the participant was
        // added to is 4. Mirroring against the window would put it at y_os = 8.
        let placement =
            to_view_placement(state(rect(0.0, 1.0, 5.0, 2.0), 1.0), Size::new(100.0, 4.0));
        assert_eq!(placement.frame.y, 2);
        assert_eq!(placement.frame.height, 1);
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

    #[test]
    fn a_resize_re_places_a_participant_whose_rect_did_not_change() {
        let mut bridge = DesktopNativeBridge::over(RecordingHost::new(Size::new(100.0, 4.0), 1.0));
        let slot = slot(0);
        // Pinned to the top of the parent: the state never changes, but the
        // mirror of it does.
        let pinned = state(rect(0.0, 0.0, 10.0, 1.0), 1.0);

        bridge.apply(slot, pinned);
        bridge.host_mut().parent = Size::new(100.0, 8.0);
        bridge.apply(slot, pinned);

        let places = bridge.host().places();
        assert_eq!(places.len(), 2, "the resize was swallowed by the memo");
        assert_eq!(places[0].frame.y, 3);
        assert_eq!(places[1].frame.y, 7);
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

        let placement = to_view_placement(clipped, Size::new(100.0, 10.0));

        assert_eq!(
            placement.frame,
            DeviceRect {
                x: 0,
                y: 7,
                width: 10,
                height: 2
            }
        );
        assert_eq!(
            placement.crop,
            Some(DeviceRect {
                x: 2,
                y: 7,
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

    #[test]
    fn a_clip_that_contains_the_participant_is_not_sent_as_a_crop() {
        let mut clipped = state(rect(0.0, 1.0, 10.0, 3.0), 1.0);
        clipped.clip = Some(rect(-1.0, 0.0, 11.0, 4.0));

        let placement = to_view_placement(clipped, Size::new(100.0, 10.0));

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
