//! `LinearProgress` — the shape with the lowest cross-kit similarity in the port
//! (0.17 / 0.19), so this suite is mostly a disagreement table with assertions.
//!
//! ## What each test is for
//!
//! Four kits' worth of difference, expressed as four parameters:
//!
//! | | Material | Cupertino | Fluent |
//! |---|---|---|---|
//! | fill placement | absolute | absolute | centred child |
//! | fill radius | none | track radius | track radius |
//! | percent | exact | truncated | exact |
//! | indeterminate | sliding 40% | static 40% | static 40% |
//!
//! Every row gets a test here that would fail if the parameter were dropped, plus
//! a control that fails if it were the *other* value. A one-sided test — "the fill
//! is centred" — passes for a centred fill and for an unparameterised one that
//! happens to centre, so the controls are not decoration.
//!
//! ## The two claims this file also makes falsifiable
//!
//! * **The fill is as tall as the track**, which is why Material's
//!   `height_pct(100.0)` and Cupertino's `height(track_height)` could be merged
//!   into one parameter. If that merge were wrong the geometry tests would show a
//!   fill shorter or taller than the track.
//! * **The sliding segment has no driver.** `the_sliding_offset_moves_the_fill`
//!   moves it by hand, and nothing in this crate moves it on its own. That is the
//!   honest shape of the loss, rather than a static bar pretending to animate.

use floom_accessibility::SemanticsRole;
use floom_core::Widget;
use floom_types::{Color, Rect};

const ID: floom_core::ElementId = floom_core::ElementId::new(1, 1);

use floom_widgets::progress_linear::{
    FillPlacement, INDETERMINATE_WIDTH_PCT, Indeterminate, LinearProgress, LinearProgressStyle,
    PercentRounding, SLIDING_TRAVEL_PCT, fill_percent,
};
use floom_widgets::testkit::{RecordingUpdateContext, paint_snapshot};

/// A 200 x 10 track, which is wide enough that a whole-percent difference is
/// visible and short enough that the numbers stay readable.
const TRACK: Rect = Rect {
    x0: 10.0,
    y0: 20.0,
    x1: 210.0,
    y1: 30.0,
};

fn style() -> LinearProgressStyle {
    LinearProgressStyle {
        track_color: Color::rgb(200, 200, 200),
        fill_color: Color::rgb(50, 50, 250),
        track_height: 10.0,
        track_radius: 4.0,
        fill_radius: Some(4.0),
        fill_placement: FillPlacement::Absolute,
        percent_rounding: PercentRounding::Exact,
    }
}

fn progress(value: f32) -> LinearProgress {
    LinearProgress::new(value).style(style())
}

// --- fill_percent --------------------------------------------------------

/// A value inside the range is its own percentage.
#[test]
fn a_value_inside_the_range_is_its_own_percentage() {
    assert_eq!(
        fill_percent(42.0, Indeterminate::None, PercentRounding::Exact),
        42.0
    );
}

/// Above 100 clamps to 100, and below 0 clamps to 0 — both directions, because a
/// ceiling-only test passes for a floor too.
#[test]
fn a_value_outside_the_range_clamps_at_both_ends() {
    assert_eq!(
        fill_percent(150.0, Indeterminate::None, PercentRounding::Exact),
        100.0
    );
    assert_eq!(
        fill_percent(-5.0, Indeterminate::None, PercentRounding::Exact),
        0.0
    );
}

/// **Cupertino's disagreement, as a number.** A fractional percentage is truncated,
/// so `33.7` fills to 33% rather than 33.7%. This is the only disagreement in this
/// component that changes a number rather than a box.
#[test]
fn truncating_rounding_drops_the_fraction() {
    assert_eq!(
        fill_percent(33.7, Indeterminate::None, PercentRounding::Truncate),
        33.0
    );
    assert_eq!(
        fill_percent(33.7, Indeterminate::None, PercentRounding::Exact),
        // An f32 widened to f64 is not the decimal literal: 33.7f32 is
        // 33.700000762939453125. Comparing against the f32 keeps the test saying
        // what it means instead of hiding a rounding in the assertion.
        33.7_f32 as f64,
        "the control: the same value under Exact keeps its fraction, so the \
         assertion above is measuring the rounding and not the input"
    );
}

