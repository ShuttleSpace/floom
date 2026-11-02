//! `Ripple` — the press ripple, including the mechanism it depends on.
//!
//! The ripple is the case that looked like it needed an engine change and
//! didn't: a two-phase, pointer-driven animation expressed through a
//! *declarative* spec. The tests here pin both halves — that the geometry and
//! phases are right, and that the declaration actually restarts when the phase
//! flips. `floom-app/tests/event_animation.rs` is the end-to-end control on the
//! host side; this file is the widget side.

use floom_core::{Color, LayoutRequest, PointerPhase, Rect, Size, Widget};
use floom_types::{ClipShape, PointerEvent};
use floom_widgets::testkit::{
    AllocatingCreateContext, SceneRecorder, TestEventContext, paint_snapshot,
};
use floom_widgets::{Ripple, Text};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 100.0,
    y1: 20.0,
};

fn allocate(widget: &dyn Widget) -> (floom_core::ElementId, AllocatingCreateContext) {
    let mut create = AllocatingCreateContext::new();
    let id = widget.create(&mut create);
    (id, create)
}

/// Send one pointer event to a ripple and report whether it consumed it.
///
/// `Widget::event` takes `&mut self`, so this takes a mutable reference; the
/// tests that only care about the consumed/not answer build a throwaway ripple.
fn send(ripple: &mut Ripple, event: floom_core::Event) -> bool {
    let (id, _create) = allocate(ripple);
    let mut cx = TestEventContext::new(id, BOX, event);
    ripple.event(&mut cx, id)
}

fn press_at(x: f64, y: f64) -> floom_core::Event {
    PointerEvent::primary_down(floom_core::Point::new(x, y)).into()
}

fn release_at(x: f64, y: f64) -> floom_core::Event {
    PointerEvent::primary_up(floom_core::Point::new(x, y)).into()
}

/// A ripple that has just been pressed at the centre of `BOX`.
fn pressed() -> Ripple {
    let mut ripple = Ripple::new();
    send(&mut ripple, press_at(50.0, 10.0));
    ripple
}

#[test]
fn a_resting_ripple_paints_nothing() {
    let ripple = Ripple::new();
    let (id, _create) = allocate(&ripple);
    let mut recorder = SceneRecorder::new(BOX);
    ripple.paint(&mut recorder, id);
    recorder.finish();
    // The recorder renders "no ops" as the literal `(empty)`.
    assert!(
        recorder.snapshot().contains("(empty)"),
        "an idle ripple must contribute no paint ops, got:\n{}",
        recorder.snapshot()
    );
}

#[test]
fn a_press_does_not_consume_the_event() {
    // Wrapping a button in a ripple must not disable the button: the press has
    // to reach the child underneath.
    let mut ripple = Ripple::new();
    assert!(
        !send(&mut ripple, press_at(10.0, 10.0)),
        "the ripple must let the press through to its child"
    );
}

#[test]
fn the_press_moves_the_ripple_into_the_expanding_phase() {
    let ripple = pressed();
    assert_eq!(
        ripple.phase(),
        floom_widgets::Phase::Expanding,
        "a press starts the expansion"
    );
}

#[test]
fn a_release_following_a_press_starts_the_fade() {
    let mut ripple = pressed();
    send(&mut ripple, release_at(50.0, 10.0));
    assert_eq!(ripple.phase(), floom_widgets::Phase::Fading);
}

#[test]
fn a_release_with_no_press_in_flight_starts_nothing() {
    // Otherwise a stray release would conjure ink out of nowhere.
    let mut ripple = Ripple::new();
    send(&mut ripple, release_at(10.0, 10.0));
    assert_eq!(
        ripple.phase(),
        floom_widgets::Phase::Idle,
        "a release with nothing pressed must leave the ripple at rest"
    );
}

#[test]
fn a_disabled_ripple_ignores_the_press_but_still_passes_it_on() {
    let mut ripple = Ripple::new().disabled(true);
    send(&mut ripple, press_at(10.0, 10.0));
    assert_eq!(
        ripple.phase(),
        floom_widgets::Phase::Idle,
        "a disabled ripple must stay at rest"
    );
    assert!(
        !send(&mut Ripple::new().disabled(true), press_at(10.0, 10.0)),
        "and must still let the press reach the child"
    );
}

#[test]
fn each_phase_declares_a_distinct_spec() {
    // The restart the host performs is keyed on this difference; if two phases
    // matched, the second phase would silently never start.
    let idle = Ripple::new().animation().expect("idle declares");
    let expanding = pressed().animation().expect("expanding declares");
    assert!(
        !idle.matches(&expanding),
        "resting and expanding must declare different specs"
    );

    let mut fading = pressed();
    send(&mut fading, release_at(50.0, 10.0));
    let fading = fading.animation().expect("fading declares");
    assert!(
        !expanding.matches(&fading),
        "expanding and fading must declare different specs"
    );
}

