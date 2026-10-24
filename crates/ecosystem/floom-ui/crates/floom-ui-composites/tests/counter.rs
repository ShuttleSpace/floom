//! `Counter` — the stepper, and the two stateful-stepper policies it disagrees
//! with `Slider` about.
//!
//! ## Why this file is mostly about two decisions
//!
//! A counter is three parts and a `+`. Almost all of it is trivially true, and the
//! parts that are not trivially true are exactly the parts worth pinning:
//!
//! 1. **`on_change` fires on a press that did not move the value.** That is v1's
//!    behaviour (`let nv = clamp(..); value.set(nv); cb(nv)` — unconditionally), and
//!    it is the *opposite* of what `Slider::set_value` does in this same crate. Both
//!    are defensible; only one of them can be true at a time, so both are tested
//!    here and the disagreement is named rather than smoothed over.
//! 2. **Which bound wins when the range is inverted.** `clamp_value` applies `min`
//!    first and `max` second, so `max` wins. An inverted range is a caller error, but
//!    "which error you get" is still a behaviour, and `f32::clamp` would have given
//!    the other answer — so it is spelled out rather than left to the reader.
//!
//! This file moved with the widget out of `floom-widgets/tests/counter.rs`, for
//! the reason the rest of this crate's tests did: a test of a widget's behaviour
//! belongs with the widget.

//! ## The saturating arithmetic is a divergence, and it is deliberate
//!
//! v1 wrote `value.get() + step` on an `i64`, which panics in a debug build at
//! `i64::MAX`. `the_engine_saturates_rather_than_panicking_at_the_extreme` pins the
//! replacement. There is no kit to disagree with: this is arithmetic.

use std::sync::{Arc, Mutex};

use floom_accessibility::SemanticsRole;
use floom_core::{ElementId, Widget};
use floom_types::{Color, Length};

use floom_ui_composites::{
    clamp_value, step_value, Counter, CounterState, CounterStyle, SharedCounter, Step,
};
use floom_widgets::testkit::RecordingUpdateContext;

const ID: ElementId = ElementId::new(1, 1);

/// A recording callback, so "did it fire" and "with what" are one observation.
type Log = Arc<Mutex<Vec<i64>>>;

fn log() -> Log {
    Arc::new(Mutex::new(Vec::new()))
}

fn read(log: &Log) -> Vec<i64> {
    log.lock().map(|l| l.clone()).unwrap_or_default()
}

fn state(value: i64, min: Option<i64>, max: Option<i64>, step: i64) -> CounterState {
    CounterState {
        value,
        min,
        max,
        step,
    }
}

// --- clamp_value ---------------------------------------------------------

/// With no bounds the value is itself.
#[test]
fn an_unbounded_value_is_its_own_clamp() {
    assert_eq!(clamp_value(7, None, None), 7);
}

/// A lower bound alone lifts a value that is under it and leaves the rest alone.
#[test]
fn a_lower_bound_alone_lifts_only_what_is_under_it() {
    assert_eq!(clamp_value(-3, Some(0), None), 0);
    assert_eq!(clamp_value(4, Some(0), None), 4);
}

/// An upper bound alone drops a value that is over it and leaves the rest alone.
#[test]
fn an_upper_bound_alone_drops_only_what_is_over_it() {
    assert_eq!(clamp_value(11, None, Some(10)), 10);
    assert_eq!(clamp_value(4, None, Some(10)), 4);
}

/// Both bounds, and a value inside them is untouched — the case a ceiling-only or a
/// floor-only test would not reach.
#[test]
fn a_value_inside_both_bounds_is_untouched() {
    assert_eq!(clamp_value(5, Some(0), Some(10)), 5);
}

/// An **inverted** range is resolved in favour of `max`, because `max` is applied
/// last. `f32::clamp` would have given 10 here; this is the other answer, on
/// purpose, and it is v1's.
#[test]
fn an_inverted_range_resolves_in_favour_of_the_upper_bound() {
    assert_eq!(
        clamp_value(5, Some(10), Some(3)),
        3,
        "min is applied first, then max, so max wins the tie"
    );
}

// --- step_value ----------------------------------------------------------

/// A press moves by the step in the direction its name says.
#[test]
fn a_press_moves_by_the_step_in_the_named_direction() {
    let s = state(5, None, None, 3);
    assert_eq!(step_value(&s, Step::Up), 8);
    assert_eq!(step_value(&s, Step::Down), 2);
}

/// The step is applied *before* the clamp, so the last press onto a bound lands
/// exactly on it rather than overshooting or stopping short.
#[test]
fn the_last_step_onto_a_bound_lands_exactly_on_it() {
    let s = state(8, Some(0), Some(10), 5);
    assert_eq!(step_value(&s, Step::Up), 10, "8 + 5 = 13, clamped to 10");
    assert_eq!(step_value(&s, Step::Down), 3);
}