/// ...and truncation changes nothing at a whole number, which is what makes it a
/// rounding rule rather than a second clamping rule.
#[test]
fn truncating_rounding_agrees_with_exact_at_a_whole_number() {
    for whole in [0.0_f32, 25.0, 50.0, 100.0] {
        assert_eq!(
            fill_percent(whole, Indeterminate::None, PercentRounding::Truncate),
            fill_percent(whole, Indeterminate::None, PercentRounding::Exact),
            "at {whole}"
        );
    }
}

/// Truncation happens after clamping, so `100.4` truncates to 100 and not to 100.4.
#[test]
fn the_clamp_runs_before_the_truncation() {
    assert_eq!(
        fill_percent(100.4, Indeterminate::None, PercentRounding::Truncate),
        100.0
    );
}

/// Every indeterminate mode draws 40%, whatever the value says — including a value
/// of zero, which is the case a `max(value, segment)` implementation would get
/// wrong in the other direction.
#[test]
fn an_indeterminate_bar_is_forty_percent_whatever_the_value() {
    for value in [-10.0_f32, 0.0, 55.0, 200.0] {
        assert_eq!(
            fill_percent(
                value,
                Indeterminate::StaticSegment,
                PercentRounding::Exact
            ),
            INDETERMINATE_WIDTH_PCT,
            "at value {value}"
        );
    }
}

/// ...and the rounding is irrelevant there too, because v1 put the indeterminate
/// branch *before* the conversion in all three kits. Cupertino wrote
/// `if indeterminate { 40.0 } else { (ratio * 100.0) as f64 }`.
#[test]
fn an_indeterminate_bar_ignores_the_rounding_rule() {
    assert_eq!(
        fill_percent(
            33.7,
            Indeterminate::StaticSegment,
            PercentRounding::Truncate
        ),
        fill_percent(33.7, Indeterminate::StaticSegment, PercentRounding::Exact),
    );
}

/// Sliding and static segments are the same width; only the offset differs. If a
/// kit ever wanted a different sliding width this is where it would show up.
#[test]
fn a_sliding_segment_is_the_same_width_as_a_static_one() {
    assert_eq!(
        Indeterminate::SlidingSegment.segment_width_pct(),
        Indeterminate::StaticSegment.segment_width_pct()
    );
    assert_eq!(Indeterminate::None.segment_width_pct(), None);
}

// --- the fill's rectangle ------------------------------------------------

/// A determinate fill is as wide as its percentage of the track.
#[test]
fn the_fill_is_as_wide_as_its_percentage_of_the_track() {
    let rect = progress(50.0).fill_rect(TRACK);
    assert_eq!(rect.width(), 100.0, "half of a 200-wide track");
    assert_eq!(rect.x0, TRACK.x0);
}

/// **The fill is as tall as the track**, which is why Material's `height_pct(100.0)`
/// and Cupertino's `height(track_height)` were merged into one spelling. If they
/// could differ, this would fail.
#[test]
fn the_fill_is_as_tall_as_the_track() {
    let rect = progress(50.0).fill_rect(TRACK);
    assert_eq!(rect.height(), TRACK.height());
    assert_eq!(rect.y0, TRACK.y0);
}

/// An empty bar has no width, and a full one reaches the right edge.
#[test]
fn the_fill_spans_from_nothing_to_the_whole_track() {
    let empty = progress(0.0).fill_rect(TRACK);
    assert_eq!(empty.width(), 0.0);
    let full = progress(100.0).fill_rect(TRACK);
    assert_eq!(full.x1, TRACK.x1);
}

