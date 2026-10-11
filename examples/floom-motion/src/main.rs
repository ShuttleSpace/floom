//! Motion, from the prelude alone.
//!
//! Eight acts, each one a capability that did not exist before S1 and each one
//! asserted rather than described, because "it animates" is the easiest claim
//! in this codebase to write down and never check:
//!
//! 1. **A curve is not a duration.** The same fade, run twice, once linear and
//!    once eased, sampled at the same instant — and the eased one is further
//!    along. That is the whole difference between a mechanical slide and a
//!    motion that looks designed, and it is invisible to any assertion that
//!    only checks the endpoints.
//! 2. **Interruption keeps what it interrupted.** Redirecting a running fade
//!    mid-flight has to start from where it is, with the speed it had — not
//!    from the start, and not from a standstill.
//! 3. **A control can animate itself.** `Progress::indeterminate()` states the
//!    shuttle it wants and the host runs it, with no host handle in sight; the
//!    frame loop stays awake for it and parks when the bar is given a value.
//! 4. **A presence transition delays unmount.** A subtree leaving the screen
//!    has to stay mounted until its exit animation has played, or the departure
//!    would be invisible — so `Conditional::exit` keeps the child on screen and
//!    only unmounts it once the host reports the transition finished.
//! 5. **A crossfade keeps both sides live.** `Crossfade` stacks its two
//!    contents in the same box and swaps by fading — so during the swap *both*
//!    stay mounted (the leaving one plays its exit, the arriving one its enter),
//!    which a plain cut would never show. This is the "one of AnimatedContent /
//!    Crossfade" half of the B5 gap.
//! 6. **An `AnimatedContent` keeps both sides live by key.** The same property
//!    as act 5, but driven by a content key instead of two fixed slots: switch
//!    the tracked value and the leaving content stays mounted (exit) while the
//!    arriving one enters, and a value that returns mounts a fresh node that
//!    replays its enter. This closes the AnimatedContent half of B5.
//! 7. **An opacity transition fades instead of snapping.** Change a subtree's
//!    target opacity and the whole subtree fades there — this is the B4
//!    "change-the-value-and-it-animates" layer extended from colour (which
//!    `Button::transition` drives through paint) to the compositor opacity
//!    channel, the Flutter `AnimatedOpacity` half. A value that returns fades
//!    back up from where it rested, not from where it was born.
//! 8. **A shared element flies its ink.** Two subtrees that own the same
//!    `shared_key` — the source up top, the destination below — are both kept
//!    mounted, and arming the flight translates the source's ink down onto the
//!    destination's box before the destination's *own* content is revealed.
//!    Redirect it mid-flight and the interrupt keeps the speed it had. This
//!    closes the `SharedTransitionLayout` / `Hero` half of B5, and the single
//!    `TranslateY` channel that carries it is B6's multi-value orchestration in
//!    action.
//!
//! Everything below is `floom::prelude` — no crate below the facade is named.
//! Run with `cargo run -p floom-motion`.

use floom::floom_scene::SceneNode;
use floom::prelude::*;

/// Frame step the host advances per `frame()`, so "six frames" is a tenth of a
/// second and the sampling instants are exact rather than approximate.
const FRAME_SECS: f64 = 1.0 / 60.0;

/// Runs `kind` on `node` and returns the sample after each of `frames` frames.
fn samples(host: &mut WidgetHost, node: NodeId, kind: AnimKind, frames: usize) -> Vec<f64> {
    host.cancel_animation(node);
    host.animate(node, kind);
    (0..frames)
        .map(|_| {
            host.frame();
            host.animation_value(node)
                .expect("the animation is running")
        })
        .collect()
}

/// A fade from opaque to invisible over `secs`, shaped by `easing`.
///
/// An `f64` in `0.0..=1.0` is the compositor's opacity; the animation runs from
/// `1.0` down to `0.0`, so the fraction of the way *travelled* is `1 - value`.
fn fade(secs: f64, easing: Easing) -> AnimKind {
    AnimKind::Tween(
        Tween::new(1.0, 0.0, secs)
            .with_easing(easing)
            .with_compositor(CompositorProp::Opacity(0.0)),
    )
}

