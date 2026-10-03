//! The provider contract (Phase 16, task 16.1).
//!
//! These are the invariants the plan says must hold no matter which platform
//! backend is behind the ABI. Each test names the failure it exists to prevent,
//! because the failure modes here are all *silent*: a leaked native view that
//! the framework can no longer name looks exactly like a working one in a
//! headless run, and only shows up as a stuck view on a device.
//!
//! Testing them against [`FakeNativeProvider`] is the point: it is what makes
//! 16.2 (diff, clipping, z-order, hit-test subtraction, focus yield) and 16.4
//! (snapshot transitions) assertable without a single OS bridge existing.

use floem_native::{
    ClipSupport, FakeNativeProvider, InputMode, InputModes, NativeCapabilities, NativeCommand,
    NativeCompositionMode, NativeError, NativeState, NativeViewController, NativeViewKind,
    NativeViewPool, Rect,
};

const OVERLAY: NativeCompositionMode = NativeCompositionMode::Overlay;
const TEXTURE: NativeCompositionMode = NativeCompositionMode::Texture;
const PREFER_OVERLAY: &[NativeCompositionMode] = &[OVERLAY];
const OVERLAY_ONLY: &[NativeCompositionMode] = &[OVERLAY];
const TEXTURE_ONLY: &[NativeCompositionMode] = &[TEXTURE];

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
    Rect { x0, y0, x1, y1 }
}

fn state(mode: NativeCompositionMode) -> NativeState {
    NativeState {
        rect: rect(0.0, 0.0, 100.0, 50.0),
        // Nothing clips it: the placement and the visible region are the same,
        // which is the common case and the one every other assertion assumes.
        clip: None,
        scale: 2.0,
        visible: true,
        z: 0,
        input: InputMode::Capture,
        mode,
    }
}

/// Slots come from the real pool, because the pool is what decides generations
/// and 16.2's recycling case depends on that being right.
fn slots() -> NativeViewPool<usize> {
    NativeViewPool::new()
}

fn controller() -> NativeViewController<FakeNativeProvider> {
    NativeViewController::new(FakeNativeProvider::new())
}

// --- lifecycle balance -----------------------------------------------------

#[test]
fn a_bound_participant_is_destroyed_exactly_once() {
    let mut c = controller();
    let mut pool = slots();
    let slot = pool.acquire(0);

    c.bind(slot, NativeViewKind::Video, PREFER_OVERLAY).unwrap();
    c.apply(slot, state(OVERLAY)).unwrap();
    c.destroy(slot).unwrap();

    let fake = c.provider();
    assert_eq!(fake.creates(), 1);
    assert_eq!(fake.destroys(), 1);
    assert_eq!(c.unreleased().count(), 0);
}

#[test]
fn destroying_an_unbound_slot_is_refused() {
    let mut c = controller();
    let mut pool = slots();
    let slot = pool.acquire(0);

    assert_eq!(c.destroy(slot), Err(NativeError::NotBound(slot)));
    assert_eq!(c.provider().command_count(), 0);
}

#[test]
fn binding_a_live_slot_twice_is_refused() {
    // Two creates on one slot leak a native view the framework can no longer
    // name: the second handle overwrites the first in the provider's map and
    // nothing will ever destroy it.
    let mut c = controller();
    let mut pool = slots();
    let slot = pool.acquire(0);

    c.bind(slot, NativeViewKind::Video, PREFER_OVERLAY).unwrap();
    assert_eq!(
        c.bind(slot, NativeViewKind::Web, PREFER_OVERLAY),
        Err(NativeError::AlreadyBound(slot))
    );
    assert_eq!(c.provider().creates(), 1);
}

#[test]
fn a_teardown_that_forgets_a_participant_is_visible() {
    let mut c = controller();
    let mut pool = slots();
    let a = pool.acquire(0);
    let b = pool.acquire(1);
    c.bind(a, NativeViewKind::Video, PREFER_OVERLAY).unwrap();
    c.bind(b, NativeViewKind::Map, PREFER_OVERLAY).unwrap();
    c.destroy(a).unwrap();

    let leaked: Vec<_> = c.unreleased().collect();
    assert_eq!(leaked, vec![b], "the undestroyed participant is named");
}

// --- recycling -------------------------------------------------------------

