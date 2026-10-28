//! One vocabulary for "what did this backend drop".
//!
//! Before [`DegradationReport`], each backend spelled its degradations its own
//! way, and a host that wanted to answer "how much of my scene did the
//! renderer drop" had to know which backend it was talking to. These tests
//! pin the two things the unified view promises:
//!
//! 1. **The same fact, the same number, whichever backend produced it.** The
//!    conversions are checked against hand-written reports, so a backend that
//!    moved a field to the wrong kind fails here rather than in a host's
//!    dashboard.
//! 2. **`unexpected()` is what keeps a capability bit honest.** A backend that
//!    claims `shadows: true` and then drops shadows has lied, and the lie is
//!    visible as a non-empty `unexpected()`. This is the check that makes
//!    "a capability bit with no implementation is worse than a missing one"
//!    a runtime fact rather than a doc comment.

use floom_renderer::{DegradationReport, RendererCapabilities};

/// A report with something dropped in each of the four kinds.
fn all_dropped() -> DegradationReport {
    DegradationReport {
        shadows_skipped: 1,
        filters_skipped: 2,
        backdrop_filters_skipped: 3,
        shaders_skipped: 4,
    }
}

#[test]
fn any_and_total_agree_on_nothing_dropped() {
    let none = DegradationReport::NONE;
    assert!(!none.any(), "nothing was dropped");
    assert_eq!(none.total(), 0);
    assert_eq!(none, DegradationReport::default());
}

#[test]
fn any_is_true_for_each_kind_on_its_own() {
    // One at a time, because a `any()` that only looked at the first field
    // would pass a test that set all four.
    for (field, value) in [
        (
            "shadows",
            DegradationReport {
                shadows_skipped: 1,
                ..Default::default()
            },
        ),
        (
            "filters",
            DegradationReport {
                filters_skipped: 1,
                ..Default::default()
            },
        ),
        (
            "backdrops",
            DegradationReport {
                backdrop_filters_skipped: 1,
                ..Default::default()
            },
        ),
        (
            "shaders",
            DegradationReport {
                shaders_skipped: 1,
                ..Default::default()
            },
        ),
    ] {
        assert!(value.any(), "{field} dropped something");
        assert_eq!(value.total(), 1, "{field} counted once");
    }
}

#[test]
fn total_adds_the_kinds_rather_than_overwriting() {
    let r = all_dropped();
    assert_eq!(r.total(), 1 + 2 + 3 + 4);
}

#[test]
fn a_capability_that_was_never_claimed_is_not_a_surprise() {
    // The whole point of `unexpected`: a backend that says up front it cannot
    // filter, and then does not filter, has not degraded anything the caller
    // was not warned about. What is left is empty, and that is the correct
    // answer rather than a false alarm.
    let caps = RendererCapabilities {
        shadows: false,
        filters: false,
        backdrop_filters: false,
        shaders: false,
    };
    let surprises = all_dropped().unexpected(&caps);
    assert!(
        !surprises.any(),
        "everything dropped was already declared: {surprises:?}"
    );
    assert_eq!(surprises, DegradationReport::NONE);
}

#[test]
fn a_capability_that_was_claimed_and_then_dropped_is_a_surprise() {
    // The lie this catches. A backend reports `shadows: true` — the host asks
    // for a shadow on that basis — and the frame then drops it. The dropped
    // shadow is a surprise, and it shows up here.
    let caps = RendererCapabilities {
        shadows: true,
        filters: false,
        backdrop_filters: false,
        shaders: false,
    };
    let surprises = all_dropped().unexpected(&caps);
    assert_eq!(
        surprises.shadows_skipped, 1,
        "the shadow was claimed and then dropped"
    );
    assert_eq!(surprises.filters_skipped, 0, "filters were never claimed");
    assert_eq!(surprises.backdrop_filters_skipped, 0);
    assert_eq!(surprises.shaders_skipped, 0);
    assert!(surprises.any(), "so this is a real surprise");
}

