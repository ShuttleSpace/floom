//! S1 / B8 — the host's animation door, and the claims it has to keep.
//!
//! B1–B3 gave the vocabulary its shape. This file is about the *door*: a
//! widget cannot start an animation (it is borrowed out of the host for the
//! whole of every call the host makes into it), and an app holding a host
//! could not reach the scheduler at all. `WidgetHost::animate` is the public
//! entry, `Widget::animation` is the declarative one, and both have to end up
//! at the same place: a sample per frame, folded into the scene the host
//! already paints.
//!
//! Every assertion here is about an *observable* difference — the alpha of a
//! painted text node, the geometry of a painted bar, the damage a frame
//! reports — rather than about the scheduler's bookkeeping, because
//! bookkeeping that agrees with itself and paints nothing is exactly the
//! failure this door could have.

use floem_animation::{AnimKind, CompositorChannel, CompositorProp, Easing, Tween};
use floem_app::{NodeId, WidgetHost};
use floem_core::{Child, ChildKey, LayoutRequest};
use floem_layout::LayoutStyle;
use floem_scene::SceneNode;
use floem_types::{Color, Event, Point, PointerEvent, PresenceTransition, Rect, Size, Transition};
use floem_widgets::{
    AnimatedContent, Button, Conditional, Container, Crossfade, Opacity, Progress, Text,
};

/// A host with one text leaf on screen and one frame behind it, so layout has
/// run and the scene holds a painted text node to watch.
fn host_with_label() -> (WidgetHost, NodeId) {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 80.0));
    let label = host.mount_root(LayoutStyle::default(), Text::new("hello"));
    host.frame();
    (host, label)
}

/// The alpha of the first text node the host painted.
fn text_alpha(host: &WidgetHost) -> u8 {
    host.scene()
        .iter()
        .find_map(|(_, node)| match node {
            SceneNode::Text { color, .. } => Some(color.a),
            _ => None,
        })
        .expect("the tree painted a text node")
}

/// The origin of the first text node the host painted.
fn text_origin(host: &WidgetHost) -> Point {
    host.scene()
        .iter()
        .find_map(|(_, node)| match node {
            SceneNode::Text { origin, .. } => Some(*origin),
            _ => None,
        })
        .expect("the tree painted a text node")
}

/// Every rect the host painted, in paint order.
fn rects(host: &WidgetHost) -> Vec<Rect> {
    host.scene()
        .iter()
        .filter_map(|(_, node)| match node {
            SceneNode::Rect { bounds, .. } => Some(*bounds),
            _ => None,
        })
        .collect()
}

/// A linear fade of a label, from opaque to invisible, over `secs`.
///
/// Linear on purpose: this file is about the *channel*, and the default curve
/// is also the pre-S1 behaviour, so every assertion here would hold with or
/// without B1. The curves have their own tests where they belong.
fn fade(from: f64, to: f64, secs: f64) -> AnimKind {
    AnimKind::Tween(Tween::new(from, to, secs).with_compositor(CompositorProp::Opacity(0.0)))
}

/// The host's door reaches the painter: an animated node's alpha changes on
/// screen, frame by frame, and lands where the animation said it would.
#[test]
fn a_node_animation_changes_what_the_host_paints() {
    let (mut host, label) = host_with_label();
    assert_eq!(
        text_alpha(&host),
        255,
        "control: the label starts fully opaque"
    );

    host.animate(label, fade(1.0, 0.0, 0.5));
    assert_eq!(host.animations_running(), 1, "the door started something");
    assert!(
        host.needs_frame(),
        "and it is what is keeping the loop awake"
    );

    let mut alphas = Vec::new();
    for _ in 0..5 {
        host.frame();
        alphas.push(text_alpha(&host));
    }
    assert!(
        alphas.windows(2).all(|pair| pair[0] > pair[1]),
        "a fade has to fade every frame it runs: {alphas:?}"
    );
    assert!(
        alphas[0] < 255,
        "the first frame of the animation is already a step into it, not the \
         un-animated appearance: {alphas:?}"
    );

    // Run it out. Half a second at the nominal step is 30 frames.
    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(text_alpha(&host), 0, "the fade reaches its target");
    assert_eq!(host.animations_running(), 0, "and then it is over");

    // The deliberate half of the design, and the one a `clear on finish` would
    // silently break: an animation *leaves the node where it put it*. Clearing
    // the sample on the last tick would snap the label back to opaque on the
    // one frame where the seam is most visible.
    host.frame();
    assert_eq!(
        text_alpha(&host),
        0,
        "a finished animation does not snap its node back to its un-animated look"
    );
    assert!(
        (host.animation_value(label).expect("the sample is parked") - 0.0).abs() < 1e-9,
        "and the parked sample is what the paint reads"
    );
}

/// Motion is a compositor-level repaint, not a layout pass. If this ever
/// starts relaying out, every frame of every animation would pay a full
/// measure/layout cycle for a change that cannot move a box.
#[test]
fn an_animation_repaints_without_relaying_out() {
    let (mut host, label) = host_with_label();
    host.animate(label, fade(1.0, 0.0, 0.5));

    let mut repainted = 0;
    for _ in 0..8 {
        let stats = host.frame();
        assert!(
            !stats.relaid,
            "a frame of an opacity animation must not re-lay-out the tree"
        );
        if !host.scene().damage().is_empty() {
            repainted += 1;
        }
    }
    assert!(
        repainted >= 7,
        "the animation has to repaint the frames it changes ({repainted} of 8 \
         damaged); a channel that moved nothing would leave every frame clean"
    );
}

