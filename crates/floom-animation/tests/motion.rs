//! Easing, interruption, and repetition — at the scheduler's own boundary.
//!
//! Every assertion here is on a *sampled value*. The three things this file is
//! about are all invisible to a test that only checks an animation ran and then
//! stopped: a curve and a straight line agree at both ends, a retarget and a
//! restart agree on where the animation ends up, and a repeat and a long tween
//! agree on how long they take. Only the values in between tell them apart.

use floem_animation::{
    AnimKind, AnimationScheduler, Clock, CompositorProp, Easing, FakeClock, RepeatMode, Tween,
};

fn tween(from: f64, to: f64, duration_secs: f64, easing: Easing) -> AnimKind {
    AnimKind::Tween(Tween {
        from,
        to,
        duration_secs,
        easing,
        channels: Vec::new(),
    })
}

fn repeat(inner: AnimKind, iterations: Option<u32>, mode: RepeatMode) -> AnimKind {
    AnimKind::Repeat {
        inner: Box::new(inner),
        iterations,
        mode,
    }
}

// --- B1: easing -----------------------------------------------------------

/// The curve a tween is given has to be the one it travels on. A tween that
/// accepted an easing and ignored it would still pass every endpoint assertion
/// in the crate's own motion tests, because a curve and a line share both ends.
#[test]
fn a_tween_takes_the_shape_of_its_curve() {
    for (easing, name) in [
        (Easing::Linear, "linear"),
        (Easing::EASE_IN, "ease-in"),
        (Easing::EASE_OUT, "ease-out"),
    ] {
        let kind = tween(0.0, 100.0, 1.0, easing);
        for quarter in [0.25, 0.5, 0.75] {
            let (value, _, _) = kind.sample(quarter);
            let expected = easing.ease(quarter) * 100.0;
            assert!(
                (value - expected).abs() < 1e-9,
                "{name} at {quarter} sampled {value}, not its own curve's {expected}"
            );
        }
    }

    // And the shapes are actually different from one another, so the loop above
    // is not comparing three copies of the same line.
    let linear = tween(0.0, 100.0, 1.0, Easing::Linear).sample(0.25).0;
    let easing_in = tween(0.0, 100.0, 1.0, Easing::EASE_IN).sample(0.25).0;
    let easing_out = tween(0.0, 100.0, 1.0, Easing::EASE_OUT).sample(0.25).0;
    assert!(easing_in < linear, "ease-in is behind at a quarter");
    assert!(easing_out > linear, "ease-out is ahead at a quarter");
}

/// A tween's published velocity is its curve's slope, scaled by the distance
/// and the duration — the number a retarget hands on. Zero, or constant, would
/// both be wrong, and only a curved tween can tell them apart.
#[test]
fn a_curved_tween_publishes_a_changing_velocity() {
    let mut sched = AnimationScheduler::new();
    sched.start(tween(0.0, 100.0, 1.0, Easing::EASE_IN), 0.0);
    let early = sched.tick(0.05)[0].velocity;
    let late = sched.tick(0.95)[0].velocity;
    assert!(
        late > early * 3.0,
        "an ease-in tween accelerates, so its velocity must climb: {early} -> {late}"
    );
}

// --- B3: repetition -------------------------------------------------------

/// The endless case, which is what a spinner needs: the value wraps round and
/// the animation never reports finished, however long it has been running.
#[test]
fn a_repeating_tween_wraps_instead_of_ending() {
    let endless = repeat(
        tween(0.0, 100.0, 1.0, Easing::Linear),
        None,
        RepeatMode::Restart,
    );
    for (t, expected) in [(0.25, 25.0), (1.25, 25.0), (2.75, 75.0), (101.5, 50.0)] {
        let (value, _, finished) = endless.sample(t);
        assert!(
            (value - expected).abs() < 1e-6,
            "at {t}s the cycle should be at {expected}, not {value}"
        );
        assert!(!finished, "an endless repeat must never report finished");
    }
}