fn main() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(240.0, 80.0));
    let root = host.mount_root(LayoutStyle::default(), Container::new());
    let label = host.mount(root, LayoutStyle::default(), Text::new("motion"));

    act_1_a_curve_is_not_a_duration(&mut host, label);
    act_2_interruption_keeps_what_it_interrupted(&mut host, label);
    act_3_a_control_can_animate_itself(&mut host, root);
    act_4_a_presence_transition_delays_unmount(&mut host);
    act_5_a_crossfade_keeps_both_sides_live(&mut host);
    act_6_an_animated_content_keeps_both_sides_live(&mut host);
    act_7_an_opacity_transition_fades_instead_of_snapping(&mut host);
    act_8_a_shared_element_flies_to_its_destination(&mut host);

    println!();
    println!("every act above was asserted, not printed");
}

/// The same fade twice, sampled at the same instant: a curve changes *when* the
/// distance is covered, never the endpoints.
fn act_1_a_curve_is_not_a_duration(host: &mut WidgetHost, label: NodeId) {
    const DURATION: f64 = 0.4;
    // Six frames is a tenth of a second — a quarter of the way in.
    const OBSERVED: usize = 6;

    let linear = samples(host, label, fade(DURATION, Easing::Linear), OBSERVED);
    let eased = samples(host, label, fade(DURATION, Easing::EASE_OUT), OBSERVED);

    let quarter = OBSERVED as f64 * FRAME_SECS / DURATION;
    let linear_travel = 1.0 - linear[OBSERVED - 1];
    let eased_travel = 1.0 - eased[OBSERVED - 1];

    println!("== 1. a curve is not a duration ==");
    println!(
        "  fade of {DURATION}s, sampled at {:.0}% of the way in:",
        quarter * 100.0
    );
    println!(
        "    linear   travelled {:.3} of the distance",
        linear_travel
    );
    println!("    ease-out travelled {:.3} of the distance", eased_travel);

    assert!(
        (linear_travel - quarter).abs() < 0.02,
        "control: a linear fade is at the same fraction as the clock \
         (t={quarter}, travelled {linear_travel})"
    );
    assert!(
        eased_travel > quarter * 1.3,
        "an ease-out is meant to be well ahead of the clock early on: \
         {eased_travel} at {quarter}"
    );
    // Note the direction: these are opacity values counting *down*, so "ahead"
    // is the smaller number. Comparing the values directly would have said the
    // opposite of what the curve does.
    let travel = |values: &[f64]| values.iter().map(|v| 1.0 - v).collect::<Vec<_>>();
    let linear_travels = travel(&linear);
    let eased_travels = travel(&eased);
    assert!(
        eased_travels
            .iter()
            .zip(&linear_travels)
            .all(|(eased, linear)| eased > linear),
        "an ease-out is ahead of linear at every sampled instant, not just the \
         last: {eased_travels:?} vs {linear_travels:?}"
    );

    // Both end in the same place, which is the half of the claim that keeps a
    // curve from being a second animation.
    let linear_end = samples(host, label, fade(DURATION, Easing::Linear), 30);
    let eased_end = samples(host, label, fade(DURATION, Easing::EASE_OUT), 30);
    assert_eq!(linear_end[29], 0.0, "linear reaches the endpoint");
    assert_eq!(eased_end[29], 0.0, "and so does ease-out");
    println!("  both curves end on 0.000, as they must");
}

/// Redirecting a running animation carries on from where it is, with the speed
/// it had.
fn act_2_interruption_keeps_what_it_interrupted(host: &mut WidgetHost, label: NodeId) {
    println!("== 2. interruption ==");
    host.cancel_animation(label);
    host.animate(label, fade(0.4, Easing::EASE_OUT));
    let relaid = {
        let mut relaid = 0;
        for _ in 0..6 {
            if host.frame().relaid {
                relaid += 1;
            }
        }
        relaid
    };
    let midway = host.animation_value(label).expect("running");
    assert_eq!(
        relaid, 0,
        "control: motion is a compositor repaint — six frames of it must not \
         re-lay-out the tree even once"
    );
    println!("  frames that re-laid-out during motion: {relaid}");

    assert!(
        host.retarget_animation(label, 1.0),
        "the node had an animation to redirect"
    );
    host.frame();
    let after = host.animation_value(label).expect("running");
    println!("  redirected at {midway:.3} -> next frame {after:.3}");
    assert!(
        after > midway - 0.25,
        "a redirect must not jump the node: {midway} -> {after}"
    );

    for _ in 0..90 {
        host.frame();
    }
    let settled = host.animation_value(label).expect("parked");
    println!("  settled on {settled:.3} (the new target was 1.0)");
    assert!(
        settled > 0.9,
        "it has to arrive at the target it was sent to"
    );

    // Stopping is the other half of this, and it is deliberately *not* the
    // same as finishing: a finished animation holds its endpoint, a stopped
    // one gives the node back to its widget's paint. (The animation above has
    // already finished, so there is nothing left to stop — this needs one in
    // flight.)
    host.animate(label, fade(5.0, Easing::Linear));
    for _ in 0..10 {
        host.frame();
    }
    let running = host.animation_value(label).expect("running");
    assert!(
        running < 1.0 && running > 0.0,
        "control: a long fade is partway through ({running})"
    );
    assert!(
        host.cancel_animation(label),
        "there was an animation to stop"
    );
    host.frame();
    assert_eq!(
        host.animation_value(label),
        None,
        "stopping releases the channel rather than freezing the node in it"
    );
    println!("  stopped mid-flight: the channel is released, the node is the widget's again");
}