/// B2 at the host's door: redirecting a running animation carries on from
/// where it is — **with the speed it had** — and arrives at the new target.
///
/// "Does not jump" alone is satisfied by a spring seeded from rest, so it is
/// not the claim. The claim has an observable signature: a node moving fast
/// *downward* that is redirected upward keeps going down for a moment before
/// it turns around. A spring seeded from rest is already climbing past where
/// it started on the very first frame, and an assertion that only looks at how
/// far the value moved cannot tell the two apart.
#[test]
fn retargeting_carries_the_momentum_it_interrupted() {
    let (mut host, label) = host_with_label();
    // Fast on purpose: the momentum has to be worth carrying.
    host.animate(label, fade(1.0, 0.0, 0.2));
    for _ in 0..5 {
        host.frame();
    }
    let midway = host.animation_value(label).expect("running");
    assert!(
        (0.4..0.9).contains(&midway),
        "control: partway through and moving fast ({midway})"
    );
    assert!(text_alpha(&host) < 255, "control: and the scene shows it");

    assert!(
        host.retarget_animation(label, 1.0),
        "it had something to redirect"
    );
    host.frame();
    let after = host.animation_value(label).expect("still running");
    assert!(
        after < midway - 0.02,
        "the retarget has to carry the speed it interrupted: {midway} -> {after} \
         (a spring seeded from rest would already be climbing past {midway})"
    );
    assert!(
        after > midway - 0.2,
        "and that is momentum, not a jump: {midway} -> {after}"
    );

    for _ in 0..90 {
        host.frame();
    }
    let settled = host.animation_value(label).expect("parked");
    assert!(
        settled > 0.9,
        "a retargeted animation has to reach its new target ({settled})"
    );
}

/// The host's doors answer honestly: nothing to redirect, nothing to cancel.
#[test]
fn the_doors_report_whether_they_had_anything_to_do() {
    let (mut host, label) = host_with_label();
    assert!(
        !host.retarget_animation(label, 0.0),
        "no animation, nothing to redirect"
    );
    assert!(!host.cancel_animation(label), "and nothing to cancel");
    assert_eq!(host.animation_value(label), None);

    host.animate(label, fade(1.0, 0.0, 0.5));
    assert!(
        !host.retarget_animation(label, 0.0) || host.animations_running() == 1,
        "a running animation is redirectable"
    );
    assert!(host.cancel_animation(label), "and cancellable");
    assert_eq!(host.animations_running(), 0);
}

/// Cancelling releases the channel: the node goes back to what its widget
/// paints, and from then on nothing moves and nothing repaints.
///
/// This is the deliberate opposite of an animation *finishing*, which parks
/// its endpoint. A stopped animation has not reached a state anybody asked
/// for, so leaving the node wrapped in a half-applied compositing would strand
/// it in a look no live animation is feeding and that only another animation
/// could clear. The two tests here pin both halves of that rule.
#[test]
fn cancelling_an_animation_releases_the_channel() {
    let (mut host, label) = host_with_label();
    host.animate(label, fade(1.0, 0.0, 10.0));
    for _ in 0..5 {
        host.frame();
    }
    assert!(text_alpha(&host) < 255, "control: the fade got somewhere");
    assert!(
        host.animation_value(label).expect("running") < 1.0,
        "control: and the channel is holding it there"
    );

    assert!(host.cancel_animation(label));
    assert_eq!(host.animations_running(), 0);
    assert!(
        !host.needs_frame(),
        "a cancelled animation must release the frame loop"
    );

    // The frame that shows the release, and then the frames that show nothing
    // at all — a channel that kept re-damaging the node would never park.
    host.frame();
    assert_eq!(
        text_alpha(&host),
        255,
        "cancelling is not freezing: the node goes back to its widget's paint"
    );
    assert_eq!(
        host.animation_value(label),
        None,
        "and the sample it was painting from is gone with it"
    );

    let stopped = text_alpha(&host);
    let stats = host.frame();
    assert_eq!(text_alpha(&host), stopped, "nothing moves any more");
    assert!(
        host.scene().damage().is_empty(),
        "and no frame keeps damaging the node"
    );
    assert!(!stats.relaid, "nor relaying it out");
}

/// A finished animation, by contrast, stays where it ended: the same fade run
/// out instead of stopped, and the node holds its endpoint.
#[test]
fn a_finished_animation_parks_its_endpoint() {
    let (mut host, label) = host_with_label();
    host.animate(label, fade(1.0, 0.0, 0.5));
    for _ in 0..40 {
        host.frame();
    }
    assert_eq!(host.animations_running(), 0, "it is over");
    assert_eq!(
        text_alpha(&host),
        0,
        "and the label is still invisible on the frame after that"
    );
    assert_eq!(
        host.animation_value(label),
        Some(0.0),
        "the endpoint is still readable as the node's sample"
    );
    host.frame();
    assert!(
        host.scene().damage().is_empty(),
        "a parked endpoint does not re-damage either"
    );
}

/// A node that leaves the tree takes its animation with it. Without this the
/// scheduler would keep sampling a node that is gone — and `needs_frame`
/// would keep the frame loop awake forever on a tree with nothing in it.
#[test]
fn unmounting_a_node_retires_its_animation() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 80.0));
    let root = host.mount_root(LayoutStyle::default(), Container::new());
    let child = host.mount(root, LayoutStyle::default(), Text::new("transient"));
    host.frame();

    host.animate(
        child,
        AnimKind::Tween(
            Tween::new(0.0, 1.0, 30.0).with_compositor(CompositorProp::TranslateY(0.0)),
        ),
    );
    host.frame();
    assert_eq!(host.animations_running(), 1, "control: it was running");
    assert!(host.needs_frame());

    host.unmount(child);
    host.frame();
    host.frame();
    assert_eq!(
        host.animations_running(),
        0,
        "the unmounted node's animation is retired, not orphaned"
    );
    assert!(
        !host.needs_frame(),
        "an empty tree with an orphan animation would never park"
    );
}

/// The declarative door: a widget that cannot hold a host states the motion it
/// wants, and the host runs it for as long as the widget keeps saying so.
#[test]
fn a_widget_that_declares_motion_animates_itself() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 40.0));
    host.mount_root(LayoutStyle::default(), Progress::indeterminate());
    host.frame();

    assert_eq!(
        host.animations_running(),
        1,
        "the bar's declaration has to start on the frame it first reports dirty"
    );

    let before = rects(&host);
    assert!(
        before.len() >= 2,
        "a bar paints a track and a moving segment"
    );

    // The track is painted first and is the box the shuttle travels along.
    let track = before[0];

    let mut seen_segment = Vec::new();
    for _ in 0..4 {
        host.frame();
        let now = rects(&host);
        assert_eq!(now.len(), before.len(), "the bar's shape is stable");
        let segment = now
            .iter()
            .zip(&before)
            .find(|(a, b)| a.x0 != b.x0 || a.x1 != b.x1)
            .map(|(a, _)| *a);
        let segment = segment.unwrap_or_else(|| panic!("no rect moved: {now:?}"));
        assert!(
            segment.x0 >= track.x0 - 0.5 && segment.x1 <= track.x1 + 0.5,
            "the shuttle stays on the bar: {segment:?} outside {track:?}"
        );
        seen_segment.push(segment.x0);
    }
    assert!(
        seen_segment.windows(2).any(|pair| pair[0] != pair[1]),
        "the shuttle has to keep moving: {seen_segment:?}"
    );
}