#[test]
fn a_recycled_slot_must_be_rebound_before_it_is_applied_to() {
    // The 16.2 scrolling case. Row 5's slot is released and handed to row 15;
    // the backend is still holding row 5's handle until it is told to re-bind.
    let mut c = controller();
    let mut pool = slots();
    let first = pool.acquire(0);
    c.bind(first, NativeViewKind::Video, PREFER_OVERLAY)
        .unwrap();
    c.destroy(first).unwrap();

    pool.release(&0);
    let recycled = pool.acquire(1);
    assert_eq!(recycled.id, first.id);
    assert_eq!(recycled.generation, first.generation + 1);

    assert_eq!(
        c.apply(recycled, state(OVERLAY)),
        Err(NativeError::NotBound(recycled)),
        "the recycled slot must not inherit the dead slot's binding"
    );
    assert_eq!(
        c.provider().applied().len(),
        0,
        "no state may reach the provider for an unbound slot"
    );

    c.bind(recycled, NativeViewKind::Video, PREFER_OVERLAY)
        .unwrap();
    c.apply(recycled, state(OVERLAY)).unwrap();
    assert_eq!(c.provider().applied().len(), 1);
}

#[test]
fn a_recycled_slot_may_be_rebound_to_a_different_kind() {
    let mut c = controller();
    let mut pool = slots();
    let first = pool.acquire(0);
    c.bind(first, NativeViewKind::Video, PREFER_OVERLAY)
        .unwrap();
    c.destroy(first).unwrap();

    pool.release(&0);
    let recycled = pool.acquire(1);
    c.bind(recycled, NativeViewKind::Map, PREFER_OVERLAY)
        .unwrap();

    let fake = c.provider();
    assert_eq!(fake.creates(), 2);
    assert_eq!(
        fake.commands().last(),
        Some(&NativeCommand::Create {
            slot: recycled,
            kind: NativeViewKind::Map
        })
    );
}

// --- geometry --------------------------------------------------------------

#[test]
fn an_inverted_rect_never_reaches_the_provider() {
    // A clip that produces an inverted rect is a host bug. Letting the provider
    // clamp it would place the view somewhere the host did not ask for.
    let mut c = controller();
    let mut pool = slots();
    let slot = pool.acquire(0);
    c.bind(slot, NativeViewKind::Video, PREFER_OVERLAY).unwrap();

    let bad = NativeState {
        rect: rect(100.0, 0.0, 40.0, 50.0),
        ..state(OVERLAY)
    };
    assert_eq!(c.apply(slot, bad), Err(NativeError::InvalidRect(bad.rect)));
    assert_eq!(c.provider().applied().len(), 0);
}

#[test]
fn a_degenerate_scale_never_reaches_the_provider() {
    let mut c = controller();
    let mut pool = slots();
    let slot = pool.acquire(0);
    c.bind(slot, NativeViewKind::Video, PREFER_OVERLAY).unwrap();

    for scale in [0.0, -1.0, f64::INFINITY] {
        let bad = NativeState {
            scale,
            ..state(OVERLAY)
        };
        assert_eq!(
            c.apply(slot, bad),
            Err(NativeError::InvalidScale(scale)),
            "scale {scale} must be rejected"
        );
    }
    // NaN cannot be compared with assert_eq (NaN != NaN), so match the shape.
    let nan = NativeState {
        scale: f64::NAN,
        ..state(OVERLAY)
    };
    assert!(
        matches!(c.apply(slot, nan), Err(NativeError::InvalidScale(s)) if s.is_nan()),
        "NaN scale must be rejected"
    );
    assert_eq!(c.provider().applied().len(), 0);
}

#[test]
fn a_zero_sized_but_valid_rect_is_allowed() {
    // A participant collapsed by a flex layout is legal; it is `visible: false`
    // territory, not an error.
    let mut c = controller();
    let mut pool = slots();
    let slot = pool.acquire(0);
    c.bind(slot, NativeViewKind::Video, PREFER_OVERLAY).unwrap();

    let collapsed = NativeState {
        rect: rect(10.0, 10.0, 10.0, 10.0),
        ..state(OVERLAY)
    };
    c.apply(slot, collapsed).unwrap();
    assert_eq!(c.provider().applied(), vec![(slot, collapsed)]);
}

#[test]
fn the_device_rect_is_the_same_no_matter_how_many_times_it_is_asked_for() {
    // Every provider must round identically or two adjacent participants gap or
    // overlap — device case D1's falsifier, in headless form.
    let s = state(OVERLAY);
    assert_eq!(s.device_rect(), s.device_rect());
    let at_1x = NativeState { scale: 1.0, ..s }.device_rect();
    let at_2x = NativeState { scale: 2.0, ..s }.device_rect();
    assert_eq!(at_2x.width, at_1x.width * 2);
    assert_eq!(at_2x.x, at_1x.x * 2);
}