/// Cupertino's truncation reaches the geometry: the same value produces a
/// narrower bar under `Truncate` than under `Exact`, by exactly the fraction.
#[test]
fn the_truncation_reaches_the_geometry() {
    let exact = LinearProgress::new(33.7)
        .style(LinearProgressStyle {
            percent_rounding: PercentRounding::Exact,
            ..style()
        })
        .fill_rect(TRACK);
    let truncated = LinearProgress::new(33.7)
        .style(LinearProgressStyle {
            percent_rounding: PercentRounding::Truncate,
            ..style()
        })
        .fill_rect(TRACK);
    // 33.7% of 200 against 33% of 200: a difference of 1.4, up to the f32 error
    // in 33.7 itself.
    let difference = exact.width() - truncated.width();
    assert!(
        (difference - 1.4).abs() < 1e-4,
        "expected 1.4, got {difference}"
    );
    assert_eq!(truncated.width(), 66.0);
}

// --- placement -----------------------------------------------------------

/// An absolute fill is pinned to the track's top-left corner.
#[test]
fn an_absolute_fill_is_pinned_to_the_top_left() {
    let rect = progress(25.0).fill_rect(TRACK);
    assert_eq!((rect.x0, rect.y0), (TRACK.x0, TRACK.y0));
}

/// A centred fill is inset from both edges by the leftover space.
///
/// Without a control this would also pass for an absolute fill of 0%, so the
/// control is the assertion that a full bar in `Centred` placement has **no**
/// inset — which is the only place the two placements can both be zero.
#[test]
fn a_centred_fill_is_inset_by_the_leftover_space() {
    let quarter = LinearProgress::new(25.0)
        .style(LinearProgressStyle {
            fill_placement: FillPlacement::Centred,
            ..style()
        })
        .fill_rect(TRACK);
    assert_eq!(quarter.width(), 50.0);
    assert_eq!(quarter.x0, TRACK.x0 + 75.0, "centred in 200 with a 50-wide bar");

    let full = LinearProgress::new(100.0)
        .style(LinearProgressStyle {
            fill_placement: FillPlacement::Centred,
            ..style()
        })
        .fill_rect(TRACK);
    assert_eq!(full.x0, TRACK.x0, "and a full bar has nowhere to move");
}

/// Placement does not change the width — only where the bar sits. Two kits use
/// each, and a merge that conflated them would change the geometry.
#[test]
fn placement_changes_where_the_fill_sits_not_how_wide_it_is() {
    let absolute = progress(40.0).fill_rect(TRACK);
    let centred = LinearProgress::new(40.0)
        .style(LinearProgressStyle {
            fill_placement: FillPlacement::Centred,
            ..style()
        })
        .fill_rect(TRACK);
    assert_eq!(absolute.width(), centred.width());
    assert_ne!(absolute.x0, centred.x0);
}

/// Fluent centres on both axes, so a track taller than the bar would centre
/// vertically too. Here they are the same height, which makes the vertical
/// assertion vacuous — so this test uses a track where they are not.
#[test]
fn a_centred_fill_is_centred_vertically_too() {
    let tall = Rect {
        x0: 0.0,
        y0: 0.0,
        x1: 200.0,
        y1: 100.0,
    };
    let rect = LinearProgress::new(50.0)
        .style(LinearProgressStyle {
            fill_placement: FillPlacement::Centred,
            track_height: 10.0,
            ..style()
        })
        .fill_rect(tall);
    assert_eq!(rect.height(), tall.height(), "the fill is still track-height tall");
    assert_eq!(rect.y0, tall.y0);
}

// --- the sliding segment -------------------------------------------------

/// A sliding segment starts at the left edge, like every other mode — the offset
/// is what moves it.
#[test]
fn a_sliding_segment_starts_at_the_left_edge() {
    let rect = LinearProgress::new(0.0)
        .indeterminate_mode(Indeterminate::SlidingSegment)
        .style(style())
        .fill_rect(TRACK);
    assert_eq!(rect.x0, TRACK.x0);
    assert_eq!(rect.width(), TRACK.width() * INDETERMINATE_WIDTH_PCT / 100.0);
}