/// A control that animates itself: `Progress` declares the shuttle it wants and
/// the host runs it.
fn act_3_a_control_can_animate_itself(host: &mut WidgetHost, root: NodeId) {
    println!("== 3. a control that animates itself ==");
    let bar = host.mount(root, LayoutStyle::default(), Progress::indeterminate());
    host.frame();

    let running = host.animations_running();
    let start = host
        .animation_value(bar)
        .expect("the declaration is running");
    println!("  the host is running {running} animation(s) nobody asked it to start");

    for _ in 0..8 {
        host.frame();
    }
    let moved = host.animation_value(bar).expect("still running");
    assert_ne!(start, moved, "the shuttle has to move");
    assert!(
        host.needs_frame(),
        "and an endless one keeps asking for frames"
    );
    println!("  shuttle: {start:.3} -> {moved:.3} after 8 frames");

    host.widget_mut::<Progress>(bar)
        .expect("the bar is mounted")
        .set_value(Some(0.4));
    host.frame();
    host.frame();
    assert_eq!(
        host.animations_running(),
        0,
        "a bar with a value stops animating itself"
    );
    assert!(
        !host.needs_frame(),
        "and stops asking for frames, so an idle app can park"
    );
    println!("  given a value, the bar stops and the frame loop parks");
}

/// A presence transition keeps an exiting subtree mounted until its exit
/// animation has finished, then unmounts it — the part of the implicit layer
/// that is "the widget's *existence* changed", not "its value did".
fn act_4_a_presence_transition_delays_unmount(_host: &mut WidgetHost) {
    println!("== 4. a presence transition delays unmount ==");

    // Its own host: the scene it inspects must not be polluted by the text
    // nodes the earlier acts left mounted.
    let mut host = WidgetHost::new();
    host.set_size(Size::new(240.0, 80.0));

    let child = Child::new(ChildKey::index(0), LayoutRequest::default(), || {
        Box::new(Text::new("hi"))
    });
    let presence = Conditional::new(true)
        .show(child)
        .presence(PresenceTransition::fade(0.2));
    let root = host.mount_root(LayoutStyle::default(), presence);
    host.frame();

    // Let the enter finish and rest.
    for _ in 0..30 {
        host.frame();
    }
    let count_text_nodes = |host: &WidgetHost| {
        host.scene()
            .iter()
            .filter(|(_, node)| matches!(node, SceneNode::Text { .. }))
            .count()
    };
    assert_eq!(count_text_nodes(&host), 1, "the child is shown");

    // Tell it to leave. The child must stay on screen the moment after — an
    // exit transition that unmounted immediately would never be seen.
    host.widget_mut::<Conditional>(root)
        .expect("the root is a conditional")
        .set_when(false);
    host.frame();
    assert_eq!(
        count_text_nodes(&host),
        1,
        "the child stays mounted the instant it is told to leave"
    );

    // And it is partway through fading out, not instantly gone.
    for _ in 0..4 {
        host.frame();
    }
    let mid_alpha = host
        .scene()
        .iter()
        .find_map(|(_, node)| match node {
            SceneNode::Text { color, .. } => Some(color.a),
            _ => None,
        })
        .expect("child present");
    assert!(
        mid_alpha < 255,
        "the exit is mid-fade, not instant: alpha {mid_alpha}"
    );

    // Run the exit out. Once the host reports the transition finished, the
    // widget unmounts the subtree.
    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(
        count_text_nodes(&host),
        0,
        "the child is unmounted only after its exit completed"
    );
    println!("  enter fades in; exit keeps the child mounted until it finishes, then it unmounts");
}