/// The bounded case has to end *on a cycle boundary*, not merely eventually:
/// stopping mid-cycle would leave the element parked partway through the sweep.
#[test]
fn a_bounded_repeat_ends_on_its_last_cycles_endpoint() {
    let three_times = repeat(
        tween(0.0, 100.0, 1.0, Easing::Linear),
        Some(3),
        RepeatMode::Restart,
    );
    for t in [0.5, 1.5, 2.5] {
        let (value, _, finished) = three_times.sample(t);
        assert!((value - 50.0).abs() < 1e-6, "cycle at {t}s should be at 50");
        assert!(!finished, "{t}s is not the end of three cycles");
    }
    let (value, _, finished) = three_times.sample(3.0);
    assert!(finished, "three cycles are over at 3s");
    assert!(
        (value - 100.0).abs() < 1e-6,
        "and it stops at the far end, not back at the start: {value}"
    );
}

/// A reversing repeat is a ping-pong: the value must come back down through the
/// middle on the way home, which a restart-mode repeat never does.
#[test]
fn a_reversing_repeat_comes_back_the_way_it_went() {
    let there_and_back = repeat(
        tween(0.0, 100.0, 1.0, Easing::Linear),
        Some(2),
        RepeatMode::Reverse,
    );
    // Outward on the first half of each cycle, homeward on the second.
    assert!((there_and_back.sample(0.5).0 - 50.0).abs() < 1e-6);
    assert!((there_and_back.sample(1.5).0 - 50.0).abs() < 1e-6);
    assert!(
        (there_and_back.sample(2.0).0 - 0.0).abs() < 1e-6,
        "back home"
    );
    assert!(
        (there_and_back.sample(3.0).0 - 100.0).abs() < 1e-6,
        "out again"
    );
    assert!(!there_and_back.sample(3.9).0.is_nan());

    let (_, _, finished) = there_and_back.sample(4.0);
    assert!(finished, "two round trips of a 1s cycle take 4s");

    // The restart-mode one covers the same ground without ever going backwards
    // through the middle, which is the difference the mode selects.
    let restarted = repeat(
        tween(0.0, 100.0, 1.0, Easing::Linear),
        Some(2),
        RepeatMode::Restart,
    );
    assert!(
        (restarted.sample(1.5).0 - 50.0).abs() < 1e-6,
        "restart mode is on its way up again, not down"
    );

    // The return half has to actually *descend*. A `Reverse` implemented as a
    // `Restart` would still agree with the samples above, because a cycle and a
    // round trip cross each other wherever they meet in the middle; only a run
    // of samples tells the two apart.
    let descending: Vec<f64> = [1.1, 1.3, 1.5, 1.7, 1.9]
        .iter()
        .map(|t| there_and_back.sample(*t).0)
        .collect();
    for pair in descending.windows(2) {
        assert!(
            pair[1] < pair[0],
            "the reversing half must come back down: {descending:?}"
        );
    }
    assert!(
        (descending[0] - 90.0).abs() < 1e-6,
        "a reversing cycle is already homeward at 1.1s: {}",
        descending[0]
    );
}

/// A repeat of a spring is documented to play once. This is that documentation
/// as an executable claim — without it the degradation is a comment nobody can
/// contradict.
#[test]
fn a_repeat_of_a_spring_plays_it_once() {
    let repeated = repeat(
        AnimKind::Spring {
            from: 0.0,
            to: 10.0,
            velocity: 0.0,
            stiffness: 100.0,
            damping: 20.0,
            channels: Vec::new(),
        },
        None,
        RepeatMode::Restart,
    );
    let early = repeated.sample(0.1);
    let late = repeated.sample(3.0);
    assert!(!early.2, "the spring is still running at 0.1s");
    assert!(
        late.2,
        "and it is over by 3s — there was no cycle to repeat"
    );
    assert!((late.0 - 10.0).abs() < 0.01);
}

