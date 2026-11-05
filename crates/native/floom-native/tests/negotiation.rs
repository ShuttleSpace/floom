// SPDX-License-Identifier: MIT
//! Capability negotiation (Phase 16, task 16.3 step 1).
//!
//! The plan asks for the matrix — four composition modes against three
//! capability sets — because negotiation is the one part of native composition
//! that is *entirely* headless and *entirely* silent when it is wrong. A
//! participant granted a mode its backend cannot compose does not fail: it
//! draws nothing, or draws in the wrong place, and looks exactly like a
//! participant that was never asked for.
//!
//! So the assertions here are exhaustive rather than illustrative. The mode set
//! is small enough that "every combination" is cheap, and the two failure modes
//! being ruled out are:
//!
//! - granting a mode the provider does not offer (a blank region at paint
//!   time), and
//! - refusing a participant that could have been composited (a feature that
//!   silently does not work on one platform).
use std::borrow::Cow;

use floom_native::{
    ClipSupport, FakeNativeProvider, InputMode, InputModes, NativeCapabilities,
    NativeCompositionMode, NativeError, NativeState, NativeViewController, NativeViewKind,
    NativeViewPool, Rect,
};

const MODES: [NativeCompositionMode; 4] = [
    NativeCompositionMode::Overlay,
    NativeCompositionMode::Texture,
    NativeCompositionMode::Surface,
    NativeCompositionMode::Offscreen,
];

const EVERYTHING: &[NativeCompositionMode] = &MODES;
const OVERLAY_ONLY: &[NativeCompositionMode] = &[NativeCompositionMode::Overlay];
const NOTHING: &[NativeCompositionMode] = &[];

/// The three backend shapes the matrix runs against.
///
/// `EVERYTHING` is a desktop backend with a full toolkit; `OVERLAY_ONLY` is X11
/// or Win32, which can raise a subwindow above the surface and nothing else;
/// `NOTHING` is Wayland before its fallback exists, or any backend whose bridge
/// has not been written — device case D5's shape.
const CAPABILITY_SETS: [(&str, &[NativeCompositionMode]); 3] = [
    ("everything", EVERYTHING),
    ("overlay only", OVERLAY_ONLY),
    ("nothing", NOTHING),
];

fn caps(modes: &'static [NativeCompositionMode], clip: ClipSupport) -> NativeCapabilities {
    NativeCapabilities {
        modes: Cow::Borrowed(modes),
        snapshot: true,
        input: InputModes::ALL,
        focusable: true,
        clip,
    }
}

/// Every (mode, capability set) pair. This is the matrix.
#[test]
fn the_whole_mode_by_capability_matrix_agrees_with_itself() {
    for (name, offered) in CAPABILITY_SETS {
        for mode in MODES {
            let got = caps(offered, ClipSupport::Provider).negotiate(&[mode]);
            if offered.contains(&mode) {
                assert_eq!(
                    got,
                    Some(mode),
                    "{name} offers {mode:?}, so asking for exactly that must \
                     get it"
                );
            } else {
                assert_eq!(
                    got, None,
                    "{name} does not offer {mode:?}: granting it would be a \
                     blank region at paint time instead of a refusal at mount \
                     time"
                );
            }
        }
    }
}

/// A caller's preference order is honoured, and only then the provider's.
///
/// Two providers can offer the same set in different orders and legitimately
/// disagree: on KMS a plane is cheaper than a blit, on X11 a subwindow is
/// cheaper than a texture round-trip. The caller's list is what decides, so a
/// widget that must be occludable gets a mode it can be occluded in even when
/// the backend would rather do something else.
#[test]
fn the_callers_order_decides_and_the_providers_order_is_its_own_business() {
    let both = caps(
        &[
            NativeCompositionMode::Texture,
            NativeCompositionMode::Overlay,
        ],
        ClipSupport::Provider,
    );
    assert_eq!(
        both.preferred(),
        Some(NativeCompositionMode::Texture),
        "the provider publishes its own preference"
    );
    assert_eq!(
        both.negotiate(&[
            NativeCompositionMode::Overlay,
            NativeCompositionMode::Texture
        ]),
        Some(NativeCompositionMode::Overlay),
        "and the caller's preference is what gets picked"
    );
    assert_eq!(
        both.negotiate(&[
            NativeCompositionMode::Texture,
            NativeCompositionMode::Overlay
        ]),
        Some(NativeCompositionMode::Texture)
    );
}