/// A step of zero moves nothing — and the caller still hears about the press,
/// which is the *other* decision in this file rather than this one.
#[test]
fn a_step_of_zero_moves_nothing() {
    let s = state(5, None, None, 0);
    assert_eq!(step_value(&s, Step::Up), 5);
    assert_eq!(step_value(&s, Step::Down), 5);
}

/// A negative step is used as given, so `Up` goes **down**. v1 behaved this way and
/// the test is named for the behaviour, not the intent: fixing it is a decision, and
/// an unnamed test would let it change by accident.
#[test]
fn a_negative_step_makes_up_go_down_which_is_v1s_behaviour() {
    let s = state(5, None, None, -2);
    assert_eq!(step_value(&s, Step::Up), 3);
    assert_eq!(step_value(&s, Step::Down), 7);
}

/// v1's `value + step` would panic here in a debug build. The engine saturates.
#[test]
fn the_engine_saturates_rather_than_panicking_at_the_extreme() {
    let up = state(i64::MAX, None, None, 1);
    assert_eq!(step_value(&up, Step::Up), i64::MAX);
    let down = state(i64::MIN, None, None, 1);
    assert_eq!(step_value(&down, Step::Down), i64::MIN);
}

// --- the widget's own arithmetic -----------------------------------------

/// A fresh counter shows what it was built with and defaults to a step of one.
#[test]
fn a_new_counter_shows_its_initial_value_and_steps_by_one() {
    let counter = Counter::new(5);
    assert_eq!(counter.value(), 5);
    assert_eq!(counter.step(), 1);
    assert_eq!(counter.minimum(), None);
    assert_eq!(counter.maximum(), None);
}

/// Pressing the two buttons moves the value the two ways.
#[test]
fn pressing_the_buttons_moves_the_value() {
    let counter = Counter::new(5);
    assert_eq!(counter.step_up(), 6);
    assert_eq!(counter.step_down(), 5);
}

/// The step is a builder, not a per-press argument, and it is read back.
#[test]
fn the_step_can_be_chosen_and_is_read_back() {
    let counter = Counter::new(0).step_size(7);
    assert_eq!(counter.step(), 7);
    assert_eq!(counter.step_up(), 7);
}

/// A bound set after construction **re-clamps immediately** — a counter built at
/// 50 and then given `max(10)` shows 10 without waiting for a press.
#[test]
fn setting_a_bound_re_clamps_the_value_immediately() {
    let counter = Counter::new(50).max(10);
    assert_eq!(counter.value(), 10, "not 50 waiting for the next press");
    assert_eq!(counter.maximum(), Some(10));
}

/// The lower bound behaves the same way, which is a separate assertion from the
/// upper one: a single one-sided test passes whether the clamp is on one side or
/// the other.
#[test]
fn setting_a_lower_bound_re_clamps_the_value_immediately() {
    let counter = Counter::new(-50).min(0);
    assert_eq!(counter.value(), 0);
    assert_eq!(counter.minimum(), Some(0));
}

/// A bound does not fire `on_change` — it is not a step, and a caller watching the
/// value should not see one for a configuration change.
#[test]
fn setting_a_bound_does_not_fire_on_change() {
    let log = log();
    let sink = log.clone();
    let counter = Counter::new(50).on_change(move |v| sink.lock().unwrap().push(v));
    let counter = counter.max(10);
    assert_eq!(counter.value(), 10, "the value did move");
    assert!(
        read(&log).is_empty(),
        "a re-clamp is not a step: {:?}",
        read(&log)
    );
}

/// Writing a value clamps it, and reports the value that actually landed rather
/// than the one that was asked for.
#[test]
fn writing_a_value_reports_what_landed_not_what_was_asked_for() {
    let log = log();
    let sink = log.clone();
    let counter = Counter::new(0)
        .max(10)
        .on_change(move |v| sink.lock().unwrap().push(v));

    assert_eq!(counter.set_value(50), 10);
    assert_eq!(read(&log), vec![10]);
}

/// **The decision.** A press at a bound fires `on_change` with the unchanged value.
/// v1 called back unconditionally, and callers were written against that.
#[test]
fn a_press_that_cannot_move_the_value_still_fires_on_change() {
    let log = log();
    let sink = log.clone();
    let counter = Counter::new(10)
        .max(10)
        .on_change(move |v| sink.lock().unwrap().push(v));

    assert_eq!(counter.step_up(), 10, "the value did not move");
    assert_eq!(
        read(&log),
        vec![10],
        "v1 wrote `value.set(nv); cb(nv)` with no guard, so the callback fires \
         even when nv == value. Slider::set_value does the opposite."
    );
}

/// **The control group for the test above.** The same press on a counter *with*
/// room to move reports a value that did change — so the test above is not passing
/// because the callback fires on everything.
#[test]
fn a_press_that_can_move_the_value_fires_with_the_new_value() {
    let log = log();
    let sink = log.clone();
    let counter = Counter::new(9)
        .max(10)
        .on_change(move |v| sink.lock().unwrap().push(v));

    assert_eq!(counter.step_up(), 10);
    assert_eq!(read(&log), vec![10]);
}