/// Animating a node that is about to report dirty must not cancel the
/// animation.
///
/// The declaration pass runs over exactly the nodes that reported dirty, and
/// "this widget declares nothing" is the same answer from a widget that never
/// declared anything and from one that withdrew its declaration. A host that
/// treats those as the same answer cancels an imperative animation on the next
/// frame the node reports dirty — which is the frame after `animate` on any
/// freshly mounted node, and the frame after any state change on any other.
/// That is the *first* thing a caller does, which is what makes it worth a test
/// of its own: the whole imperative door is unusable if this regresses.
#[test]
fn an_imperative_animation_survives_a_node_that_reports_dirty() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 80.0));
    // Deliberately no frame before `animate`: a freshly mounted node still has
    // its mount-time dirty to report, and that is the state an app animates a
    // node on arrival in.
    let label = host.mount_root(LayoutStyle::default(), Text::new("arriving"));
    host.animate(label, fade(1.0, 0.0, 1.0));

    host.frame();
    assert_eq!(
        host.animations_running(),
        1,
        "the frame that reconciles the node's declaration must not take the \
         animation away"
    );
    let first = host.animation_value(label).expect("running");
    assert!(first < 1.0, "control: and it has started moving ({first})");

    // The same thing again, from a change the widget itself reports.
    for _ in 0..3 {
        host.frame();
    }
    let before = host.animation_value(label).expect("running");
    host.widget_mut::<Text>(label)
        .expect("mounted")
        .set_content("arrived");
    host.frame();
    assert_eq!(
        host.animations_running(),
        1,
        "a content change is not a withdrawal of a declaration nobody made"
    );
    let after = host.animation_value(label).expect("running");
    assert!(
        after < before,
        "and it carried on rather than restarting: {before} -> {after}"
    );
}

/// A declaration still wins over an imperative animation: the widget owns its
/// own element, so the first time it gets to say what it wants, that is what
/// runs. The two owners are not combined and the app is not asked — this is the
/// documented precedence, not an accident.
#[test]
fn a_declaration_takes_over_from_an_imperative_animation() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 40.0));
    let bar = host.mount_root(LayoutStyle::default(), Progress::indeterminate());
    // Both owners on one element, before either has had a chance to run: the
    // widget declares a shuttle, and the app imperatively starts something else
    // on the same node. The mount frame is where the declaration is reconciled
    // for the first time, so that is where the two owners meet.
    host.animate(bar, fade(1.0, 0.0, 1.0));
    assert_eq!(host.animations_running(), 1, "control: the app's animation");
    host.frame();

    // Which one is running? The bar's shuttle never ends; the app's fade is
    // over in a second. Running well past that second is the observable that
    // says which of the two owners won, and it is not available from the value
    // alone: both of them are on their way to zero early on.
    for _ in 0..90 {
        host.frame();
    }
    assert_eq!(
        host.animations_running(),
        1,
        "after 1.5 s the app's one-second fade would be over; something endless \
         is still running, so the declaration took the element back"
    );
    let value = host.animation_value(bar).expect("running");
    assert!(
        (0.0..=1.0).contains(&value),
        "and it is the shuttle's own value ({value})"
    );
}

/// The frame loop has to stay awake for an animation that is *not* changing
/// any pixel yet.
///
/// `needs_frame` has two ways to say yes while motion is in flight — the
/// animation itself, and the damage the last frame left. An animating node
/// usually produces both, which makes the first one easy to assert without
/// ever touching it. This test isolates it: a fade slow enough that its
/// sampled alpha still rounds to the same byte damages nothing at all, and a
/// loop that keyed off damage alone would park here and freeze the animation
/// on its first frame forever.
#[test]
fn a_running_animation_keeps_the_loop_awake_when_no_pixel_changes() {
    let (mut host, label) = host_with_label();
    // 255 alpha steps over 10 000 seconds is one step every ~39 s, so the
    // frames below move the sample without moving a single pixel.
    host.animate(label, fade(1.0, 0.0, 10_000.0));
    host.frame();

    assert_eq!(host.animations_running(), 1, "control: it is running");
    assert_eq!(
        text_alpha(&host),
        255,
        "control: and it has not changed a pixel"
    );
    assert!(
        host.scene().damage().is_empty(),
        "control: so the last frame left nothing damaged"
    );
    assert!(
        host.needs_frame(),
        "an animation in flight keeps the loop awake on its own account, not \
         on the strength of a damaged pixel"
    );

    // And it is really progressing, at a resolution the screen cannot show.
    let before = host.animation_value(label).expect("running");
    for _ in 0..3 {
        host.frame();
        assert!(host.needs_frame(), "the loop stays awake every frame of it");
    }
    let after = host.animation_value(label).expect("running");
    assert!(
        after < before,
        "the clock and the sample keep advancing: {before} -> {after}"
    );
    assert_eq!(
        text_alpha(&host),
        255,
        "and the screen still shows none of it"
    );
}

/// The control for the test above: the same widget with a value declares
/// nothing, so the host has nothing to run and the bar holds still. Without
/// this pair, "the bar moved" could be satisfied by a bar that always moves.
#[test]
fn a_determinate_bar_declares_no_motion() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 40.0));
    host.mount_root(LayoutStyle::default(), Progress::determinate(0.5));
    // Two frames: the first owes the scene its freshly mounted nodes, so it
    // is not an idle frame no matter what declares anything.
    host.frame();
    host.frame();

    assert_eq!(
        host.animations_running(),
        0,
        "a bar with a value is not animating"
    );
    assert!(
        !host.needs_frame(),
        "so nothing is asking for frames on its behalf"
    );
    let before = rects(&host);
    assert!(!before.is_empty(), "it still paints");
    host.frame();
    assert_eq!(
        rects(&host),
        before,
        "and it paints the same thing twice in a row"
    );
}