// --- snapshot / restore ----------------------------------------------------

#[test]
fn snapshot_twice_without_restore_is_an_error() {
    // A second freeze strands the first snapshot's texture: nobody holds a
    // handle to it any more, and it is never released.
    let mut c = controller();
    let mut pool = slots();
    let slot = pool.acquire(0);
    c.bind(slot, NativeViewKind::Video, PREFER_OVERLAY).unwrap();

    let first = c.snapshot(slot).unwrap();
    assert!(first.is_some());
    assert_eq!(c.snapshot(slot), Err(NativeError::AlreadyFrozen(slot)));
    assert_eq!(
        c.provider()
            .commands()
            .iter()
            .filter(|cmd| matches!(cmd, NativeCommand::Snapshot { .. }))
            .count(),
        1
    );
}

#[test]
fn restoring_a_slot_that_was_not_frozen_is_an_error() {
    let mut c = controller();
    let mut pool = slots();
    let slot = pool.acquire(0);
    c.bind(slot, NativeViewKind::Video, PREFER_OVERLAY).unwrap();

    assert_eq!(c.restore(slot), Err(NativeError::NotFrozen(slot)));
    assert_eq!(c.provider().command_count(), 1, "only the create");
}

#[test]
fn a_snapshot_round_trip_hands_ownership_back_exactly_once() {
    let mut c = controller();
    let mut pool = slots();
    let slot = pool.acquire(0);
    c.bind(slot, NativeViewKind::Video, PREFER_OVERLAY).unwrap();

    let snap = c.snapshot(slot).unwrap().unwrap();
    assert!(c.is_frozen(slot));
    c.restore(slot).unwrap();
    assert!(!c.is_frozen(slot));
    assert_eq!(c.restore(slot), Err(NativeError::NotFrozen(slot)));

    let _ = snap;
    assert_eq!(
        c.provider().commands(),
        &[
            NativeCommand::Create {
                slot,
                kind: NativeViewKind::Video
            },
            NativeCommand::Snapshot { slot },
            NativeCommand::Restore { slot },
        ][..]
    );
}

#[test]
fn destroying_a_frozen_participant_hands_the_texture_back_first() {
    // Otherwise the snapshot leaks even though the view went away cleanly.
    let mut c = controller();
    let mut pool = slots();
    let slot = pool.acquire(0);
    c.bind(slot, NativeViewKind::Video, PREFER_OVERLAY).unwrap();
    c.snapshot(slot).unwrap();
    c.destroy(slot).unwrap();

    assert_eq!(
        c.provider().commands(),
        &[
            NativeCommand::Create {
                slot,
                kind: NativeViewKind::Video
            },
            NativeCommand::Snapshot { slot },
            NativeCommand::Restore { slot },
            NativeCommand::Destroy { slot },
        ][..]
    );
    assert!(!c.is_frozen(slot));
    assert!(!c.is_bound(slot));
}

#[test]
fn a_provider_that_cannot_snapshot_says_so_without_being_asked_twice() {
    let caps = NativeCapabilities {
        modes: OVERLAY_ONLY,
        snapshot: false,
        ..NativeCapabilities::default()
    };
    let mut c = NativeViewController::new(FakeNativeProvider::with_capabilities(caps));
    let mut pool = slots();
    let slot = pool.acquire(0);
    c.bind(slot, NativeViewKind::Video, PREFER_OVERLAY).unwrap();

    assert_eq!(c.snapshot(slot), Ok(None));
    assert!(
        !c.provider()
            .commands()
            .iter()
            .any(|cmd| matches!(cmd, NativeCommand::Snapshot { .. })),
        "no snapshot command may be issued to a provider that cannot snapshot"
    );
}

// --- focus ----------------------------------------------------------------

#[test]
fn focus_is_exclusive_and_the_previous_participant_is_blurred() {
    // Two owners of IME is a visible bug (D3): the candidate window jumps
    // between the framework caret and the native field.
    let mut c = controller();
    let mut pool = slots();
    let a = pool.acquire(0);
    let b = pool.acquire(1);
    c.bind(a, NativeViewKind::TextInput, PREFER_OVERLAY)
        .unwrap();
    c.bind(b, NativeViewKind::TextInput, PREFER_OVERLAY)
        .unwrap();

    c.focus(a).unwrap();
    assert_eq!(c.focused(), Some(a));
    c.focus(b).unwrap();
    assert_eq!(c.focused(), Some(b));

    assert_eq!(
        c.provider().commands(),
        &[
            NativeCommand::Create {
                slot: a,
                kind: NativeViewKind::TextInput
            },
            NativeCommand::Create {
                slot: b,
                kind: NativeViewKind::TextInput
            },
            NativeCommand::Focus { slot: a },
            NativeCommand::Blur { slot: a },
            NativeCommand::Focus { slot: b },
        ][..]
    );
}