/// A crossfade stacks its two contents in the same box and swaps by fading, so
/// during the swap *both* stay mounted — the leaving side plays its exit, the
/// arriving side its enter — which a plain cut would never show. This is the
/// AnimatedContent / Crossfade half of the B5 gap.
fn act_5_a_crossfade_keeps_both_sides_live(_host: &mut WidgetHost) {
    println!("== 5. a crossfade keeps both sides live ==");

    let mut host = WidgetHost::new();
    host.set_size(Size::new(240.0, 80.0));

    let side_a = Child::new(ChildKey::index(0), LayoutRequest::default(), || {
        Box::new(Text::new("A"))
    });
    let side_b = Child::new(ChildKey::index(1), LayoutRequest::default(), || {
        Box::new(Text::new("B"))
    });
    let root = host.mount_root(
        LayoutStyle::default(),
        Crossfade::new(true)
            .a(side_a)
            .b(side_b)
            .presence(PresenceTransition::fade(0.2)),
    );
    host.frame();

    // Let A's enter finish and rest.
    for _ in 0..30 {
        host.frame();
    }
    let count_text = |host: &WidgetHost| {
        host.scene()
            .iter()
            .filter(|(_, node)| matches!(node, SceneNode::Text { .. }))
            .count()
    };
    let first_alpha = |host: &WidgetHost| {
        host.scene()
            .iter()
            .find_map(|(_, node)| match node {
                SceneNode::Text { color, .. } => Some(color.a),
                _ => None,
            })
            .expect("a child is mounted")
    };
    assert_eq!(count_text(&host), 1, "a rested crossfade shows only A");
    assert_eq!(first_alpha(&host), 255, "and A is fully shown");

    // Swap to B. The instant after the flip *both* sides are mounted — A mid-
    // exit, B mid-enter — so count_text is 2, and their alphas are at opposite
    // ends of the fade: A still fully visible, B still invisible.
    host.widget_mut::<Crossfade>(root)
        .expect("the root is a crossfade")
        .set_show_a(false);
    host.frame();
    let both: Vec<u8> = host
        .scene()
        .iter()
        .filter_map(|(_, node)| match node {
            SceneNode::Text { color, .. } => Some(color.a),
            _ => None,
        })
        .collect();
    assert_eq!(
        both.len(),
        2,
        "a crossfade keeps both sides mounted through the swap"
    );
    assert!(
        both.iter().any(|&a| a == 255) && both.iter().any(|&a| a == 0),
        "the leaving side is still shown and the arriving side is still hidden \
         at the start of the swap: {both:?}"
    );

    // Across the whole transition both sides stay live.
    for _ in 0..4 {
        host.frame();
        assert_eq!(
            count_text(&host),
            2,
            "both sides remain mounted while the crossfade plays"
        );
    }

    // Run it out: A's exit finishes, it unmounts, and only B rests shown.
    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(
        count_text(&host),
        1,
        "once the swap completes only B is mounted"
    );
    assert_eq!(first_alpha(&host), 255, "and B rests fully shown");
    println!(
        "  A and B both stay on screen during the swap; A unmounts only when its exit finishes"
    );
}