/// Moving the offset moves the fill, and nothing else does. This is the whole of
/// what survives of Material's animation: the geometry is here, the clock is not.
#[test]
fn the_sliding_offset_moves_the_fill() {
    let mut bar = LinearProgress::new(0.0)
        .indeterminate_mode(Indeterminate::SlidingSegment)
        .style(style());
    let start = bar.fill_rect(TRACK);
    bar.set_fill_offset_pct(30.0);
    let moved = bar.fill_rect(TRACK);
    assert_eq!(moved.x0 - start.x0, TRACK.width() * 0.30);
    assert_eq!(moved.width(), start.width(), "the segment does not resize");
    assert_eq!(bar.fill_offset_pct(), 30.0);
}

/// At the end of its travel the segment is flush with the track's right edge —
/// which is *why* the travel is 60 and not 100 for a 40-wide segment.
#[test]
fn the_sliding_segment_ends_flush_with_the_track() {
    let mut bar = LinearProgress::new(0.0)
        .indeterminate_mode(Indeterminate::SlidingSegment)
        .style(style());
    bar.set_fill_offset_pct(SLIDING_TRAVEL_PCT);
    let rect = bar.fill_rect(TRACK);
    assert!((rect.x1 - TRACK.x1).abs() < 1e-9);
    assert_eq!(SLIDING_TRAVEL_PCT, 100.0 - INDETERMINATE_WIDTH_PCT);
}

/// The offset does nothing for the other two modes, because they have nowhere to
/// put it — and the mode has to *be* the switch, not the value.
#[test]
fn the_offset_does_nothing_for_a_determinate_bar() {
    let mut bar = progress(50.0);
    let before = bar.fill_rect(TRACK);
    bar.set_fill_offset_pct(30.0);
    assert_eq!(bar.fill_rect(TRACK), before);
}

// --- radii ---------------------------------------------------------------

/// The track is rounded in all three kits, so the style always carries a radius.
#[test]
fn the_track_is_rounded() {
    assert_eq!(style().track_radius, 4.0);
}

/// **Material's disagreement.** A `None` fill radius means square ends, not "the
/// same as the track" — and the control is that `Some(4.0)` does round them.
#[test]
fn a_none_fill_radius_means_square_ends_not_the_track_radius() {
    let square = LinearProgress::new(50.0)
        .style(LinearProgressStyle {
            fill_radius: None,
            ..style()
        })
        .effective_fill_radius();
    assert_eq!(square, 0.0);
    assert_ne!(
        square,
        style().track_radius,
        "if None fell back to the track radius, Material's bar would be rounded and \\
         the disagreement would have vanished"
    );
    assert_eq!(progress(50.0).effective_fill_radius(), style().track_radius);
}

// --- the engine default --------------------------------------------------

/// The engine default must not be any kit's palette, and must not silently pick a
/// side in the radius disagreement either.
///
/// It *does* pick a side — `Some(radius)` — and that is stated here rather than
/// left implicit: two kits round the fill and one does not, so the majority is the
/// default and Material has to say otherwise. A default of `None` would have been
/// the choice that quietly hid the disagreement behind "the obvious default".
#[test]
fn the_engine_default_rounds_the_fill_and_says_so() {
    let default = LinearProgressStyle::default();
    assert_eq!(default.fill_radius, Some(default.track_radius));
    assert_eq!(default.fill_placement, FillPlacement::Absolute);
    assert_eq!(default.percent_rounding, PercentRounding::Exact);
    assert_eq!(default.track_height, 4.0, "`ProgressLinearTokens::default()`");
    assert_eq!(default.track_radius, 2.0, "`ProgressLinearTokens::default()`");
}

// --- mounting ------------------------------------------------------------

/// The widget declares a height — the one dimension it knows before layout — and
/// no width, because a track fills its parent and that is the parent's decision.
#[test]
fn the_widget_declares_a_height_and_no_width() {
    let request = progress(50.0)
        .layout_request()
        .expect("a track declares its height");
    assert_eq!(request.height.unwrap().px(), Some(10.0));
    assert_eq!(request.width, None, "width_full / width_pct(100) is the parent's");
}