#[test]
fn focus_on_a_non_focusable_provider_is_refused() {
    let caps = NativeCapabilities {
        modes: OVERLAY_ONLY,
        focusable: false,
        ..NativeCapabilities::default()
    };
    let mut c = NativeViewController::new(FakeNativeProvider::with_capabilities(caps));
    let mut pool = slots();
    let slot = pool.acquire(0);
    c.bind(slot, NativeViewKind::Video, PREFER_OVERLAY).unwrap();

    assert_eq!(c.focus(slot), Err(NativeError::Unsupported("focus")));
    assert_eq!(c.focused(), None);
}

#[test]
fn destroying_the_focused_participant_clears_focus() {
    let mut c = controller();
    let mut pool = slots();
    let slot = pool.acquire(0);
    c.bind(slot, NativeViewKind::TextInput, PREFER_OVERLAY)
        .unwrap();
    c.focus(slot).unwrap();
    c.destroy(slot).unwrap();

    assert_eq!(
        c.focused(),
        None,
        "a destroyed participant must not keep owning IME"
    );
}

// --- capability negotiation ------------------------------------------------

#[test]
fn a_participant_nobody_can_composite_is_refused_at_bind_time() {
    // Wayland has no subwindows (D5). The host must find out when it mounts the
    // participant, not when it paints a blank region.
    let mut c = NativeViewController::new(FakeNativeProvider::incapable());
    let mut pool = slots();
    let slot = pool.acquire(0);

    assert_eq!(
        c.bind(slot, NativeViewKind::Video, PREFER_OVERLAY),
        Err(NativeError::NoSupportedMode)
    );
    assert_eq!(
        c.provider().command_count(),
        0,
        "nothing may be created for a participant that cannot be composited"
    );
}

#[test]
fn negotiation_falls_back_and_the_host_records_what_it_got() {
    let caps = NativeCapabilities {
        modes: TEXTURE_ONLY,
        snapshot: false,
        input: InputModes::ALL,
        focusable: true,
        clip: ClipSupport::Provider,
    };
    let mut c = NativeViewController::new(FakeNativeProvider::with_capabilities(caps));
    let mut pool = slots();
    let slot = pool.acquire(0);

    let chosen = c
        .bind(slot, NativeViewKind::Video, &[OVERLAY, TEXTURE])
        .unwrap();
    assert_eq!(chosen, TEXTURE, "Overlay was preferred but not on offer");
    assert_eq!(c.mode(slot), Some(TEXTURE));
}

#[test]
fn applying_a_mode_the_provider_lacks_is_refused() {
    let caps = NativeCapabilities {
        modes: TEXTURE_ONLY,
        ..NativeCapabilities::default()
    };
    let mut c = NativeViewController::new(FakeNativeProvider::with_capabilities(caps));
    let mut pool = slots();
    let slot = pool.acquire(0);
    c.bind(slot, NativeViewKind::Video, &[TEXTURE]).unwrap();

    assert_eq!(
        c.apply(slot, state(OVERLAY)),
        Err(NativeError::UnsupportedMode(OVERLAY))
    );
    assert_eq!(c.provider().applied().len(), 0);
}

#[test]
fn an_input_mode_the_provider_lacks_is_refused() {
    let caps = NativeCapabilities {
        modes: &[OVERLAY],
        input: InputModes::PASSTHROUGH_ONLY,
        ..NativeCapabilities::default()
    };
    let mut c = NativeViewController::new(FakeNativeProvider::with_capabilities(caps));
    let mut pool = slots();
    let slot = pool.acquire(0);
    c.bind(slot, NativeViewKind::Video, PREFER_OVERLAY).unwrap();

    assert_eq!(
        c.apply(slot, state(OVERLAY)),
        Err(NativeError::Unsupported("capture input")),
        "state() defaults to Capture"
    );
    c.apply(
        slot,
        NativeState {
            input: InputMode::Passthrough,
            ..state(OVERLAY)
        },
    )
    .unwrap();
}

