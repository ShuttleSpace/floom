// SPDX-License-Identifier: MIT
//! The profiler's contract.
//!
//! Wall-clock timing is not testable as written, so the profiler takes an
//! injectable clock; every duration here is exact because the clock is
//! scripted. The interesting assertions are about *attribution* (which stage
//! got which span), *the budget verdict*, and *the counters' arithmetic* —
//! not about nanoseconds observed.

use floom_devtools::profiler::{CacheTracker, Profiler, Stage};
use floom_inspector::JsonSnapshot;

/// A clock that replays a script of nanosecond readings.
#[derive(Clone, Default)]
struct ScriptedClock(std::rc::Rc<std::cell::RefCell<Vec<u64>>>);

impl ScriptedClock {
    fn new(readings: &[u64]) -> Self {
        Self(std::rc::Rc::new(std::cell::RefCell::new(readings.to_vec())))
    }

    fn as_clock(&self) -> impl Fn() -> u64 + 'static {
        let script = self.0.clone();
        move || {
            let mut script = script.borrow_mut();
            if script.len() > 1 {
                script.remove(0)
            } else {
                script[0]
            }
        }
    }
}

#[test]
fn stages_are_attributed_with_exact_durations() {
    // read 0: frame start; read 1: layout span start; read 2: layout span end
    // (= paint start); read 3: paint end; read 4: frame end.
    let clock = ScriptedClock::new(&[0, 10, 1_500, 2_400, 3_000]);
    let profiler = Profiler::with_clock(8_000, clock.as_clock());
    profiler.begin_frame();
    {
        let _layout = profiler.stage(Stage::Layout);
        let _paint = profiler.stage(Stage::Paint);
    }
    let frame = profiler.end_frame().expect("a frame");
    assert_eq!(frame.stages.len(), 2);
    assert_eq!(frame.stages[0].stage, "paint");
    assert_eq!(frame.stages[0].duration_ns, 900, "2_400 - 1_500");
    assert_eq!(frame.stages[1].stage, "layout");
    // The layout span closed when its guard dropped — after paint's — so it
    // includes paint. A frame's wall clock is not the sum of nested spans.
    assert_eq!(
        frame.stages[1].duration_ns, 2_990,
        "3_000 - 10; includes paint"
    );
    assert!(!frame.budget_exceeded, "3_000 < 8_000");
}

#[test]
fn a_nested_span_is_included_in_its_parent() {
    // layout { render { } }: the layout span closes after the render span and
    // includes it. The spans are recorded inner-first (closed first), and the
    // frame total is the outer span's wall clock, not the sum.
    let clock = ScriptedClock::new(&[0, 10, 100, 200, 300, 400]);
    let profiler = Profiler::with_clock(10_000, clock.as_clock());
    profiler.begin_frame();
    {
        let _layout = profiler.stage(Stage::Layout);
        {
            let _render = profiler.stage(Stage::Render);
        }
    }
    let frame = profiler.end_frame().expect("a frame");
    assert_eq!(frame.stages.len(), 2);
    assert_eq!(frame.stages[0].stage, "render");
    assert_eq!(frame.stages[0].duration_ns, 100, "200 - 100");
    assert_eq!(frame.stages[1].stage, "layout");
    assert_eq!(
        frame.stages[1].duration_ns, 290,
        "300 - 10; includes render"
    );
}

#[test]
fn the_budget_verdict_flags_an_over_budget_frame() {
    let clock = ScriptedClock::new(&[0, 500, 16_700]);
    let profiler = Profiler::with_clock(16_666, clock.as_clock());
    profiler.begin_frame();
    drop(profiler.stage(Stage::Paint));
    let frame = profiler.end_frame().expect("a frame");
    assert!(frame.budget_exceeded, "16_700 > 16_666");
    assert_eq!(frame.stages[0].duration_ns, 16_200);

    // And a frame inside the budget is not flagged.
    let clock = ScriptedClock::new(&[0, 500, 16_600]);
    let profiler = Profiler::with_clock(16_666, clock.as_clock());
    profiler.begin_frame();
    drop(profiler.stage(Stage::Paint));
    let frame = profiler.end_frame().expect("a frame");
    assert!(!frame.budget_exceeded);
}

#[test]
fn the_timeline_accumulates_frames_in_order() {
    let clock = ScriptedClock::new(&[0, 0, 100, 200, 300]);
    let profiler = Profiler::with_clock(16_666, clock.as_clock());
    profiler.begin_frame();
    profiler.end_frame();
    profiler.begin_frame();
    profiler.end_frame();
    let timeline = profiler.timeline();
    assert_eq!(timeline.len(), 2);
    assert_eq!(timeline[0].index, 0);
    assert_eq!(timeline[1].index, 1);
}

#[test]
fn the_timeline_serializes_to_the_protocol_snapshot() {
    // Determinism: the same frames must serialize to the same bytes, twice.
    let clock = ScriptedClock::new(&[0, 10, 1_000, 200, 300]);
    let profiler = Profiler::with_clock(16_666, clock.as_clock());
    profiler.begin_frame();
    drop(profiler.stage(Stage::Input));
    drop(profiler.stage(Stage::Layout));
    profiler.end_frame();

    let json_a = profiler.to_json().expect("serializes");
    let json_b = profiler.to_json().expect("serializes");
    assert_eq!(json_a, json_b);

    // And it parses as the protocol's own performance snapshot.
    let snapshot = floom_inspector::PerformanceSnapshot::from_json(&json_a).expect("parses");
    assert_eq!(snapshot.frames.len(), 1);
    assert_eq!(snapshot.frames[0].stages.len(), 2);
    // Closed spans are appended as they close: input closed before layout.
    assert_eq!(snapshot.frames[0].stages[0].stage, "input");
    assert_eq!(snapshot.frames[0].stages[1].stage, "layout");
}

#[test]
fn allocation_counters_do_their_own_arithmetic() {
    let clock = ScriptedClock::new(&[0]);
    let profiler = Profiler::with_clock(16_666, clock.as_clock());
    profiler.record_alloc(128);
    profiler.record_alloc(64);
    profiler.record_dealloc(64);
    let summary = profiler.allocations();
    assert_eq!(summary.allocations, 2);
    assert_eq!(summary.deallocations, 1);
    assert_eq!(summary.bytes_allocated, 192);
    assert_eq!(summary.live(), 1);
}

#[test]
fn cache_trackers_report_hit_rate_and_only_that() {
    let cache = CacheTracker::new("glyph");
    assert_eq!(cache.stats().hit_rate(), None, "an idle cache is not 100%");

    cache.hit();
    cache.hit();
    cache.miss();
    cache.miss();
    cache.miss();
    let stats = cache.stats();
    assert_eq!(stats.name, "glyph");
    assert_eq!(stats.hits, 2);
    assert_eq!(stats.misses, 3);
    let rate = stats.hit_rate().expect("2 of 5");
    assert!((rate - 0.4).abs() < 1e-9, "{rate}");

    // Entries and bytes are the host's to maintain; the tracker just carries.
    cache.set_entries(42, 2048);
    assert_eq!(cache.stats().entries, 42);
    assert_eq!(cache.stats().bytes, 2048);
}

#[test]
fn every_stage_has_the_protocol_spelling() {
    assert_eq!(Stage::Input.name(), "input");
    assert_eq!(Stage::Reactive.name(), "reactive");
    assert_eq!(Stage::Layout.name(), "layout");
    assert_eq!(Stage::Paint.name(), "paint");
    assert_eq!(Stage::Scene.name(), "scene");
    assert_eq!(Stage::Render.name(), "render");
    assert_eq!(Stage::Native.name(), "native");
}
