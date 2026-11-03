//! Chart tests: scale arithmetic, headless geometry, and the accessible
//! summary.
//!
//! The chart's numbers are all computed without a renderer, which is the point
//! of "renderer-neutral primitives": the geometry can be asserted exactly, and
//! the painted output is only a projection of it.

use floom_core::{ElementId, Rect, Widget};
use floom_widgets::testkit::{AllocatingCreateContext, SceneRecorder, semantics_snapshot};
use floom_widgets_data::chart::{AxisSpec, Chart, ChartSeries, DataPoint, LinearScale, SeriesKind};

const PLOT: Rect = Rect {
    x0: 40.0,
    y0: 10.0,
    x1: 340.0,
    y1: 210.0,
};

fn allocate(widget: &dyn Widget) -> ElementId {
    let mut create = AllocatingCreateContext::new();
    widget.create(&mut create)
}

fn line(name: &str, points: &[(f64, f64)]) -> ChartSeries {
    ChartSeries::line(name, points.iter().copied())
}

fn bars(name: &str, points: &[(f64, f64)]) -> ChartSeries {
    ChartSeries::bars(name, points.iter().copied())
}

#[test]
fn a_linear_scale_maps_its_domain_onto_its_range() {
    let s = LinearScale::new(0.0, 10.0, 0.0, 100.0);
    assert!((s.map(0.0) - 0.0).abs() < 1e-9);
    assert!((s.map(5.0) - 50.0).abs() < 1e-9);
    assert!((s.map(10.0) - 100.0).abs() < 1e-9);
    // Values outside the domain extrapolate rather than clamp, so a caller
    // that wants clamping can do it and one that wants the true position is
    // not silently lied to.
    assert!((s.map(12.0) - 120.0).abs() < 1e-9);
}

#[test]
fn a_flipped_range_is_how_screen_y_works() {
    // Screen y grows downward, so the y scale's range is inverted; the same
    // struct must express that without a special case.
    let s = LinearScale::new(0.0, 100.0, 210.0, 10.0);
    assert!(
        (s.map(0.0) - 210.0).abs() < 1e-9,
        "the bottom of the data is low on screen"
    );
    assert!(
        (s.map(100.0) - 10.0).abs() < 1e-9,
        "the top of the data is high on screen"
    );
    assert!((s.invert(s.map(37.0)) - 37.0).abs() < 1e-6);
}

#[test]
fn a_degenerate_domain_maps_to_the_middle_instead_of_dividing_by_zero() {
    // A single-point series is common and must not produce NaN geometry.
    let s = LinearScale::new(5.0, 5.0, 0.0, 100.0);
    let mapped = s.map(5.0);
    assert!(
        mapped.is_finite(),
        "mapping a flat domain produced {mapped}"
    );
    assert!(
        (mapped - 50.0).abs() < 1e-9,
        "a flat domain should sit centred, got {mapped}"
    );
}

#[test]
fn bounds_span_every_series() {
    let chart = Chart::new()
        .series(line("a", &[(1.0, 2.0), (3.0, 4.0)]))
        .series(bars("b", &[(0.0, -1.0), (2.0, 9.0)]));
    assert_eq!(chart.x_bounds(), Some((0.0, 3.0)));
    assert_eq!(chart.y_bounds(), Some((-1.0, 9.0)));
}

#[test]
fn explicit_axis_limits_win_over_the_data() {
    let chart = Chart::new()
        .series(line("a", &[(1.0, 2.0), (3.0, 4.0)]))
        .y_axis(AxisSpec::new("Revenue").limits(-10.0, 10.0));
    assert_eq!(chart.y_bounds(), Some((-10.0, 10.0)));
    assert_eq!(
        chart.x_bounds(),
        Some((1.0, 3.0)),
        "the x axis still auto-fits"
    );
}

#[test]
fn a_zero_baseline_is_added_to_a_range_that_would_hide_the_bars() {
    // A bar chart whose values are all 8..12 must still show the axis from 0,
    // or the bars silently misrepresent their magnitude.
    let chart = Chart::new().series(bars("a", &[(0.0, 8.0), (1.0, 12.0)]));
    assert_eq!(chart.y_bounds(), Some((0.0, 12.0)));

    // A line chart is not a magnitude encoding, so it keeps a tight fit.
    let chart = Chart::new().series(line("a", &[(0.0, 8.0), (1.0, 12.0)]));
    assert_eq!(chart.y_bounds(), Some((8.0, 12.0)));
}

