//! Motion, from the prelude alone.
//!
//! Three acts, each one a capability that did not exist before S1 and each one
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
//!
//! Everything below is `floem2::prelude` — no crate below the facade is named.
//! Run with `cargo run -p floem2-motion`.

use floem2::floem_scene::SceneNode;
use floem2::prelude::*;

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

    let child = Child::new(
        ChildKey::index(0),
        LayoutRequest::default(),
        || Box::new(Text::new("hi")),
    );
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
    assert!(mid_alpha < 255, "the exit is mid-fade, not instant: alpha {mid_alpha}");

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

    let side_a = Child::new(
        ChildKey::index(0),
        LayoutRequest::default(),
        || Box::new(Text::new("A")),
    );
    let side_b = Child::new(
        ChildKey::index(1),
        LayoutRequest::default(),
        || Box::new(Text::new("B")),
    );
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
    println!("  A and B both stay on screen during the swap; A unmounts only when its exit finishes");
}