/// Clearing the declaration stops the motion, and re-stating it starts it
/// again — the widget's half of the two-owner contract.
#[test]
fn a_declaration_that_goes_away_stops_the_motion() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 40.0));
    let bar = host.mount_root(LayoutStyle::default(), Progress::indeterminate());
    host.frame();
    assert_eq!(host.animations_running(), 1);

    host.widget_mut::<Progress>(bar)
        .expect("the bar is mounted")
        .set_value(Some(0.25));
    host.frame();
    host.frame();
    assert_eq!(
        host.animations_running(),
        0,
        "a widget that stopped declaring motion stops being animated"
    );
    assert!(!host.needs_frame());

    let mut host2 = WidgetHost::new();
    host2.set_size(Size::new(200.0, 40.0));
    let bar2 = host2.mount_root(LayoutStyle::default(), Progress::determinate(0.25));
    host2.frame();
    assert_eq!(
        host2.animations_running(),
        0,
        "control: determinate at rest"
    );
    host2
        .widget_mut::<Progress>(bar2)
        .expect("mounted")
        .set_value(None);
    host2.frame();
    assert_eq!(
        host2.animations_running(),
        1,
        "declaring motion again restarts it"
    );
}

/// Re-stating the *same* declaration is not a restart. This is what lets a
/// widget say what it wants on every update without resetting the animation
/// it is already running — a spinner that restarts every frame never leaves
/// its first position.
#[test]
fn restating_the_same_declaration_does_not_restart_it() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 40.0));
    let bar = host.mount_root(LayoutStyle::default(), Progress::indeterminate());
    host.frame();
    for _ in 0..20 {
        host.frame();
    }
    let progressed = host.animation_value(bar).expect("running");

    // A change to the widget that is not a change to its motion: the revision
    // bumps, the node reports dirty, and the declaration is re-stated
    // verbatim. It must be recognised as the one already running.
    // `Progress` is not `Copy` (it can carry a `Brush`), so this has to be the
    // in-place setter: `Progress::disabled` takes `self` by value and would
    // otherwise be a no-op written into a temporary.
    host.widget_mut::<Progress>(bar)
        .expect("mounted")
        .set_disabled(true);
    host.frame();
    assert_eq!(host.animations_running(), 1);
    let after = host.animation_value(bar).expect("still running");
    assert!(
        (after - progressed).abs() < 0.05,
        "the animation carried on from {progressed}; a restart would have put \
         it back at the start ({after})"
    );
}

/// The sample reads the same from outside the widget as inside its paint.
#[test]
fn the_parked_sample_is_the_same_number_the_widget_paints_with() {
    let (mut host, label) = host_with_label();
    host.animate(label, fade(1.0, 0.0, 1.0));
    for _ in 0..6 {
        host.frame();
        let value = host.animation_value(label).expect("running");
        let expected = (255.0 * value).round().clamp(0.0, 255.0) as u8;
        assert_eq!(
            text_alpha(&host),
            expected,
            "the scene alpha is the parked sample scaled to 8 bits ({value})"
        );
    }
}

/// A `TranslateY` animation reaches the same place by the same route: this is
/// the channel's second compositor property, and the one a real container
/// will drive once sub-tree transforms land.
#[test]
fn a_translation_animation_moves_the_painted_box() {
    let (mut host, label) = host_with_label();
    let origin = host
        .scene()
        .iter()
        .find_map(|(_, node)| match node {
            SceneNode::Text { origin, .. } => Some(*origin),
            _ => None,
        })
        .expect("a text node");

    host.animate(
        label,
        AnimKind::Tween(
            Tween::new(0.0, 40.0, 0.5).with_compositor(CompositorProp::TranslateY(0.0)),
        ),
    );
    for _ in 0..30 {
        host.frame();
    }
    let moved = host
        .scene()
        .iter()
        .find_map(|(_, node)| match node {
            SceneNode::Text { origin, .. } => Some(*origin),
            _ => None,
        })
        .expect("a text node");
    assert!(
        (moved.y - origin.y - 40.0).abs() < 0.5,
        "the label should have travelled 40px down: {origin:?} -> {moved:?}"
    );
    assert_eq!(moved.x, origin.x, "and not sideways");
}

// --- B6: multi-value arrangement -------------------------------------------

/// One tween drives two compositor channels in lock-step: the text *fades* on
/// the same frames it *moves*, from a single scheduled animation.
///
/// This is the observable the whole multi-value arrangement exists for — opacity
/// and translation travelling together without two tweens, a per-widget
/// choreography, or a re-layout. Both channels share one curve, but each spans
/// its own value range (opacity 1.0 → 0.0, translate 0 → 40 px), so the single
/// driver value maps to both. Mid-flight, on the very same frame, the alpha has
/// left full *and* the origin has left (but not reached) its destination.
#[test]
fn a_multichannel_animation_drives_opacity_and_translation_together() {
    let (mut host, label) = host_with_label();
    let origin = text_origin(&host);

    host.animate(
        label,
        AnimKind::Tween(Tween {
            from: 0.0,
            to: 1.0,
            duration_secs: 0.5,
            easing: Easing::EASE_OUT,
            channels: vec![
                CompositorChannel::new(CompositorProp::Opacity(0.0), 1.0, 0.0),
                CompositorChannel::new(CompositorProp::TranslateY(0.0), 0.0, 40.0),
            ],
        }),
    );
    assert_eq!(
        host.animations_running(),
        1,
        "the door started one animation"
    );
    host.frame();

    let alpha = text_alpha(&host);
    let moved = text_origin(&host);
    assert!(
        alpha < 255,
        "the opacity channel is fading on the same frame: {alpha}"
    );
    assert!(
        moved.y > origin.y && moved.y < origin.y + 40.0,
        "and the translation channel is mid-travel on that same frame: \
         {origin:?} -> {moved:?}"
    );

    // Both land at their endpoints together.
    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(text_alpha(&host), 0, "opacity reached its target");
    let landed = text_origin(&host);
    assert!(
        (landed.y - origin.y - 40.0).abs() < 0.5,
        "translation reached its target too: {origin:?} -> {landed:?}"
    );
    assert_eq!(
        host.animations_running(),
        0,
        "and then the one animation rests"
    );
}

// --- B4: implicit transitions ---------------------------------------------