// --- the diff belongs to the host, not the controller ----------------------

#[test]
fn repeated_identical_states_are_forwarded_because_the_host_owns_the_diff() {
    // Deliberate: the controller must not dedupe. Deciding that nothing changed
    // is 16.2's per-frame diff, which knows about damage and about frames that
    // never ran. A controller that quietly dropped the second `apply` would
    // make that diff untestable.
    let mut c = controller();
    let mut pool = slots();
    let slot = pool.acquire(0);
    c.bind(slot, NativeViewKind::Video, PREFER_OVERLAY).unwrap();

    let s = state(OVERLAY);
    c.apply(slot, s).unwrap();
    c.apply(slot, s).unwrap();

    assert_eq!(c.provider().applied(), vec![(slot, s), (slot, s)]);
}

#[test]
fn a_boxed_provider_behaves_the_same_as_a_concrete_one() {
    // The host holds `Box<dyn NativeViewProvider>`; the controller must not
    // care.
    let mut c = NativeViewController::boxed(FakeNativeProvider::new());
    let mut pool = slots();
    let slot = pool.acquire(0);

    let mode = c.bind(slot, NativeViewKind::Web, PREFER_OVERLAY).unwrap();
    assert_eq!(mode, OVERLAY);
    assert!(c.is_bound(slot));
    c.apply(slot, state(mode)).unwrap();
    c.destroy(slot).unwrap();

    assert!(!c.is_bound(slot));
    assert_eq!(c.unreleased().count(), 0);
    // Un-recoverable through `&dyn`, so the assertions above go through the
    // controller's own bookkeeping — which is what the host will have anyway.
    assert_eq!(
        c.apply(slot, state(mode)),
        Err(NativeError::NotBound(slot)),
        "the erased provider still enforces the lifecycle"
    );
}

// --- placement versus clipping ---------------------------------------------

/// A state that needs cropping must never reach a provider that declared it
/// cannot crop.
///
/// The provider would honour its rect by fitting its content into it, which
/// rescales a map or a video instead of cropping it — a wrong frame that looks
/// plausible. Refusing at the controller is what makes the invariant hold for
/// every backend instead of for the ones that remembered.
#[test]
fn a_state_that_needs_cropping_never_reaches_a_provider_that_cannot_crop() {
    let mut c = NativeViewController::new(FakeNativeProvider::cannot_crop());
    let mut pool = slots();
    let slot = pool.acquire(0);
    c.bind(slot, NativeViewKind::Video, PREFER_OVERLAY).unwrap();

    let mut s = state(OVERLAY);
    s.clip = Some(rect(0.0, 0.0, 100.0, 20.0));

    assert_eq!(
        c.apply(slot, s),
        Err(NativeError::UnsupportedClip(rect(0.0, 0.0, 100.0, 20.0)))
    );
    assert_eq!(
        c.provider().applied().len(),
        0,
        "and nothing was applied on the way out"
    );
}

/// A clip that contains the participant does not need cropping, so it is not a
/// reason to refuse anything.
#[test]
fn a_clip_that_does_not_bite_is_allowed_even_without_cropping_support() {
    let mut c = NativeViewController::new(FakeNativeProvider::cannot_crop());
    let mut pool = slots();
    let slot = pool.acquire(0);
    c.bind(slot, NativeViewKind::Video, PREFER_OVERLAY).unwrap();

    let mut s = state(OVERLAY);
    s.clip = Some(rect(-10.0, -10.0, 500.0, 500.0));

    assert_eq!(c.apply(slot, s), Ok(()));
    assert_eq!(c.provider().applied().len(), 1);
}

/// The provider is told *where the view is* and *what part of it shows*, as two
/// things. It is what lets a provider crop in place rather than rescale.
#[test]
fn a_cropping_provider_is_handed_placement_and_crop_separately() {
    let mut c = NativeViewController::new(FakeNativeProvider::new());
    let mut pool = slots();
    let slot = pool.acquire(0);
    c.bind(slot, NativeViewKind::Video, PREFER_OVERLAY).unwrap();

    let mut s = state(OVERLAY);
    s.clip = Some(rect(0.0, 30.0, 100.0, 50.0));
    c.apply(slot, s).unwrap();

    let (_, applied) = c.provider().applied()[0];
    assert_eq!(
        applied.rect,
        rect(0.0, 0.0, 100.0, 50.0),
        "the placement is still the whole view"
    );
    assert_eq!(applied.visible_rect(), rect(0.0, 30.0, 100.0, 50.0));
    assert_ne!(
        applied.visible_rect(),
        applied.rect,
        "and the crop is not the placement"
    );
}