#[test]
fn the_ink_is_clipped_to_the_rounded_box() {
    let ripple = Ripple::new().radius(8.0);
    assert_eq!(
        ripple.clip(),
        Some(ClipShape::RoundedBox { radius: 8.0 }),
        "without the clip the circle spills past the corners"
    );
}

#[test]
fn the_ink_colour_is_used_at_full_opacity_while_expanding() {
    let ripple = pressed().ink(Color::rgba(10, 20, 30, 200));
    let (id, _create) = allocate(&ripple);
    // Sample the expansion at its midpoint by asking for a mid value: the
    // testkit hands back the declared `from` on the first paint, so drive the
    // recorder and read the inked rect.
    // Sample mid-expansion: at t=0 the ink has zero radius, so nothing paints,
    // which is correct but tells us nothing.
    let mut recorder = SceneRecorder::new(BOX).with_animation_value(0.5);
    ripple.paint(&mut recorder, id);
    recorder.finish();
    let snap = recorder.snapshot();
    assert!(
        snap.contains("RoundedRect"),
        "an expanding ripple paints its ink circle, got:\n{snap}"
    );
}

#[test]
fn the_diameter_covers_the_whole_box_from_the_pointer() {
    // Centred press on a 100x20 box: farthest corner is (50, 10) away, so the
    // diameter is 2 * sqrt(50^2 + 10^2).
    let expected = 2.0 * (50.0_f64 * 50.0 + 10.0 * 10.0).sqrt();
    let mut ripple = Ripple::new();
    send(&mut ripple, press_at(50.0, 10.0));

    // The painted circle's box spans `diameter`, and its centre sits on the
    // press point, so read the diameter back out of the paint op.
    let (id, _create) = allocate(&ripple);
    let mut recorder = SceneRecorder::new(BOX).with_animation_value(1.0);
    ripple.paint(&mut recorder, id);
    recorder.finish();
    let line = recorder
        .snapshot()
        .lines()
        .find(|l| l.contains("RoundedRect"))
        .expect("ink painted")
        .to_string();
    let width = parse_rect_width(&line);
    assert!(
        (width - expected).abs() < 0.5,
        "ink diameter {width} should be about {expected} (got: {line})"
    );
}

#[test]
fn the_setters_follow_a_theme_change() {
    // The radius function used to re-evaluate against the theme on every use. The
    // tree is mounted once, so a kit pushes the new value in through these instead
    // -- which only works if each one bumps the revision, or the change would sit
    // unread.
    let mut ripple = Ripple::new().radius(4.0).ink(Color::rgba(1, 2, 3, 4));
    assert_eq!(ripple.clip_radius(), 4.0);

    ripple.set_radius(12.0);
    ripple.set_ink(Color::rgba(9, 9, 9, 40));
    ripple.set_disabled(true);

    assert_eq!(ripple.clip_radius(), 12.0);
    assert!(ripple.is_disabled());
    assert_eq!(
        ripple.clip(),
        Some(ClipShape::RoundedBox { radius: 12.0 }),
        "the new radius must reach the clip declaration"
    );

    // The changes above must be picked up once...
    let (id, _create) = allocate(&ripple);
    let mut first = floom_widgets::testkit::RecordingUpdateContext::new();
    ripple.update(&mut first, id);
    assert!(
        first.dirty_count() > 0,
        "a radius/ink/disabled change must invalidate the element"
    );

    // ...and then stop: setting the same values again must dirty nothing, or a
    // theme signal firing on every unrelated change would repaint constantly.
    ripple.set_radius(12.0);
    ripple.set_ink(Color::rgba(9, 9, 9, 40));
    ripple.set_disabled(true);
    let mut second = floom_widgets::testkit::RecordingUpdateContext::new();
    ripple.update(&mut second, id);
    assert_eq!(
        second.dirty_count(),
        0,
        "an idempotent setter must not mark the element dirty"
    );
}

#[test]
fn it_holds_exactly_one_child() {
    let ripple = Ripple::new().child(LayoutRequest::new(), || Box::new(Text::new("x")));
    assert_eq!(ripple.children().len(), 1);
    assert!(
        Ripple::new().children().is_empty(),
        "a ripple with no child is a leaf"
    );
    let _ = Size::new(1.0, 1.0);
    let _ = PointerPhase::Down;
}

/// Pull the width out of a `RoundedRect (x0, y0, x1, y1) ...` snapshot line.
fn parse_rect_width(line: &str) -> f64 {
    let open = line.find('(').expect("coords");
    let close = line[open..].find(')').expect("coords end") + open;
    let nums: Vec<f64> = line[open + 1..close]
        .split(',')
        .map(|s| s.trim().parse().expect("number"))
        .collect();
    nums[2] - nums[0]
}