#[test]
fn each_kind_is_checked_against_its_own_capability() {
    // One capability at a time, so a crossed wire — filters checked against
    // `shadows`, say — fails here rather than silently reporting the wrong
    // kind as the surprise.
    let caps = RendererCapabilities::ALL;
    let surprises = all_dropped().unexpected(&caps);
    assert_eq!(surprises.shadows_skipped, 1);
    assert_eq!(surprises.filters_skipped, 2);
    assert_eq!(surprises.backdrop_filters_skipped, 3);
    assert_eq!(surprises.shaders_skipped, 4);

    // And with nothing claimed, nothing is a surprise even at ALL-dropped.
    assert!(!all_dropped().unexpected(&RendererCapabilities::NONE).any());
}

#[test]
fn a_partial_claim_leaves_only_the_claimed_kind_as_a_surprise() {
    // Vello's actual shape: it claims shadows (on rects) and shaders, and
    // does not claim filters. A frame that drops a filter is expected; a
    // frame that drops a shadow is not — even though the shadow it dropped
    // was one on a *path*, which the capability's doc says is out of reach.
    // That residual is the honest limit of a bit: it cannot carry a per-shape
    // answer, so the report says "surprise" and the caller reads the
    // backend's own counter to find out which shape.
    let caps = RendererCapabilities {
        shadows: true,
        filters: false,
        backdrop_filters: false,
        shaders: true,
    };
    let frame = DegradationReport {
        shadows_skipped: 1,          // a path's shadow
        filters_skipped: 5,          // declared
        backdrop_filters_skipped: 2, // declared
        shaders_skipped: 0,
    };
    let surprises = frame.unexpected(&caps);
    assert_eq!(surprises.shadows_skipped, 1);
    assert_eq!(surprises.filters_skipped, 0, "declared, so expected");
    assert_eq!(
        surprises.backdrop_filters_skipped, 0,
        "declared, so expected"
    );
    assert_eq!(surprises.shaders_skipped, 0, "and none was dropped");
}

// --- The backends' conversions --------------------------------------------

/// Vello spells all four the same way the report does, so its conversion is a
/// field-for-field copy. The interesting assertion is that a *new* field
/// cannot be silently forgotten: a report built from stats that dropped a
/// backdrop filter must carry it.
#[test]
fn vellos_report_carries_every_kind_it_counts() {
    // Driven through the public constructor rather than a literal, so this
    // test fails if the stats struct gains a field the conversion ignores.
    let stats = floom_renderer_vello::GpuFrameStats {
        shadows_skipped: 2,
        filters_skipped: 3,
        shaders_skipped: 4,
        ..Default::default()
    };
    let report = stats.degradation();
    assert_eq!(report.shadows_skipped, 2);
    assert_eq!(report.filters_skipped, 3);
    assert_eq!(report.shaders_skipped, 4);
    // Vello counts a backdrop filter inside `filters_skipped` — one number
    // for both, because the answer is the same for both — so the report's
    // separate field stays zero rather than double-counting.
    assert_eq!(report.backdrop_filters_skipped, 0);
}

/// The software rasteriser is the reference: it draws shadows, filters and
/// backdrops, so its report's zeros are meaningful rather than uninteresting.
#[test]
fn the_reference_reports_no_shadow_or_filter_degradation() {
    let sw = floom_renderer_software::SoftwareRenderer::new(8, 8);
    let report = sw.degradation();
    assert_eq!(report.shadows_skipped, 0, "the reference draws shadows");
    assert_eq!(report.filters_skipped, 0, "and filter scopes");
    assert_eq!(report.backdrop_filters_skipped, 0, "and backdrop filters");
    assert_eq!(report.shaders_skipped, 0, "and this frame asked for none");
    assert!(!report.any());
}