// --- rebinding -------------------------------------------------------------

const OFFSCREEN: NativeCompositionMode = NativeCompositionMode::Offscreen;

/// Changing a participant's composition mode replaces the view on the *same*
/// slot.
///
/// The generation is deliberately left alone: this is the same participant, not
/// a slot recycled for another row, and a provider keying per-slot resources
/// has to be able to tell those two cases apart.
#[test]
fn rebinding_swaps_the_view_without_recycling_the_slot() {
    let mut c = NativeViewController::new(FakeNativeProvider::new());
    let mut pool = slots();
    let slot = pool.acquire(0);
    c.bind(slot, NativeViewKind::Video, PREFER_OVERLAY).unwrap();

    let mode = c.rebind(slot, NativeViewKind::Video, &[OFFSCREEN]).unwrap();

    assert_eq!(mode, OFFSCREEN);
    assert_eq!(c.mode(slot), Some(OFFSCREEN));
    assert_eq!(c.provider().creates(), 2, "a new view was created");
    assert_eq!(c.provider().destroys(), 1, "and the old one destroyed");
    assert_eq!(
        c.unreleased().collect::<Vec<_>>(),
        vec![slot],
        "the pool's accounting still sees exactly one live view, on the same slot"
    );
    assert_eq!(
        slot.generation, 1,
        "and the generation is untouched: a bumped one means 'recycled for \
         something else', which this is not"
    );
}

/// Focus survives the swap. A native text field that had to be recomposited
/// must not silently drop the keyboard — the IME goes with it.
#[test]
fn rebinding_keeps_focus_with_the_participant() {
    let mut c = NativeViewController::new(FakeNativeProvider::new());
    let mut pool = slots();
    let slot = pool.acquire(0);
    c.bind(slot, NativeViewKind::TextInput, PREFER_OVERLAY)
        .unwrap();
    c.focus(slot).unwrap();
    assert_eq!(c.focused(), Some(slot));

    c.rebind(slot, NativeViewKind::TextInput, &[OFFSCREEN])
        .unwrap();

    assert_eq!(c.focused(), Some(slot), "still the keyboard owner");
    assert_eq!(
        c.provider().focuses(),
        2,
        "the new view was handed focus, having never had it"
    );
}

#[test]
fn rebinding_a_slot_that_is_not_bound_is_refused() {
    let mut c = NativeViewController::new(FakeNativeProvider::new());
    let mut pool = slots();
    let slot = pool.acquire(0);

    assert_eq!(
        c.rebind(slot, NativeViewKind::Video, PREFER_OVERLAY),
        Err(NativeError::NotBound(slot))
    );
    assert_eq!(c.provider().creates(), 0);
}

/// A provider that cannot crop may still be *told* about a crop when the
/// framework is the one drawing it.
///
/// `Texture` and `Offscreen` put the pixels in the framework's hands, so the
/// crop is applied downstream of the provider. Refusing these would make the
/// one fallback that works for a cropless backend impossible.
#[test]
fn a_cropless_provider_may_be_told_about_a_crop_it_does_not_apply() {
    let mut c = NativeViewController::new(FakeNativeProvider::cannot_crop());
    let mut pool = slots();
    let slot = pool.acquire(0);
    c.bind(slot, NativeViewKind::Video, &[OFFSCREEN]).unwrap();

    let mut s = state(OFFSCREEN);
    s.clip = Some(rect(0.0, 0.0, 100.0, 20.0));

    assert_eq!(c.apply(slot, s), Ok(()));
    assert_eq!(c.provider().applied().len(), 1);
}

/// And a hidden participant gets through whatever its mode: there is no visible
/// region to get wrong, and a host has to be able to take a participant it
/// cannot composite correctly off the screen.
#[test]
fn a_cropless_provider_can_be_told_to_hide_a_clipped_participant() {
    let mut c = NativeViewController::new(FakeNativeProvider::cannot_crop());
    let mut pool = slots();
    let slot = pool.acquire(0);
    c.bind(slot, NativeViewKind::Video, PREFER_OVERLAY).unwrap();

    let mut s = state(OVERLAY);
    s.clip = Some(rect(0.0, 0.0, 100.0, 20.0));
    s.visible = false;

    assert_eq!(c.apply(slot, s), Ok(()));
    assert_eq!(c.provider().applied(), vec![(slot, s)]);
}