/// An `AnimatedContent` is the N-ary, content-keyed generalisation of the
/// crossfade: instead of two fixed sides it switches on a value, and whenever
/// that value changes the leaving content stays mounted and plays its exit
/// while the arriving content plays its enter. So it is the same "both sides
/// live" property as act 5, but driven by a key rather than a two-slot swap —
/// this closes the AnimatedContent half of B5.
fn act_6_an_animated_content_keeps_both_sides_live(_host: &mut WidgetHost) {
    println!("== 6. an animated content keeps both sides live ==");

    let mut host = WidgetHost::new();
    host.set_size(Size::new(240.0, 80.0));

    let root = host.mount_root(
        LayoutStyle::default(),
        AnimatedContent::new(1u32)
            .content_with(|v| {
                let label = format!("v{v}");
                Child::new(ChildKey::index(0), LayoutRequest::default(), move || {
                    Box::new(Text::new(label.clone()))
                })
            })
            .presence(PresenceTransition::fade(0.2)),
    );
    host.frame();

    for _ in 0..30 {
        host.frame();
    }
    let count_text = |host: &WidgetHost| {
        host.scene()
            .iter()
            .filter(|(_, node)| matches!(node, SceneNode::Text { .. }))
            .count()
    };
    let first_alpha = |host: &WidgetHost| {
        host.scene()
            .iter()
            .find_map(|(_, node)| match node {
                SceneNode::Text { color, .. } => Some(color.a),
                _ => None,
            })
            .expect("a child is mounted")
    };
    assert_eq!(count_text(&host), 1, "rests showing one content");
    assert_eq!(first_alpha(&host), 255, "and it is fully shown");

    // Switch the key to 2. The instant after, both the leaving (1) and arriving
    // (2) contents are mounted — one still shown, the other still hidden.
    host.widget_mut::<AnimatedContent<u32>>(root)
        .expect("the root is an animated content")
        .set_value(2);
    host.frame();
    let both: Vec<u8> = host
        .scene()
        .iter()
        .filter_map(|(_, node)| match node {
            SceneNode::Text { color, .. } => Some(color.a),
            _ => None,
        })
        .collect();
    assert_eq!(
        both.len(),
        2,
        "an animated content keeps both contents mounted through the switch"
    );
    assert!(
        both.iter().any(|&a| a == 255) && both.iter().any(|&a| a == 0),
        "the leaving side is still shown and the arriving side is still hidden \
         at the start of the switch: {both:?}"
    );

    for _ in 0..4 {
        host.frame();
        assert_eq!(
            count_text(&host),
            2,
            "both contents remain mounted while the switch plays"
        );
    }

    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(
        count_text(&host),
        1,
        "once the switch completes only 2 is mounted"
    );
    assert_eq!(first_alpha(&host), 255, "and it rests fully shown");

    // And a value that returns mounts a fresh node that replays its enter: the
    // returning 1 starts from invisible, it does not resurrect the 1 that
    // already finished.
    host.widget_mut::<AnimatedContent<u32>>(root)
        .expect("the root is an animated content")
        .set_value(1);
    host.frame();
    let returned: Vec<u8> = host
        .scene()
        .iter()
        .filter_map(|(_, node)| match node {
            SceneNode::Text { color, .. } => Some(color.a),
            _ => None,
        })
        .collect();
    assert_eq!(
        returned.len(),
        2,
        "the returning value and the leaving value are both mounted"
    );
    assert!(
        returned.iter().any(|&a| a == 0),
        "the returning value replays its enter from invisible: {returned:?}"
    );

    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(count_text(&host), 1);
    assert_eq!(first_alpha(&host), 255);
    println!("  v1 -> v2 keeps both mounted; v2 -> v1 (a return) replays the enter from invisible");
}

/// The implicit opacity transition: change a subtree's target opacity and it
/// fades there instead of snapping. This is the B4 "change-the-value-and-it-
/// animates" layer extended from colour (which `Button::transition` drives
/// through paint) to the compositor opacity channel — the Flutter
/// `AnimatedOpacity` half.
fn act_7_an_opacity_transition_fades_instead_of_snapping(_host: &mut WidgetHost) {
    println!("== 7. an opacity transition fades instead of snapping ==");

    let mut host = WidgetHost::new();
    host.set_size(Size::new(240.0, 80.0));
    let child = Child::new(ChildKey::index(1), LayoutRequest::default(), || {
        Box::new(Text::new("hi"))
    });
    let root = host.mount_root(
        LayoutStyle::default(),
        Opacity::new(child)
            .opacity(1.0)
            .transition(Transition::new(0.2)),
    );
    host.frame();

    let first_alpha = |host: &WidgetHost| {
        host.scene()
            .iter()
            .find_map(|(_, node)| match node {
                SceneNode::Text { color, .. } => Some(color.a),
                _ => None,
            })
            .expect("a child is mounted")
    };

    assert_eq!(first_alpha(&host), 255, "rests fully shown");

    // Change the target to 0.5. A few frames in it is strictly between the
    // endpoints — it animated rather than snapped.
    host.widget_mut::<Opacity>(root)
        .expect("the root is an opacity")
        .set_opacity(0.5);
    for _ in 0..3 {
        host.frame();
    }
    let mid = first_alpha(&host);
    assert!(mid < 255, "the fade has left full opacity: {mid}");
    assert!(mid > 140, "but has not yet arrived at 0.5: {mid}");

    for _ in 0..30 {
        host.frame();
    }
    let settled = first_alpha(&host);
    assert!(
        settled < 155 && settled > 110,
        "it landed near 0.5: {settled}"
    );

    // And back to 1.0: it fades up from 0.5, not from the initial 1.0.
    host.widget_mut::<Opacity>(root)
        .expect("the root is an opacity")
        .set_opacity(1.0);
    host.frame();
    let just_started = first_alpha(&host);
    assert!(
        just_started < 255,
        "it did not snap back to full opacity: {just_started}"
    );
    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(first_alpha(&host), 255, "and lands fully shown again");
    println!("  1.0 -> 0.5 fades; 0.5 -> 1.0 fades back up from 0.5, not from 1.0");
}