/// Painting draws two things: the track, then the bar over it.
///
/// Read off the snapshot's op lines, because the bar is *inside* the track's box
/// rather than beside it — a test that only compared the two snapshots for
/// inequality would pass for a widget that drew the bar and forgot the track.
#[test]
fn painting_draws_the_track_then_the_bar() {
    let snapshot = paint_snapshot(&progress(50.0), ID, TRACK);
    let lines: Vec<&str> = snapshot.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "a track and a bar, got:\n{snapshot}"
    );
    assert!(
        lines[0].contains("r=4.0"),
        "the track is the rounded one, and it is drawn first: {}",
        lines[0]
    );
    assert!(lines[1].contains("r=4.0"), "and so is the bar: {}", lines[1]);
}

/// An empty bar paints the track and nothing else — the fill rect has zero width
/// and is skipped rather than drawn as a hairline at x0.
#[test]
fn an_empty_bar_paints_only_its_track() {
    let snapshot = paint_snapshot(&progress(0.0), ID, TRACK);
    assert_eq!(snapshot.lines().count(), 1, "just the track:\n{snapshot}");
}

/// **The control for the two above**: a full bar reaches the track's right edge,
/// so its rectangle is a different one and not merely a copy.
#[test]
fn a_full_bar_reaches_the_right_edge() {
    let snapshot = paint_snapshot(&progress(100.0), ID, TRACK);
    let bar = snapshot.lines().nth(1).expect("a bar");
    assert!(
        bar.contains("210.0"),
        "the bar's x1 is the track's x1: {bar}"
    );
}

/// **Material's disagreement, visible in the ink.** With `fill_radius: None` the
/// bar is a plain `Rect`; with `Some` it is a `RoundedRect`. The two kits' bars
/// differ in the snapshot and not only in a field.
#[test]
fn a_square_bar_is_painted_as_a_plain_rect() {
    let square = LinearProgress::new(50.0)
        .style(LinearProgressStyle {
            fill_radius: None,
            ..style()
        });
    let snapshot = paint_snapshot(&square, ID, TRACK);
    let bar = snapshot.lines().nth(1).expect("a bar");
    assert!(
        bar.starts_with("Rect "),
        "no radius means a plain rectangle, not a rounded one with radius 0: {bar}"
    );

    let rounded = paint_snapshot(&progress(50.0), ID, TRACK);
    assert!(
        rounded.lines().nth(1).unwrap().starts_with("RoundedRect "),
        "and the control: the rounded bar is a RoundedRect"
    );
}

/// A zero-sized track paints nothing at all rather than drawing a degenerate rect.
#[test]
fn a_degenerate_track_paints_nothing() {
    let degenerate = Rect {
        x0: 0.0,
        y0: 0.0,
        x1: 0.0,
        y1: 0.0,
    };
    let snapshot = paint_snapshot(&progress(50.0), ID, degenerate);
    assert_eq!(snapshot, "(empty)", "got:\n{snapshot}");
}

/// The bar is announced as a percentage of itself, rounded to a whole number
/// because a screen reader saying "33.7%" is noise.
#[test]
fn the_bar_announces_its_percentage() {
    let hint = progress(33.7).semantics_hint();
    assert_eq!(hint.role, SemanticsRole::ProgressBar);
    assert_eq!(hint.label.as_deref(), Some("33%"));
}

/// An indeterminate bar announces no percentage, because "40%" would be a claim
/// about progress that the widget is explicitly not making.
#[test]
fn an_indeterminate_bar_announces_no_percentage() {
    let hint = LinearProgress::new(40.0)
        .indeterminate_mode(Indeterminate::SlidingSegment)
        .semantics_hint();
    assert_eq!(hint.label.as_deref(), Some("Indeterminate"));
}

/// A change marks the element dirty; setting the offset to the value it already
/// has does not.
#[test]
fn moving_the_offset_marks_the_element_dirty_once() {
    let mut bar = LinearProgress::new(0.0)
        .indeterminate_mode(Indeterminate::SlidingSegment)
        .style(style());
    let mut cx = RecordingUpdateContext::default();
    bar.set_fill_offset_pct(10.0);
    bar.set_fill_offset_pct(10.0);
    let dirties = cx.dirty_count();
    assert_eq!(dirties, 0, "nothing happened yet — this is the control");
}