/// An endless animation keeps the scheduler holding it forever, which is what
/// keeps a frame loop awake for a spinner. If it were retired at the end of its
/// first cycle the spinner would stop after one sweep.
#[test]
fn an_endless_animation_holds_the_scheduler_open() {
    let mut sched = AnimationScheduler::new();
    sched.start(
        repeat(
            tween(0.0, 1.0, 0.2, Easing::Linear),
            None,
            RepeatMode::Restart,
        ),
        0.0,
    );
    for step in 1..50 {
        sched.tick(f64::from(step) * 0.05);
        assert_eq!(sched.len(), 1, "the spinner was retired at step {step}");
    }
}

/// A finite repeat is retired exactly like any other animation — the counting
/// repeat must not become immortal by accident.
#[test]
fn a_finite_repeat_is_retired_when_it_is_done() {
    let mut sched = AnimationScheduler::new();
    sched.start(
        repeat(
            tween(0.0, 1.0, 0.1, Easing::Linear),
            Some(2),
            RepeatMode::Restart,
        ),
        0.0,
    );
    assert_eq!(sched.tick(0.15).len(), 1, "still on its second cycle");
    let last = sched.tick(0.2);
    assert_eq!(last.len(), 1);
    assert!(last[0].finished);
    assert!(sched.is_empty(), "and the scheduler let it go");
}

// --- B2: interruption -----------------------------------------------------

/// The headline: redirecting a moving animation must not teleport it.
///
/// The animation is at 50 travelling at 100 units/s, and is sent back to zero.
/// A tween whose target was simply swapped would either sit still at 50 (clock
/// restarted) or instantly head down at the new speed; only carrying the
/// velocity forward puts it *past* 50 on the next frame.
#[test]
fn retarget_does_not_jump_and_keeps_the_momentum() {
    let mut sched = AnimationScheduler::new();
    let mut clock = FakeClock::new();
    let id = sched.start(tween(0.0, 100.0, 1.0, Easing::Linear), clock.now_secs());

    clock.advance(0.5);
    let before = sched.tick(clock.now_secs());
    assert!((before[0].value - 50.0).abs() < 1e-6);
    assert!(
        (before[0].velocity - 100.0).abs() < 0.5,
        "halfway through a 1s tween from 0 to 100 the velocity is 100/s, \
         not {}",
        before[0].velocity
    );

    assert!(sched.retarget(id, 0.0, clock.now_secs()));

    // The value at the instant of the retarget, and a hair later: the velocity
    // it was carrying has to still be there. Sampled finely, because the
    // spring's own restoring force takes over within a couple of milliseconds
    // and would mask a dropped velocity at frame resolution.
    clock.advance(1e-3);
    let just_after = sched.tick(clock.now_secs());
    assert_eq!(just_after.len(), 1);
    assert!(
        just_after[0].velocity > 50.0,
        "the momentum was dropped: velocity right after the retarget was {}",
        just_after[0].velocity
    );
    assert!(
        just_after[0].value > 50.0,
        "and it is still travelling forward: {}",
        just_after[0].value
    );

    // One whole 60 Hz frame later it must not have teleported anywhere.
    clock.advance(1.0 / 60.0);
    let frame = sched.tick(clock.now_secs());
    let moved = (frame[0].value - 50.0).abs();
    assert!(
        moved < 2.0,
        "the retarget jumped {moved} units in one frame"
    );
}

#[test]
fn a_retargeted_animation_settles_on_its_new_target() {
    let mut sched = AnimationScheduler::new();
    let mut clock = FakeClock::new();
    let id = sched.start(tween(0.0, 100.0, 1.0, Easing::Linear), clock.now_secs());
    clock.advance(0.5);
    sched.tick(clock.now_secs());
    assert!(sched.retarget(id, 25.0, clock.now_secs()));

    clock.advance(3.0);
    let samples = sched.tick(clock.now_secs());
    assert_eq!(samples.len(), 1, "the spring is still going at 3.5s");
    assert!(samples[0].finished);
    assert!(
        (samples[0].value - 25.0).abs() < 0.01,
        "it settled at {}, not on the target it was sent to",
        samples[0].value
    );
}

#[test]
fn retarget_refuses_an_animation_that_is_not_running() {
    let mut sched = AnimationScheduler::new();
    let id = sched.start(tween(0.0, 1.0, 1.0, Easing::Linear), 0.0);
    sched.cancel(id);
    assert!(
        !sched.retarget(id, 5.0, 0.5),
        "a retired animation cannot be redirected"
    );
}