/// Both ends report, so a caller can tell "hit the ceiling" from "hit the floor"
/// without reading the widget.
#[test]
fn both_ends_of_the_range_are_reported() {
    let log = log();
    let sink = log.clone();
    let counter = Counter::new(0)
        .min(0)
        .max(1)
        .on_change(move |v| sink.lock().unwrap().push(v));

    counter.step_down();
    counter.step_up();
    counter.step_up();
    assert_eq!(
        read(&log),
        vec![0, 1, 1],
        "the last press repeats the ceiling"
    );
}

/// A counter with no callback does not need one to work.
#[test]
fn a_counter_without_a_callback_still_steps() {
    let counter = Counter::new(1);
    assert_eq!(counter.step_up(), 2);
    assert_eq!(counter.step_down(), 1);
}

// --- the shared handle ---------------------------------------------------

/// The handle is shared, so a caller that clones it sees the same value the
/// widget's own buttons write — which is the only reason it is `Arc<Mutex<..>>`.
#[test]
fn the_shared_handle_and_the_widget_are_one_value() {
    let counter = Counter::new(3);
    let handle = counter.state();

    counter.step_up();
    assert_eq!(handle.lock().unwrap().value, 4);

    // ...and a write through the handle is what the widget reports next.
    handle.lock().unwrap().value = 99;
    assert_eq!(counter.value(), 99);
}

/// `with_shared_state` adopts a handle rather than allocating a second one — the
/// reason a caller can hold a value it built itself.
#[test]
fn with_shared_state_adopts_the_handles_value() {
    let handle: SharedCounter = Arc::new(Mutex::new(state(7, None, None, 1)));
    let counter = Counter::with_shared_state(handle.clone());
    assert_eq!(counter.value(), 7);
    counter.step_up();
    assert_eq!(handle.lock().unwrap().value, 8);
}

// --- mounting ------------------------------------------------------------

/// The widget builds: a band, and inside it the three parts.
#[test]
fn the_counter_builds_a_band_holding_three_parts() {
    let mut counter = Counter::new(5);
    let mut cx = RecordingUpdateContext::default();

    counter.update(&mut cx, ID);
    let children = counter.children();
    assert_eq!(children.len(), 1, "one band");

    // Two levels down, not one: the band is a `Surface`, and the three parts live
    // in the `Stack` it holds. Asserting on the band's own children would find
    // exactly one child and look like a broken counter.
    let band = children[0].build();
    let row_holder = band.children();
    assert_eq!(row_holder.len(), 1, "the band holds one row");
    let row = row_holder[0].build();
    let parts = row.children();
    assert_eq!(
        parts.len(),
        3,
        "a step button, the readout, a step button — the structure v1 wrote as \
         `Stack::horizontal((dec, val, inc))`"
    );
}

/// The band is the size the style says — a layout test that asserted the band and
/// the buttons together would still pass with one of them hard-coded to a match.
#[test]
fn the_band_takes_its_size_from_the_style() {
    let style = CounterStyle {
        width: 123.0,
        height: 45.0,
        ..CounterStyle::default()
    };
    let counter = Counter::new(0).style(style);
    let band = counter.children()[0].build();

    let request = band.layout_request().expect("the band declares a size");
    assert_eq!(request.width.unwrap(), Length::Px(123.0));
    assert_eq!(request.height.unwrap(), Length::Px(45.0));
}

/// The style is what the caller passed, and it is readable afterwards — the
/// counterpart to the rule that a value handed to a widget must actually be read.
#[test]
fn the_style_the_caller_passes_is_the_style_that_comes_back() {
    let style = CounterStyle {
        button_size: 30.0,
        gap: 9.0,
        border: 2.0,
        ..CounterStyle::default()
    };
    let counter = Counter::new(0).style(style);
    let resolved = counter.resolved_style();
    assert_eq!(resolved.button_size, 30.0);
    assert_eq!(resolved.gap, 9.0);
    assert_eq!(resolved.border, 2.0);
}

/// The engine default must not be Material's palette. The three kits read
/// `surface_variant` / `on_surface` / `outline`, so an unstyled counter is neutral
/// grey — if this ever drifts towards a kit's colours, an unstyled counter would
/// silently become that kit.
#[test]
fn the_engine_default_is_neutral_rather_than_materials_palette() {
    let style = CounterStyle::default();
    assert_eq!(
        (
            style.button_background,
            style.button_foreground,
            style.border_color
        ),
        (
            Color::rgb(230, 230, 230),
            Color::rgb(20, 20, 20),
            Color::rgb(120, 120, 120)
        ),
        "grey on grey; a kit's surface_variant / on_surface / outline would read as \
         'the default is Material's' to anyone who skipped the kit"
    );
}

/// The counter announces itself as a group, so the two buttons and the readout are
/// read as one control rather than as three unrelated nodes.
#[test]
fn the_counter_announces_itself_as_a_group() {
    let counter = Counter::new(0);
    assert_eq!(counter.semantics_hint().role, SemanticsRole::Group);
}