#[test]
fn ticks_land_on_the_plot_and_include_both_ends() {
    let chart = Chart::new().series(line("a", &[(0.0, 0.0), (10.0, 100.0)]));
    let ticks = chart.y_ticks(PLOT);
    assert_eq!(ticks.len(), 5, "default tick count");
    assert!((ticks.first().unwrap().value - 0.0).abs() < 1e-9);
    assert!((ticks.last().unwrap().value - 100.0).abs() < 1e-9);
    // Ascending value runs downward on screen.
    assert!(ticks[0].position > ticks[1].position);
    for t in &ticks {
        assert!(
            (PLOT.y0..=PLOT.y1).contains(&t.position),
            "tick at {} escapes the plot",
            t.position
        );
    }
}

#[test]
fn bar_heights_are_proportional_to_their_values() {
    let chart = Chart::new().series(bars("a", &[(0.0, 0.0), (1.0, 50.0), (2.0, 100.0)]));
    let chart = chart.y_axis(AxisSpec::new("y").limits(0.0, 100.0));
    let bars = chart.bars(PLOT);
    assert_eq!(bars.len(), 3);

    let height = |i: usize| bars[i].geom.height();
    assert!(height(0).abs() < 1e-6, "zero draws no bar");
    assert!(
        (height(2) - PLOT.height()).abs() < 1e-6,
        "the max fills the plot"
    );
    assert!((height(1) - PLOT.height() / 2.0).abs() < 1e-6);
}

#[test]
fn bars_sit_on_the_axis_and_stay_inside_the_plot() {
    let chart = Chart::new()
        .series(bars("a", &[(0.0, 10.0), (1.0, 20.0), (2.0, 30.0)]))
        .y_axis(AxisSpec::new("y").limits(0.0, 30.0));
    let bars = chart.bars(PLOT);
    for b in &bars {
        assert!(
            (b.geom.y1 - PLOT.y1).abs() < 1e-6,
            "bars grow up from the axis"
        );
        assert!(b.geom.x0 >= PLOT.x0 && b.geom.x1 <= PLOT.x1, "{:?}", b.geom);
        assert!(b.geom.y0 >= PLOT.y0, "{:?}", b.geom);
    }
    // Evenly spaced, in data order, and never overlapping.
    for pair in bars.windows(2) {
        assert!(pair[0].geom.x1 <= pair[1].geom.x0);
        let a = pair[0].geom.x1 - pair[0].geom.x0;
        let b = pair[1].geom.x1 - pair[1].geom.x0;
        assert!((a - b).abs() < 1e-6, "bar widths drifted: {a} vs {b}");
    }
    assert!(bars[0].geom.x0 < bars[2].geom.x0);
}

#[test]
fn a_line_series_yields_one_segment_per_gap() {
    let chart = Chart::new().series(line("a", &[(0.0, 0.0), (1.0, 5.0), (2.0, 3.0), (3.0, 9.0)]));
    let segments = chart.segments();
    assert_eq!(segments.len(), 3);
    assert_eq!(segments[0].series, 0);
    assert!((segments[0].from.x - 0.0).abs() < 1e-9);
    assert!((segments[0].to.x - 1.0).abs() < 1e-9);
}

#[test]
fn a_single_point_series_has_no_segments_but_does_have_a_marker() {
    let chart = Chart::new().series(line("a", &[(4.0, 4.0)]));
    assert!(chart.segments().is_empty());
    let markers = chart.markers(PLOT);
    assert_eq!(markers.len(), 1);
    // One distinct value is a degenerate domain, and centring it is the only
    // answer that does not divide by zero.
    assert!(
        (markers[0].at.x - (PLOT.x0 + PLOT.x1) / 2.0).abs() < 1e-6,
        "a lone point centres itself, got {}",
        markers[0].at.x
    );
    assert!((markers[0].at.y - (PLOT.y0 + PLOT.y1) / 2.0).abs() < 1e-6);
}