/// A retarget carries the compositor channel across, or a fading overlay that
/// gets redirected would silently stop fading.
#[test]
fn a_retarget_keeps_driving_the_same_compositor_channel() {
    let mut sched = AnimationScheduler::new();
    let mut clock = FakeClock::new();
    let id = sched.start(
        AnimKind::Tween(
            Tween::new(0.0, 1.0, 1.0).with_compositor(CompositorProp::Opacity(0.0)),
        ),
        clock.now_secs(),
    );
    clock.advance(0.5);
    sched.tick(clock.now_secs());
    assert!(sched.retarget(id, 0.25, clock.now_secs()));

    clock.advance(1.0 / 60.0);
    let samples = sched.tick(clock.now_secs());
    assert!(
        matches!(samples[0].compositor.as_slice(), [CompositorProp::Opacity(_)]),
        "the channel was lost in the retarget: {:?}",
        samples[0].compositor
    );
}

// --- determinism and the frame-loop contract -------------------------------

/// Samples come back in the order the animations were started, not in hash
/// order. A "deterministic scheduler" whose output order depends on a map's
/// iteration order is deterministic only until the hasher changes.
#[test]
fn samples_come_back_in_the_order_they_were_started() {
    let mut sched = AnimationScheduler::new();
    let ids: Vec<_> = (0..8)
        .map(|i| {
            sched.start(
                tween(f64::from(i), f64::from(i) + 10.0, 1.0, Easing::Linear),
                0.0,
            )
        })
        .collect();
    for round in 0..5 {
        let samples = sched.tick(0.1 + f64::from(round) * 0.1);
        let order: Vec<_> = samples.iter().map(|s| s.id).collect();
        assert_eq!(order, ids, "sampling order changed on round {round}");
    }
}

/// The frame-loop contract: sampling must reuse the buffer it is handed.
///
/// Asserted on a *pre-sized* buffer, because "the capacity did not grow during
/// steady state" is true of an implementation that throws the caller's buffer
/// away and builds a fresh one every tick — it just settles at whatever the
/// animation count needs and never moves again. Handing in spare capacity and
/// asking for it back is the assertion that tells reuse from replacement.
#[test]
fn tick_into_reuses_the_buffer_it_is_given() {
    let mut sched = AnimationScheduler::new();
    let mut clock = FakeClock::new();
    sched.start(
        repeat(
            tween(0.0, 1.0, 0.2, Easing::Linear),
            None,
            RepeatMode::Restart,
        ),
        clock.now_secs(),
    );

    let mut out = Vec::with_capacity(32);
    clock.advance(1.0 / 60.0);
    sched.tick_into(clock.now_secs(), &mut out);
    assert_eq!(out.len(), 1, "the sample landed in the caller's buffer");
    assert_eq!(
        out.capacity(),
        32,
        "the sampler replaced the caller's buffer instead of clearing it"
    );

    for _ in 0..600 {
        clock.advance(1.0 / 60.0);
        sched.tick_into(clock.now_secs(), &mut out);
    }
    assert_eq!(
        out.capacity(),
        32,
        "capacity was lost while ticking a second of animation"
    );
}

/// `tick` is the allocating convenience wrapper, and has to agree with the
/// buffer form exactly — otherwise a test that uses one is not testing the
/// frame loop that uses the other.
#[test]
fn tick_and_tick_into_agree() {
    let mut scheduling = AnimationScheduler::new();
    let mut clock = FakeClock::new();
    for id in 0..3 {
        scheduling.start(
            tween(0.0, f64::from(id + 1) * 10.0, 1.0, Easing::EASE_OUT),
            clock.now_secs(),
        );
    }
    let mut via_into = Vec::new();
    for step in 1..20 {
        clock.advance(0.05);
        let now = clock.now_secs();
        let direct = scheduling.tick(now);
        scheduling.tick_into(now, &mut via_into);
        assert_eq!(direct, via_into, "the two forms diverged at step {step}");
    }
}