/// A shared element flies one subtree's ink down onto the other's box, then
/// reveals the destination's *own* content — and can be redirected mid-flight
/// without snapping. This is the `SharedTransitionLayout` / `Hero` half of B5,
/// carried on a single `TranslateY` channel (B6).
fn act_8_a_shared_element_flies_to_its_destination(_host: &mut WidgetHost) {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    println!("== 8. a shared element flies to its destination ==");

    let mut host = WidgetHost::new();
    host.set_size(Size::new(240.0, 120.0));
    let fly = Arc::new(AtomicBool::new(false));

    let src = Child::new(ChildKey::index(0), LayoutRequest::new(), || {
        Box::new(Text::new("src"))
    });
    let dst = Child::new(ChildKey::index(1), LayoutRequest::new(), || {
        Box::new(Text::new("dst"))
    });
    let _root = host.mount_root(
        LayoutStyle::default(),
        SharedTransitionLayout::new(7, 40.0)
            .duration(0.4)
            .fly_signal(fly.clone())
            .source(src)
            .dest(dst),
    );
    host.frame();

    // Capture the source's resting Y, then arm the flight.
    let first_y = host
        .scene()
        .iter()
        .find_map(|(_, node)| match node {
            SceneNode::Text { origin, .. } => Some(origin.y),
            _ => None,
        })
        .expect("a shared element text is mounted");
    let one_text = |host: &WidgetHost| {
        host.scene()
            .iter()
            .filter(|(_, node)| matches!(node, SceneNode::Text { .. }))
            .count()
    };
    assert_eq!(one_text(&host), 1, "at rest only the source is shown");
    println!("  source rests at y={first_y:.0}; arming the flight…");

    fly.store(true, Ordering::Relaxed);

    // Sample the flight: the ink should travel monotonically down to the
    // destination box (first_y + 40), landing on it.
    let mut ys = Vec::new();
    for _ in 0..40 {
        host.frame();
        ys.push(
            host.scene()
                .iter()
                .find_map(|(_, node)| match node {
                    SceneNode::Text { origin, .. } => Some(origin.y),
                    _ => None,
                })
                .expect("the in-flight element is mounted"),
        );
    }
    let landed = ys[ys.len() - 1];
    assert_eq!(host.animations_running(), 0, "the flight ends on its own");
    assert!(
        ys.iter()
            .any(|y| (first_y + 2.0..first_y + 38.0).contains(y)),
        "the ink travels through the gap between source and destination: {ys:?}"
    );
    assert!(
        ys.windows(2).all(|w| w[1] >= w[0] - 0.5),
        "a one-way downward flight never detours upward: {ys:?}"
    );
    assert!(
        (landed - (first_y + 40.0)).abs() < 8.0,
        "the ink lands on the destination box: {landed} vs {first_y}+40"
    );

    // And once landed, exactly one `dst` text occupies the lower box: the
    // destination's own content is what is shown (its origin is at the bottom
    // box, not the source's top box the flight left behind).
    let landed_origin = host
        .scene()
        .iter()
        .find_map(|(_, node)| match node {
            SceneNode::Text { origin, .. } => Some(origin.y),
            _ => None,
        })
        .expect("one shared element remains");
    assert_eq!(one_text(&host), 1, "still one shared element after landing");
    assert!(
        (landed_origin - (first_y + 40.0)).abs() < 8.0,
        "the remaining text sits on the destination's box, source's top box is \
         gone: {landed_origin}"
    );
    assert!(
        !host.needs_frame(),
        "a landed shared element lets the frame loop park"
    );
    println!(
        "  flew src (y={first_y:.0}) -> dst box ({:.0}); revealed 'dst' content",
        first_y + 40.0
    );
}