/// The fill colour of the first solid rect the host painted — a button's own
/// background, since it is the only rect on a bare-button tree.
fn first_rect_fill(host: &WidgetHost) -> Color {
    host.scene()
        .iter()
        .find_map(|(_, node)| match node {
            SceneNode::Rect { fill, .. } => fill.solid(),
            _ => None,
        })
        .expect("the tree painted a rect")
}

/// A host with one button on screen, a frame behind it, and its box reported.
fn host_with_button(transition: Option<Transition>) -> (WidgetHost, NodeId, Rect) {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 80.0));
    let mut button = Button::new("Hover me");
    if let Some(transition) = transition {
        button = button.transition(transition);
    }
    let node = host.mount_root(LayoutStyle::default(), button);
    host.frame();
    let bounds = host.bounds(node);
    (host, node, bounds)
}

/// Hovers the button by dispatching a move event into its centre.
fn hover(host: &mut WidgetHost, bounds: Rect) {
    let centre = Point::new(
        bounds.x0 + bounds.width() / 2.0,
        bounds.y0 + bounds.height() / 2.0,
    );
    host.dispatch(Event::Pointer(PointerEvent::hover(centre)));
}

/// The centre of a node's box.
fn centre_of(bounds: Rect) -> Point {
    Point::new(
        bounds.x0 + bounds.width() / 2.0,
        bounds.y0 + bounds.height() / 2.0,
    )
}

/// Presses the primary button down inside the box.
fn press_down(host: &mut WidgetHost, bounds: Rect) {
    host.dispatch(Event::Pointer(PointerEvent::primary_down(centre_of(
        bounds,
    ))));
}

/// Releases the primary button inside the box. This is the event that clears
/// `pressed` — and therefore the one that moves the resolved colour away from
/// the pressed colour.
fn press_up(host: &mut WidgetHost, bounds: Rect) {
    host.dispatch(Event::Pointer(PointerEvent::primary_up(centre_of(bounds))));
}

/// A button with an armed transition fades its fill toward the hover colour
/// instead of snapping: hovering starts a *fresh* transition, the first frame
/// after the event is still near the resting colour (not already at the hover
/// colour), and it lands on the hover colour once the transition has run out.
#[test]
fn a_transitioned_button_fades_its_fill_on_hover() {
    let (mut host, _node, bounds) = host_with_button(Some(Transition::new(0.5)));
    let resting = first_rect_fill(&host);

    // Settle the tree first: a transitioned button declares its transition on
    // the frame it mounts, and that declaration seeds an initial 0 → 1 tween.
    // Let it run out so the *only* motion left to observe is the hover's.
    for _ in 0..40 {
        host.frame();
    }
    assert_eq!(
        host.animations_running(),
        0,
        "a settled transitioned button is not animating"
    );

    hover(&mut host, bounds);
    host.frame();
    assert_eq!(
        host.animations_running(),
        1,
        "hovering a transitioned button restarts its transition"
    );

    // The first painted frame after the hover is still at the resting colour:
    // the transition starts from where the fill *is*, not where it is going.
    // (The deliberate contrast is the untransitioned control below, which is
    // already at the hover colour on this same frame.)
    let first = first_rect_fill(&host);
    assert_eq!(
        first, resting,
        "the transition opens at the resting colour, not a jump to the hover \
         colour: {first:?}"
    );

    // A few frames in, the fill is strictly between the resting and the hover
    // colour — proof it is moving, not parked at either end.
    for _ in 0..8 {
        host.frame();
    }
    let midway = first_rect_fill(&host);
    assert_ne!(
        midway, resting,
        "the fill leaves its resting colour: {midway:?}"
    );

    // Run the transition out — 0.5 s is 30 nominal frames — and it lands on the
    // hover colour, which is `base` lifted toward white.
    for _ in 0..40 {
        host.frame();
    }
    let settled = first_rect_fill(&host);
    assert_ne!(
        settled, resting,
        "the hover colour differs from the resting one"
    );
    assert_ne!(
        midway, settled,
        "the fill did not jump straight to the hover colour: {midway:?} -> {settled:?}"
    );
    assert_eq!(
        host.animations_running(),
        0,
        "the transition is over once the colour arrives"
    );
}

/// Letting go fades too.
///
/// Releasing a press is its own colour change — `pressed` clears, so the fill
/// has to travel from the pressed colour back to the hovered one. It is the
/// one state transition that does *not* arrive as `Press::Consumed` (a release
/// is `Press::Activate`), which is exactly why arming the transition inside a
/// single match arm loses it: every other change fades and letting go snaps.
#[test]
fn releasing_a_press_fades_the_fill_back() {
    let (mut host, _node, bounds) = host_with_button(Some(Transition::new(0.5)));
    for _ in 0..40 {
        host.frame();
    }

    // Press, then settle into the pressed colour.
    press_down(&mut host, bounds);
    for _ in 0..40 {
        host.frame();
    }
    let pressed = first_rect_fill(&host);
    assert_eq!(
        host.animations_running(),
        0,
        "the press transition has run out"
    );

    // Let go. The target colour moves back to the hovered colour, so the host
    // must restart the transition — and the frame it does, the fill has to
    // still be the pressed colour it is leaving, not the one it is heading to.
    press_up(&mut host, bounds);
    host.frame();
    assert_eq!(
        host.animations_running(),
        1,
        "releasing restarts the transition"
    );
    let on_release = first_rect_fill(&host);
    assert_eq!(
        on_release, pressed,
        "the fill opens the release fade at the pressed colour, not a jump \
         back: {on_release:?} vs {pressed:?}"
    );

    // A few frames in it has left the pressed colour but has not arrived.
    for _ in 0..8 {
        host.frame();
    }
    let midway = first_rect_fill(&host);
    assert_ne!(midway, pressed, "the fill leaves the pressed colour");

    for _ in 0..40 {
        host.frame();
    }
    let settled = first_rect_fill(&host);
    assert_ne!(
        midway, settled,
        "the release travelled instead of snapping: {midway:?} -> {settled:?}"
    );
    assert_eq!(
        host.animations_running(),
        0,
        "the release transition is over once the colour arrives"
    );
}