/// When nothing works, it is an error at mount time — and the provider is not
/// left holding a view.
///
/// The alternative is what the phase exists to avoid: a participant that is
/// granted, appears in the registry, and draws nothing, which is
/// indistinguishable from one nobody asked for.
#[test]
fn a_participant_with_no_acceptable_mode_is_refused_before_anything_is_created() {
    let mut controller = NativeViewController::new(FakeNativeProvider::incapable());
    let mut pool = NativeViewPool::new();
    let slot = pool.acquire(0);

    assert_eq!(
        controller.bind(slot, NativeViewKind::Web, EVERYTHING),
        Err(NativeError::NoSupportedMode)
    );
    assert_eq!(
        controller.provider().command_count(),
        0,
        "no view may be created for a participant that cannot be composited"
    );
    assert_eq!(controller.bound_count(), 0);
    assert_eq!(controller.unreleased().count(), 0, "and nothing to leak");
}

/// `Texture` and `Offscreen` are the modes the *framework* draws, which is the
/// property the clip fallback in 16.2 leans on. Pin it here, because a future
/// fifth mode has to make the same decision consciously.
#[test]
fn the_modes_the_framework_draws_are_the_ones_it_can_crop() {
    for mode in MODES {
        let expected = matches!(
            mode,
            NativeCompositionMode::Texture | NativeCompositionMode::Offscreen
        );
        assert_eq!(
            mode.is_host_composited(),
            expected,
            "{mode:?}: Overlay and Surface are shown by the OS, so a crop is \
             theirs or nobody's; Texture and Offscreen put the pixels in our \
             hands"
        );
    }
}

/// The croppable fallback is the intersection of what the caller accepts and
/// what the framework draws — and `None` is a real answer.
#[test]
fn the_croppable_fallback_is_the_intersection_of_both_sides() {
    let caller = &[
        NativeCompositionMode::Overlay,
        NativeCompositionMode::Texture,
    ];
    assert_eq!(
        caps(EVERYTHING, ClipSupport::Unsupported).negotiate_croppable(caller),
        Some(NativeCompositionMode::Texture),
        "Overlay is the caller's first choice but the framework cannot crop it"
    );
    assert_eq!(
        caps(OVERLAY_ONLY, ClipSupport::Unsupported).negotiate_croppable(caller),
        None,
        "a backend that offers only Overlay has nothing to fall back to, and \
         saying so is the only honest answer"
    );
    assert_eq!(
        caps(EVERYTHING, ClipSupport::Platform).negotiate_croppable(caller),
        Some(NativeCompositionMode::Texture),
        "a backend that does not need the fallback still reports one: the \
         caller's list is what is being searched, not the provider's"
    );
}

/// Input is negotiated on the same footing as composition, and per mode of
/// input: a view can be transparent to the pointer while still being
/// composited, and that is a different statement from a view that takes it.
#[test]
fn input_support_is_a_set_the_caller_has_to_be_inside() {
    for (modes, passthrough, capture) in [
        (InputModes::ALL, true, true),
        (InputModes::PASSTHROUGH_ONLY, true, false),
        (InputModes::NONE, false, false),
    ] {
        assert_eq!(modes.supports(InputMode::Passthrough), passthrough);
        assert_eq!(modes.supports(InputMode::Capture), capture);
    }
    assert_eq!(
        InputModes::default(),
        InputModes::PASSTHROUGH_ONLY,
        "the default refuses to take the pointer: a view that did not ask for \
         input must not start swallowing it"
    );
}

/// A state carries the mode it was negotiated in, and the controller checks it
/// again on every frame — a provider whose capabilities changed under it (a
/// device lost, a compositor reconfigured) cannot be sent a mode it no longer
/// supports.
#[test]
fn a_mode_the_provider_stopped_supporting_is_refused_per_frame() {
    let mut controller = NativeViewController::new(FakeNativeProvider::with_capabilities(caps(
        OVERLAY_ONLY,
        ClipSupport::Platform,
    )));
    let mut pool = NativeViewPool::new();
    let slot = pool.acquire(0);
    let mode = controller
        .bind(slot, NativeViewKind::Video, OVERLAY_ONLY)
        .unwrap();
    assert_eq!(mode, NativeCompositionMode::Overlay);

    let state = NativeState {
        rect: Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 100.0,
            y1: 50.0,
        },
        clip: None,
        scale: 1.0,
        visible: true,
        z: 0,
        input: InputMode::Capture,
        mode: NativeCompositionMode::Offscreen,
    };
    assert_eq!(
        controller.apply(slot, state),
        Err(NativeError::UnsupportedMode(
            NativeCompositionMode::Offscreen
        ))
    );
}