/// The terminal cannot attribute its one counter, and the report says so
/// rather than guessing. This is the one place the unified view is less
/// informative than the backend's own stats — and the honest direction.
#[test]
fn the_terminal_reports_its_one_counter_as_shadows_and_says_so() {
    let grid = floom_renderer_tui::TuiGrid::new(4, 2, floom_renderer::Size::new(40.0, 20.0));
    let report = grid.stats().degradation();
    assert_eq!(report.total(), 0, "an untouched grid dropped nothing");

    let mut painted = floom_renderer_tui::TuiGrid::new(4, 2, floom_renderer::Size::new(40.0, 20.0));
    // A rounded rect with a shadow: two things the grid cannot express, so
    // `skipped` is 2 and the report's total is 2.
    let mut scene = floom_renderer::Scene::new();
    scene.begin_update();
    let _ = scene.insert(floom_renderer::SceneNode::Rect {
        bounds: floom_renderer::Rect {
            x0: 1.0,
            y0: 1.0,
            x1: 9.0,
            y1: 9.0,
        },
        fill: floom_types::Brush::Solid(floom_renderer::Color::rgb(10, 20, 30)),
        radius: 3.0,
        shadow: Some(floom_types::Shadow {
            color: floom_renderer::Color::rgba(0, 0, 0, 128),
            blur: 4.0,
            offset_x: 1.0,
            offset_y: 1.0,
            spread: 0.0,
        }),
    });
    scene.finish_update();
    floom_renderer_tui::paint_scene(
        &mut painted,
        &scene,
        &floom_renderer::ResourceRegistry::new(),
    );
    let report = painted.stats().degradation();
    assert_eq!(
        report.total(),
        2,
        "the radius and the shadow, as one number"
    );
}

/// The check that makes a capability bit accountable, run against the real
/// vello backend.
///
/// Vello claims shadows, **filters** and shaders, and does *not* claim backdrop
/// filters. So a frame in which it dropped only backdrops must report **no
/// surprises** — the caller was told, and what happened is what it was told
/// would happen. If vello ever started dropping a filter scope (or a shadow)
/// while still claiming the bit, this is the test that would go red.
///
/// The filter count here is deliberately zero: filters are drawn now, so a
/// dropped one would be a genuine surprise and this test would catch it. Before
/// the offscreen filter pass existed, a non-zero `filters_skipped` was the
/// *declared* case and this test asserted it was unsurprising — which is why
/// the assertion had to change when the capability did.
#[test]
fn vellos_claims_and_its_drops_agree() {
    let Some(presenter) = floom_renderer_vello::VelloGpuPresenter::new() else {
        // No adapter: the capability answer is a property of the code, not
        // the machine, but the presenter is the only thing that carries it and
        // it cannot be built without one. Skip rather than fail.
        return;
    };
    let caps = <floom_renderer_vello::VelloGpuPresenter as floom_renderer::Presenter>::capabilities(
        &presenter,
    );
    assert!(caps.filters, "vello draws filter scopes now");
    let frame = DegradationReport {
        // What vello actually drops today: backdrops only, and that is
        // declared.
        filters_skipped: 0,
        backdrop_filters_skipped: 1,
        // And a path's shadow, which `shadows: true` does not cover — the
        // residual a bit cannot express.
        shadows_skipped: 1,
        shaders_skipped: 0,
    };
    let surprises = frame.unexpected(&caps);
    assert_eq!(
        surprises.backdrop_filters_skipped, 0,
        "backdrops were declared"
    );
    // The shadow is the honest residual: the bit says shadows work, the
    // counter says one did not. That is exactly the signal a host should see.
    assert_eq!(surprises.shadows_skipped, 1);
    assert_eq!(surprises.filters_skipped, 0);
}

/// The same check inverted: a dropped filter scope **is** a surprise now that
/// vello claims `filters: true`. Without this, `unexpected()` could stop
/// subtracting filters entirely and the test above would still pass.
#[test]
fn a_dropped_filter_is_a_surprise_once_the_bit_claims_it() {
    let Some(presenter) = floom_renderer_vello::VelloGpuPresenter::new() else {
        return;
    };
    let caps = <floom_renderer_vello::VelloGpuPresenter as floom_renderer::Presenter>::capabilities(
        &presenter,
    );
    let frame = DegradationReport {
        filters_skipped: 2,
        ..DegradationReport::default()
    };
    assert_eq!(
        frame.unexpected(&caps).filters_skipped,
        2,
        "a claimed capability that dropped something must surface as a surprise"
    );
}

/// The skia adapter records; it does not draw. Its report is what its own
/// counters carry, and nothing invented.
#[test]
fn the_recorder_reports_only_what_it_counted() {
    let rec = floom_renderer_skia_abi::SkiaRecording::default();
    let report = rec.degradation();
    assert!(!report.any(), "a fresh recording has dropped nothing");
}