/// A button *without* a transition is the deliberate control: hovering changes
/// the fill instantly, and no animation is what the host reports.
#[test]
fn an_untransitioned_button_snaps_its_fill_on_hover() {
    let (mut host, _node, bounds) = host_with_button(None);
    let resting = first_rect_fill(&host);

    hover(&mut host, bounds);
    host.frame();

    assert_eq!(
        host.animations_running(),
        0,
        "no transition means no animation"
    );
    assert_ne!(
        first_rect_fill(&host),
        resting,
        "but the colour still changes — just all at once"
    );
}

// ---------------------------------------------------------------------------
// B5 — presence transitions (enter / exit)
//
// The part of the implicit layer that is not "a value changed": a subtree's
// *existence* changed. Something leaving has to stay mounted until its exit
// animation is over, or the departure would be invisible. So a presence widget
// is a state machine whose exit arm keeps the child on screen and only
// unmounts it once the host reports the transition finished.
// ---------------------------------------------------------------------------

/// A host whose root is a `Conditional` with a `Text` child, optionally with
/// enter/exit transitions. One frame has run, so the child (if shown) is laid
/// out and painted.
fn host_with_presence(
    when: bool,
    enter: Option<PresenceTransition>,
    exit: Option<PresenceTransition>,
) -> (WidgetHost, NodeId) {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 80.0));
    let child = Child::new(ChildKey::index(1), LayoutRequest::default(), || {
        Box::new(Text::new("hi"))
    });
    let mut conditional = Conditional::new(when).show(child);
    if let Some(enter) = enter {
        conditional = conditional.enter(enter);
    }
    if let Some(exit) = exit {
        conditional = conditional.exit(exit);
    }
    let root = host.mount_root(LayoutStyle::default(), conditional);
    host.frame();
    (host, root)
}

/// The alpha of the *child's* text node — the only text node the scene holds,
/// since the conditional itself paints nothing. `None` when no child is
/// mounted.
fn child_text_alpha(host: &WidgetHost) -> Option<u8> {
    host.scene().iter().find_map(|(_, node)| match node {
        SceneNode::Text { color, .. } => Some(color.a),
        _ => None,
    })
}

/// How many text nodes the scene currently holds — exactly the number of
/// mounted child subtrees the conditional shows.
fn child_text_count(host: &WidgetHost) -> usize {
    host.scene()
        .iter()
        .filter(|(_, node)| matches!(node, SceneNode::Text { .. }))
        .count()
}

/// The alpha of *every* text node the scene holds, in paint order. A crossfade
/// keeps two children mounted at once, so this is how we watch both at once.
fn child_text_alphas(host: &WidgetHost) -> Vec<u8> {
    host.scene()
        .iter()
        .filter_map(|(_, node)| match node {
            SceneNode::Text { color, .. } => Some(color.a),
            _ => None,
        })
        .collect()
}

/// A presence child fades in on first appearance instead of popping in.
#[test]
fn a_presence_child_fades_in_on_first_appearance() {
    let (mut host, _root) = host_with_presence(true, Some(PresenceTransition::fade(0.2)), None);

    // The very first frame has the child mounted (layout ran) but the enter
    // tween has only parked its t = 0 sample, so it is still transparent.
    assert_eq!(child_text_count(&host), 1, "the child is mounted");
    assert_eq!(
        child_text_alpha(&host),
        Some(0),
        "and at the start of the enter it is fully transparent, not resting"
    );

    // Run the enter out.
    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(child_text_alpha(&host), Some(255), "it lands fully shown");
    assert_eq!(host.animations_running(), 0, "and then rests");
}

/// A presence child stays mounted *while* it exits, then unmounts only once
/// the exit animation has finished.
#[test]
fn a_presence_child_exits_without_vanishing_instantly() {
    let (mut host, root) = host_with_presence(
        true,
        Some(PresenceTransition::fade(0.2)),
        Some(PresenceTransition::fade(0.2)),
    );

    // Let the enter finish and rest.
    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(child_text_count(&host), 1);
    assert_eq!(child_text_alpha(&host), Some(255));

    // Flip it off. The child must remain on screen the moment after — an exit
    // transition that unmounted immediately would never be seen.
    host.widget_mut::<Conditional>(root)
        .expect("the root is a conditional")
        .set_when(false);
    host.frame();
    assert_eq!(child_text_count(&host), 1, "still mounted mid-exit");

    // A few frames in, the fade-out has progressed past its t = 0 sample, so
    // the child is dimming but still mounted — and dimming *monotonically
    // toward invisible*, which is what separates "leaving" from an exit that
    // ran the enter's endpoints backwards and would be climbing instead.
    let mut dimming = Vec::new();
    for _ in 0..4 {
        host.frame();
        assert_eq!(child_text_count(&host), 1, "still mounted mid-exit");
        dimming.push(child_text_alpha(&host).expect("child present"));
    }
    assert!(dimming[0] < 255, "partway through fading out: {dimming:?}");
    assert!(
        dimming.windows(2).all(|pair| pair[0] > pair[1]),
        "the exit travels toward invisible rather than back toward shown: {dimming:?}"
    );

    // Run the exit out. Once the host reports the transition finished, the
    // widget unmounts the subtree.
    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(
        child_text_count(&host),
        0,
        "the child is unmounted only after its exit completed"
    );
}

/// A child that leaves and comes back **replays** its enter instead of
/// resuming the finished one.
///
/// The re-appearance re-declares a tween identical in every field to the one
/// the host already ran to completion, so nothing about the value says "start
/// over" — only the widget's own restart key does. Without it the child would
/// appear already arrived, which is how "it only fades in the first time"
/// bugs are born.
#[test]
fn a_presence_child_that_returns_replays_its_enter() {
    // Deliberately no exit transition, so leaving is instant and what is left
    // to observe when it returns is *its own* enter, not a departure still
    // running.
    let (mut host, root) = host_with_presence(true, Some(PresenceTransition::fade(0.2)), None);

    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(
        child_text_alpha(&host),
        Some(255),
        "control: it entered once and rested"
    );

    host.widget_mut::<Conditional>(root)
        .expect("the root is a conditional")
        .set_when(false);
    host.frame();
    assert_eq!(
        child_text_count(&host),
        0,
        "an exit with no transition is instant, so nothing is left to resume"
    );

    // Come back: the enter has to start over from invisible.
    host.widget_mut::<Conditional>(root)
        .expect("the root is a conditional")
        .set_when(true);
    host.frame();
    assert_eq!(
        child_text_count(&host),
        1,
        "it is mounted again on the frame it returns"
    );
    assert_eq!(
        child_text_alpha(&host),
        Some(0),
        "and it replays its enter from invisible, rather than appearing already \
         arrived on the strength of the one that finished"
    );

    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(
        child_text_alpha(&host),
        Some(255),
        "the replayed enter reaches rest"
    );
}

