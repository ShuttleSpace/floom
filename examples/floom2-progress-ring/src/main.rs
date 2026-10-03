//! Self-drawn motion, from the prelude alone.
//!
//! Three capabilities, each asserted:
//!
//! 1. **The host can drive a drawn shape.** A progress ring is a `Canvas`
//!    whose painter reads `animation_value` and turns it into arc geometry;
//!    `WidgetHost::animate` runs the tween, and the *drawn* arc moves with it
//!    — visible as the arc's scene node changing shape between frames.
//! 2. **A control can animate itself, forever.** The `Spinner` declares an
//!    endless sweep and the host runs it: the app never starts anything, the
//!    frame loop never parks while it is mounted, and the value moves.
//! 3. **A signal can repaint a canvas, paint-only.** Bumping the ping from
//!    app code wakes the host (`needs_frame`) without a relayout, and the
//!    next frame redraws.
//!
//! Everything below is `floom2::prelude`. Run with
//! `cargo run -p floom2-progress-ring`.

use floom2::advanced::PaintContext;
use floom2::floom_scene::SceneNode;
use floom2::prelude::*;

const RING_SIZE: f32 = 96.0;

fn main() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(240.0, 240.0));

    let ping = RwSignal::new(());
    let root = host.mount_root(LayoutStyle::default(), Stack::new(Axis::Vertical));
    let ring = host.mount(
        root,
        LayoutStyle {
            width: Some(RING_SIZE),
            height: Some(RING_SIZE),
            ..LayoutStyle::default()
        },
        Canvas::new(paint_ring)
            .size(RING_SIZE, RING_SIZE)
            .repaint_on(ping)
            .label("Progress ring"),
    );
    host.frame();

    // The ring runs alone first: its "the loop parks" claims need an app with
    // no other animation mounted. The spinner arrives in act 2.
    act_1_the_host_drives_a_drawn_shape(&mut host, ring);
    act_3_a_ping_repaints_paint_only(&mut host, ring, ping);
    act_2_a_control_animates_itself(&mut host, root);

    println!();
    println!("every act above was asserted, not printed");
}

/// The ring's painter: a faint track plus an arc that sweeps `progress` of a
/// full turn. The animation value comes from the host's animation channel for
/// this element — the painter is where motion becomes geometry.
fn paint_ring(cx: &mut dyn PaintContext) {
    let b = cx.bounds();
    let center = Point::new(b.x0 + b.width() / 2.0, b.y0 + b.height() / 2.0);
    let radius = (b.width().min(b.height()) / 2.0) - 6.0;

    cx.stroke_path(
        &Path::circle(center, radius),
        &Stroke::new(6.0, Color::rgb(230, 230, 230)),
    );

    let progress = cx.animation_value().unwrap_or(0.0).clamp(0.0, 1.0);
    let sweep = std::f64::consts::TAU * progress;
    let arc = Path::ellipse_arc(center, radius, radius, -std::f64::consts::FRAC_PI_2, sweep);
    cx.stroke_path(&arc, &Stroke::new(6.0, Color::rgb(30, 90, 200)));
}

/// One tween, read by a painter: the drawn arc's scene node changes shape as
/// the value advances.
fn act_1_the_host_drives_a_drawn_shape(host: &mut WidgetHost, ring: NodeId) {
    println!("== 1. the host drives a drawn shape ==");
    host.animate(
        ring,
        AnimKind::Tween(Tween::new(0.0, 1.0, 0.5).with_easing(Easing::Linear)),
    );

    // Five frames in (~0.083s of a 0.5s tween): a small arc.
    for _ in 0..5 {
        host.frame();
    }
    let early = host.animation_value(ring).expect("the tween is running");
    let early_bounds = ring_arc_bounds(host);
    assert!(
        early > 0.05 && early < 0.35,
        "the tween is partway: {early}"
    );

    // Thirty more frames (~0.58s total): finished, full circle.
    for _ in 0..30 {
        host.frame();
    }
    let late = host.animation_value(ring).expect("parked at the end");
    let late_bounds = ring_arc_bounds(host);
    println!("  animation value: {early:.3} -> {late:.3}");
    println!("  arc box: {} -> {}", fmt(early_bounds), fmt(late_bounds));

    assert!((late - 1.0).abs() < 1e-9, "the tween reached its end");
    assert_ne!(
        early_bounds, late_bounds,
        "the arc the painter drew is not the same arc — the value reached the canvas"
    );

    // The value was parked, not stopped: nothing asks for frames now.
    assert!(!host.needs_frame(), "a finished tween lets the loop park");
    println!("  a finished tween parks the frame loop");
}

/// The bounds of the ring's arc node — the second path the ring paints
/// (first is the track). This is the drawn shape, not the value.
fn ring_arc_bounds(host: &WidgetHost) -> Rect {
    let mut arc = None;
    for (_, node) in host.scene().iter() {
        if let SceneNode::Path { path, stroke, .. } = node {
            // The arc is the blue stroke; the track is grey. Distinguish by
            // stroke colour: the painter drew the track with (230,230,230).
            if let Some(s) = stroke {
                if s.color != Color::rgb(230, 230, 230) {
                    arc = Some(path.bounds());
                }
            }
        }
    }
    arc.expect("the ring's arc is in the scene")
}

fn fmt(r: Rect) -> String {
    format!("[{:.0},{:.0} .. {:.0},{:.0}]", r.x0, r.y0, r.x1, r.y1)
}

/// The spinner nobody started and nobody can stop (except by unmounting it).
fn act_2_a_control_animates_itself(host: &mut WidgetHost, root: NodeId) {
    println!("== 2. a control that animates itself ==");
    let spinner = host.mount(root, LayoutStyle::default(), Spinner::new().size(24.0));
    host.frame();
    let running = host.animations_running();
    let start = host.animation_value(spinner).expect("the sweep is running");
    println!("  running animations nobody asked for: {running}");

    for _ in 0..10 {
        host.frame();
    }
    let moved = host.animation_value(spinner).expect("still running");
    println!("  sweep: {start:.3} -> {moved:.3} after 10 frames");
    assert_ne!(start, moved, "the spinner's arc travels");
    assert!(
        host.needs_frame(),
        "an endless cycle keeps asking for frames while it is mounted"
    );
    assert!(
        host.animations_running() >= 1,
        "and the host is the one running it"
    );
}

/// Bumping the ping wakes the host and repaints the canvas — without a
/// relayout, because the request is paint-only by construction.
fn act_3_a_ping_repaints_paint_only(host: &mut WidgetHost, ring: NodeId, ping: RwSignal<()>) {
    println!("== 3. a signal repaints the canvas ==");
    // The tween from act 1 finished; the loop is parked.
    assert!(!host.needs_frame(), "control: parked before the ping");

    ping.set(());
    assert!(
        host.needs_frame(),
        "the ping scheduled work for the next frame"
    );
    let stats = host.frame();
    println!("  painted {} widgets on the wake frame", stats.painted);
    assert!(stats.painted > 0, "the wake frame really painted");
    assert!(
        !host.needs_frame(),
        "and the queue is drained: one ping, one repaint"
    );

    // The canvas is the thing that came back: its semantics still stand and a
    // second ping behaves the same way.
    ping.set(());
    assert!(host.needs_frame(), "a second ping wakes it again");
    host.frame();
    assert!(!host.needs_frame());
    assert!(
        host.semantics_hint(ring).is_some(),
        "the ring is still mounted and describing itself"
    );
}