#[test]
fn stroking_cost_is_bounded_by_the_plot_width_not_the_point_count() {
    // Ten thousand points must not become ten thousand rectangles: the stroke
    // is rasterized into one column per device pixel of the plot.
    let points: Vec<(f64, f64)> = (0..10_000)
        .map(|i| (i as f64, ((i as f64) * 0.01).sin()))
        .collect();
    let chart = Chart::new().series(line("a", &points));
    let stroke = chart.stroke(PLOT);
    let width = PLOT.width().ceil() as usize;
    assert!(
        stroke.len() <= width,
        "{} stroke rects for a {width}px plot",
        stroke.len()
    );
    assert!(!stroke.is_empty());
    for column in &stroke {
        assert!(
            column.geom.x0 >= PLOT.x0 && column.geom.x1 <= PLOT.x1,
            "{:?} escapes the plot",
            column.geom
        );
    }
}

#[test]
fn a_flat_line_strokes_at_a_constant_height() {
    let chart = Chart::new()
        .series(line("a", &[(0.0, 5.0), (1.0, 5.0), (2.0, 5.0)]))
        .y_axis(AxisSpec::new("y").limits(0.0, 10.0));
    let stroke = chart.stroke(PLOT);
    assert!(!stroke.is_empty());
    let y = stroke[0].geom.y0;
    for column in &stroke {
        assert!(
            (column.geom.y0 - y).abs() < 1e-6,
            "flat data wobbled: {:?}",
            column.geom
        );
    }
    // Half the plot height = the middle of the box.
    assert!((y - (PLOT.y0 + PLOT.y1) / 2.0).abs() < 1e-6, "got {y}");
}

#[test]
fn the_summary_names_the_kind_axes_extremes_and_peak() {
    let chart = Chart::new()
        .series(bars("sales", &[(0.0, 10.0), (1.0, 90.0), (2.0, 30.0)]))
        .x_axis(AxisSpec::new("Quarter"))
        .y_axis(AxisSpec::new("Revenue"));
    let summary = chart.summary();
    assert_eq!(summary.series, 1);
    assert_eq!(summary.points, 3);
    assert_eq!(
        summary.peak.as_ref().map(|(n, _)| n.as_str()),
        Some("sales")
    );

    let text = summary.describe();
    for needle in [
        "Bar chart",
        "1 series",
        "3 points",
        "Quarter",
        "Revenue",
        "90",
    ] {
        assert!(text.contains(needle), "{needle:?} missing from {text:?}");
    }
}

#[test]
fn a_mixed_chart_says_so_rather_than_picking_one_kind() {
    let chart = Chart::new()
        .series(bars("a", &[(0.0, 1.0)]))
        .series(line("b", &[(0.0, 1.0)]));
    assert!(chart.summary().describe().contains("Mixed chart"));
}

#[test]
fn an_empty_chart_says_it_has_no_data() {
    let chart = Chart::new();
    assert_eq!(chart.x_bounds(), None);
    assert_eq!(chart.y_bounds(), None);
    let text = chart.summary().describe();
    assert!(text.contains("no data"), "got {text:?}");
}

#[test]
fn the_chart_paints_a_frame_and_publishes_its_summary_to_assistive_tech() {
    let chart = Chart::new()
        .series(bars("sales", &[(0.0, 10.0), (1.0, 90.0), (2.0, 30.0)]))
        .y_axis(AxisSpec::new("Revenue"));
    let id = allocate(&chart);

    let mut rec = SceneRecorder::new(PLOT);
    chart.paint(&mut rec, id);
    rec.finish();
    assert!(
        !rec.ops().is_empty(),
        "a chart with data must draw something"
    );

    let label = semantics_snapshot(&chart, id, PLOT);
    assert!(
        label.contains("Bar chart"),
        "the accessible summary is the chart's only text surface: {label:?}"
    );
}

#[test]
fn series_builders_keep_the_kind_and_name() {
    let s = ChartSeries::scatter("cloud", [(0.0, 0.0), (1.0, 1.0)]);
    assert_eq!(s.name, "cloud");
    assert_eq!(s.kind, SeriesKind::Scatter);
    assert_eq!(s.points.len(), 2);
    assert_eq!(s.points[0], DataPoint { x: 0.0, y: 0.0 });
    // An area series is stroked *and* filled; the flag is what says so.
    assert!(SeriesKind::Area.is_filled());
    assert!(!SeriesKind::Line.is_filled());
    assert!(SeriesKind::Bar.is_filled());
    assert!(!SeriesKind::Scatter.is_filled());
}