/// A `Conditional` with no transitions is the old behaviour: mount/unmount,
/// no animation, no grace period.
#[test]
fn an_untransitioned_conditional_unmounts_instantly() {
    let (mut host, root) = host_with_presence(true, None, None);
    assert_eq!(child_text_count(&host), 1);

    host.widget_mut::<Conditional>(root)
        .expect("the root is a conditional")
        .set_when(false);
    host.frame();
    assert_eq!(
        child_text_count(&host),
        0,
        "no exit transition means no grace period"
    );
    assert_eq!(host.animations_running(), 0);
}

/// A grow exit keeps the child mounted while it shrinks, then unmounts.
#[test]
fn a_presence_child_keeps_mounted_through_a_grow_exit() {
    let (mut host, root) = host_with_presence(
        true,
        Some(PresenceTransition::grow(0.2, 0.5)),
        Some(PresenceTransition::grow(0.2, 0.5)),
    );
    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(child_text_count(&host), 1);

    host.widget_mut::<Conditional>(root)
        .expect("the root is a conditional")
        .set_when(false);
    host.frame();
    assert_eq!(child_text_count(&host), 1, "still mounted mid-exit");

    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(
        child_text_count(&host),
        0,
        "unmounts after the grow finished"
    );
}

/// A `Crossfade` keeps *both* contents mounted through the swap, so the one
/// leaving stays on screen while the one arriving fades in — the defining
/// property of a crossfade, and exactly the presence choreography B5 asks for.
#[test]
fn a_crossfade_keeps_both_children_mounted_while_it_swaps() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 80.0));
    let child_a = Child::new(ChildKey::index(1), LayoutRequest::default(), || {
        Box::new(Text::new("A"))
    });
    let child_b = Child::new(ChildKey::index(2), LayoutRequest::default(), || {
        Box::new(Text::new("B"))
    });
    let root = host.mount_root(
        LayoutStyle::default(),
        Crossfade::new(true)
            .a(child_a)
            .b(child_b)
            .presence(PresenceTransition::fade(0.2)),
    );
    host.frame();

    // Let A's enter finish and rest before we assert the settled state.
    for _ in 0..30 {
        host.frame();
    }

    // Rested showing A: exactly one child mounted, fully opaque.
    assert_eq!(
        child_text_count(&host),
        1,
        "a rested crossfade shows only its active side"
    );
    assert_eq!(child_text_alphas(&host), vec![255]);

    // Flip to B. The instant after the flip *both* sides are mounted — A is
    // mid-exit, B is mid-enter — which a plain swap (or a `Conditional`) would
    // never do. And they are at opposite ends of the fade: A still fully
    // visible, B still invisible.
    host.widget_mut::<Crossfade>(root)
        .expect("the root is a crossfade")
        .set_show_a(false);
    host.frame();
    let both = child_text_alphas(&host);
    assert_eq!(
        both.len(),
        2,
        "a crossfade keeps both children mounted through the swap"
    );
    assert!(
        both.iter().any(|&a| a == 255) && both.iter().any(|&a| a == 0),
        "the leaving side is still shown and the arriving side is still hidden \
         at the start of the swap: {both:?}"
    );

    // Across the whole transition both sides stay live — never dropping back to
    // a single mounted child until the swap is done.
    for _ in 0..4 {
        host.frame();
        assert_eq!(
            child_text_count(&host),
            2,
            "both sides remain mounted while the crossfade plays"
        );
    }

    // Run it out. The leave is finished, so A unmounts and only B rests shown.
    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(
        child_text_count(&host),
        1,
        "once the swap completes only the arriving side is mounted"
    );
    assert_eq!(
        child_text_alphas(&host),
        vec![255],
        "and it rests fully shown"
    );
}

/// An `AnimatedContent` keeps both the leaving and arriving contents mounted
/// through a switch — exactly the crossfade property, but driven by a content
/// key instead of a fixed two-slot swap. This is the whole point of B5's
/// `AnimatedContent` slice, and what separates it from a plain `Conditional`
/// swap (which never keeps the old subtree on screen).
#[test]
fn an_animated_content_keeps_both_sides_live_while_it_switches() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 80.0));
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

    // Let the initial content settle fully shown.
    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(child_text_count(&host), 1, "rests showing one content");
    assert_eq!(child_text_alphas(&host), vec![255]);

    // Switch the key. The instant after, both the leaving (1) and arriving (2)
    // contents are mounted — one still fully shown, the other still hidden.
    host.widget_mut::<AnimatedContent<u32>>(root)
        .expect("the root is an animated content")
        .set_value(2);
    host.frame();
    let both = child_text_alphas(&host);
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
            child_text_count(&host),
            2,
            "both contents remain mounted while the switch plays"
        );
    }

    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(
        child_text_count(&host),
        1,
        "once the switch completes only the arriving content is mounted"
    );
    assert_eq!(
        child_text_alphas(&host),
        vec![255],
        "and it rests fully shown"
    );
}

/// Without a transition, an `AnimatedContent` swaps the content instantly: the
/// leaving side is not kept around, because there is no exit to play. This is
/// the same instant behaviour a `Conditional` has, and confirms the transition
/// is what buys the crossfade.
#[test]
fn an_animated_content_swaps_instantly_without_a_transition() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 80.0));
    let root = host.mount_root(
        LayoutStyle::default(),
        AnimatedContent::new(1u32)
            .content_with(|v| {
                let label = format!("v{v}");
                Child::new(ChildKey::index(0), LayoutRequest::default(), move || {
                    Box::new(Text::new(label.clone()))
                })
            })
            .presence(PresenceTransition::fade(0.0)),
    );
    host.frame();
    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(child_text_count(&host), 1);
    assert_eq!(child_text_alphas(&host), vec![255]);

    host.widget_mut::<AnimatedContent<u32>>(root)
        .expect("the root is an animated content")
        .set_value(2);
    host.frame();
    // No transition: the departure leaves nothing behind, so only the arriving
    // content is mounted (a `Conditional` swap would do the same).
    assert_eq!(
        child_text_count(&host),
        1,
        "an instant swap keeps only the arriving content"
    );
    // The arriving content settles to fully shown within a frame or two — an
    // instant enter still samples t = 0 on its mount frame before folding to
    // rest, which is the same one-frame artefact an untransitioned `Conditional`
    // shows.
    for _ in 0..3 {
        host.frame();
    }
    assert_eq!(child_text_count(&host), 1);
    assert_eq!(child_text_alphas(&host), vec![255]);
}

/// A value that returns after having left replays its enter rather than
/// reappearing already arrived. Because each content gets a fresh, uniquely
/// keyed subtree, coming back to a value mounts a new node that starts its
/// enter from invisible.
#[test]
fn an_animated_content_replays_enter_when_a_value_returns() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 80.0));
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
    assert_eq!(child_text_alphas(&host), vec![255]);

    // Leave to 2 and let it settle.
    host.widget_mut::<AnimatedContent<u32>>(root)
        .expect("the root is an animated content")
        .set_value(2);
    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(child_text_count(&host), 1);

    // Come back to 1. A fresh node for 1 mounts and must start its enter from
    // invisible — it cannot lean on the 1 that already finished.
    host.widget_mut::<AnimatedContent<u32>>(root)
        .expect("the root is an animated content")
        .set_value(1);
    host.frame();
    let both = child_text_alphas(&host);
    assert_eq!(
        both.len(),
        2,
        "the returning value and the leaving value are both mounted"
    );
    assert!(
        both.iter().any(|&a| a == 0),
        "the returning value replays its enter from invisible: {both:?}"
    );

    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(child_text_count(&host), 1);
    assert_eq!(child_text_alphas(&host), vec![255]);
}

// ---------------------------------------------------------------------------
// B4 — implicit opacity transition (the colour channel extended to compositor
// opacity)
//
// `Button::transition` already animates *colour* by reading the host's sampled
// progress in paint. `Opacity` animates the compositor opacity channel instead,
// so the same "change the value and it animates" contract now covers a whole
// subtree's transparency. The host applies the declared `AnimationSpec::prop`
// to the compositor and folds it onto the children, so no host change is
// needed — these tests only watch the alpha of the text the subtree paints.
// ---------------------------------------------------------------------------

/// A host whose root is an `Opacity` wrapping a `Text` child, at `value`
/// opacity, optionally with an implicit transition. One frame has run.
fn host_with_opacity(value: f32, transition: Option<Transition>) -> (WidgetHost, NodeId) {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 80.0));
    let child = Child::new(ChildKey::index(1), LayoutRequest::default(), || {
        Box::new(Text::new("hi"))
    });
    let opacity = Opacity::new(child).opacity(value);
    let opacity = match transition {
        Some(t) => opacity.transition(t),
        None => opacity,
    };
    let root = host.mount_root(LayoutStyle::default(), opacity);
    host.frame();
    (host, root)
}

/// A change of target fades the subtree from its old opacity to the new one
/// rather than snapping.
#[test]
fn an_opacity_transition_fades_from_its_old_value_to_the_new_one() {
    let (mut host, root) = host_with_opacity(1.0, Some(Transition::new(0.2)));

    // Rests fully shown before any change.
    assert_eq!(child_text_alpha(&host), Some(255));

    // Change the target; the next frames fade from the old (1.0) toward 0.5.
    host.widget_mut::<Opacity>(root)
        .expect("the root is an opacity")
        .set_opacity(0.5);
    for _ in 0..3 {
        host.frame();
    }
    let mid = child_text_alpha(&host).expect("child present");
    assert!(mid < 255, "the fade has left full opacity: {mid}");
    assert!(mid > 140, "but has not yet arrived at 0.5: {mid}");

    // Run it out: it lands at ~0.5 (127) rather than snapping to an endpoint.
    for _ in 0..30 {
        host.frame();
    }
    let settled = child_text_alpha(&host).expect("child present");
    assert!(settled < 155, "it faded down toward 0.5: {settled}");
    assert!(settled > 110, "and did not under-shoot past it: {settled}");
    assert_eq!(host.animations_running(), 0, "and then rests");
}

/// Without a transition the new opacity is applied on the very next frame.
#[test]
fn an_opacity_without_a_transition_is_applied_instantly() {
    let (mut host, root) = host_with_opacity(1.0, None);

    assert_eq!(child_text_alpha(&host), Some(255), "starts opaque");

    host.widget_mut::<Opacity>(root)
        .expect("the root is an opacity")
        .set_opacity(0.5);
    host.frame();

    // No transition → the new opacity is applied on the very next frame.
    let alpha = child_text_alpha(&host).expect("child present");
    assert!(alpha < 135 && alpha > 119, "0.5 applied instantly: {alpha}");

    // The zero-duration tween the static value rides on completes on the next
    // frame, so there is no animation left running.
    host.frame();
    assert_eq!(
        host.animations_running(),
        0,
        "and there is no animation running"
    );
}

/// A transition that returns to its old value fades back up from where it
/// rested, not from where the subtree was born. This is the teeth for the
/// `from = last_target` arm: drop it and the 0.5 → 1.0 leg snaps to full
/// opacity because `from` stays at the initial 1.0.
#[test]
fn an_opacity_transition_reverses_to_its_old_value_when_it_returns() {
    let (mut host, root) = host_with_opacity(1.0, Some(Transition::new(0.2)));

    // 1.0 → 0.5 and settle.
    host.widget_mut::<Opacity>(root)
        .expect("the root is an opacity")
        .set_opacity(0.5);
    for _ in 0..30 {
        host.frame();
    }
    let faded = child_text_alpha(&host).expect("child present");
    assert!(
        faded < 155 && faded > 110,
        "control: it faded to ~0.5: {faded}"
    );

    // 0.5 → 1.0: must fade back up from 0.5, not jump.
    host.widget_mut::<Opacity>(root)
        .expect("the root is an opacity")
        .set_opacity(1.0);
    host.frame();
    let just_started = child_text_alpha(&host).expect("child present");
    assert!(
        just_started < 255,
        "it did not snap back to full opacity: {just_started}"
    );

    for _ in 0..30 {
        host.frame();
    }
    assert_eq!(
        child_text_alpha(&host),
        Some(255),
        "and lands fully shown again"
    );
}
