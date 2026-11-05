// SPDX-License-Identifier: MIT
//! Renderer-neutral chart primitives.
//!
//! A chart is two things: geometry computed from numbers, and a sentence
//! describing itself. Both are produced here without a renderer, which is what
//! makes the geometry exactly assertable — [`Chart::bars`], [`Chart::stroke`]
//! and [`Chart::markers`] are plain arithmetic over a [`Rect`], and
//! [`Chart::paint`] is only a projection of them onto whatever the paint
//! contract can express.
//!
//! Because that contract has no stroke primitive, a line is rasterized into one
//! column per device pixel of the plot. The cost is therefore bounded by the
//! *plot width*, not by the point count — a 10k-point series still strokes in
//! one pass over the same 300 columns.

use std::any::Any;
use std::cmp::Ordering;

use floom_accessibility::{SemanticsRole, SemanticsState};
use floom_core::{
    Color, CreateContext, ElementId, LayoutRequest, PaintContext, Point, Rect, SemanticsContext,
    UpdateContext, Widget, WidgetAny,
};
use floom_widgets::{SemanticsHint, to_core_role, to_core_state};

/// Room reserved around the plot for axes and labels.
const AXIS_MARGIN: f64 = 24.0;
/// Stroke thickness in logical pixels.
const STROKE_THICKNESS: f64 = 1.0;
/// Horizontal gap between a bar and its band edge.
const BAR_INSET: f64 = 1.0;

const PLOT_BG: Color = Color::rgb(0xFC, 0xFD, 0xFE);
const GRID_LINE: Color = Color::rgb(0xE6, 0xEA, 0xEF);

/// Series ink, cycled by series index so a chart is readable without a legend
/// lookup for the common small-N case.
const SERIES_INK: [Color; 6] = [
    Color::rgb(0x2E, 0x6C, 0xC4),
    Color::rgb(0xD1, 0x5B, 0x2A),
    Color::rgb(0x2E, 0x9E, 0x6B),
    Color::rgb(0x8B, 0x4F, 0xC4),
    Color::rgb(0xC4, 0xA1, 0x2E),
    Color::rgb(0xC4, 0x2E, 0x5B),
];

fn series_ink(index: usize) -> Color {
    SERIES_INK[index % SERIES_INK.len()]
}

/// Prints a number the way a chart label should: no trailing `.0`, and never a
/// value that reads as an integer when it is not one.
fn fmt_number(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// One sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DataPoint {
    pub x: f64,
    pub y: f64,
}

impl DataPoint {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeriesKind {
    Line,
    Bar,
    Area,
    Scatter,
}

impl SeriesKind {
    /// Whether the kind encodes magnitude, and therefore needs a zero baseline.
    pub const fn is_filled(self) -> bool {
        matches!(self, SeriesKind::Bar | SeriesKind::Area)
    }

    pub const fn noun(self) -> &'static str {
        match self {
            SeriesKind::Line => "Line chart",
            SeriesKind::Bar => "Bar chart",
            SeriesKind::Area => "Area chart",
            SeriesKind::Scatter => "Scatter plot",
        }
    }
}

/// One named series of points.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartSeries {
    pub name: String,
    pub kind: SeriesKind,
    pub points: Vec<DataPoint>,
}

impl ChartSeries {
    pub fn new(
        name: impl Into<String>,
        kind: SeriesKind,
        points: impl IntoIterator<Item = (f64, f64)>,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            points: points
                .into_iter()
                .map(|(x, y)| DataPoint::new(x, y))
                .collect(),
        }
    }

    pub fn line(name: impl Into<String>, points: impl IntoIterator<Item = (f64, f64)>) -> Self {
        Self::new(name, SeriesKind::Line, points)
    }

    pub fn bars(name: impl Into<String>, points: impl IntoIterator<Item = (f64, f64)>) -> Self {
        Self::new(name, SeriesKind::Bar, points)
    }

    pub fn area(name: impl Into<String>, points: impl IntoIterator<Item = (f64, f64)>) -> Self {
        Self::new(name, SeriesKind::Area, points)
    }

    pub fn scatter(name: impl Into<String>, points: impl IntoIterator<Item = (f64, f64)>) -> Self {
        Self::new(name, SeriesKind::Scatter, points)
    }
}

/// Axis configuration. `min`/`max` are `None` to auto-fit the data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxisSpec {
    pub title: String,
    pub min: Option<OrderedF64>,
    pub max: Option<OrderedF64>,
    /// Tick marks to draw, endpoints included.
    pub ticks: usize,
}

/// An `f64` wrapper with a total order, so axis limits can live in an `Eq`
/// struct without an `Eq`-on-`f64` hack.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrderedF64(pub f64);

impl Eq for OrderedF64 {}

impl PartialOrd for OrderedF64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(Ordering::Equal)
    }
}

impl AxisSpec {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            min: None,
            max: None,
            ticks: 5,
        }
    }

    pub fn limits(mut self, min: f64, max: f64) -> Self {
        self.min = Some(OrderedF64(min));
        self.max = Some(OrderedF64(max));
        self
    }

    pub fn ticks(mut self, ticks: usize) -> Self {
        self.ticks = ticks.max(2);
        self
    }

    /// The explicit limits, when both ends were given.
    ///
    /// Half-specified limits are ignored rather than guessed at: an axis with
    /// only a `min` is almost always a caller mistake, and silently inventing a
    /// `max` would hide it.
    fn explicit_limits(&self) -> Option<(f64, f64)> {
        match (self.min, self.max) {
            (Some(min), Some(max)) => Some((min.0, max.0)),
            _ => None,
        }
    }
}

impl Default for AxisSpec {
    fn default() -> Self {
        Self::new("")
    }
}

/// Affine map from a data domain onto a screen range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearScale {
    domain_min: f64,
    domain_max: f64,
    range_min: f64,
    range_max: f64,
}

impl LinearScale {
    pub const fn new(domain_min: f64, domain_max: f64, range_min: f64, range_max: f64) -> Self {
        Self {
            domain_min,
            domain_max,
            range_min,
            range_max,
        }
    }

    /// Maps a value onto the range.
    ///
    /// A degenerate domain (one distinct value) maps to the *middle* rather
    /// than dividing by zero: a single-point series is common, and NaN geometry
    /// would be the worst possible answer to it.
    pub fn map(&self, value: f64) -> f64 {
        let span = self.domain_max - self.domain_min;
        let t = if span.abs() < f64::EPSILON {
            0.5
        } else {
            (value - self.domain_min) / span
        };
        self.range_min + t * (self.range_max - self.range_min)
    }

    pub fn invert(&self, position: f64) -> f64 {
        let span = self.range_max - self.range_min;
        let t = if span.abs() < f64::EPSILON {
            0.5
        } else {
            (position - self.range_min) / span
        };
        self.domain_min + t * (self.domain_max - self.domain_min)
    }

    pub const fn domain(&self) -> (f64, f64) {
        (self.domain_min, self.domain_max)
    }

    pub const fn range(&self) -> (f64, f64) {
        (self.range_min, self.range_max)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bar {
    pub series: usize,
    pub index: usize,
    pub value: f64,
    pub geom: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub series: usize,
    pub from: DataPoint,
    pub to: DataPoint,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Marker {
    pub series: usize,
    pub index: usize,
    pub at: Point,
}

/// One rasterized column of a stroke.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeColumn {
    pub series: usize,
    pub geom: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tick {
    pub value: f64,
    pub position: f64,
}

/// The sentence a chart says about itself.
///
/// This is the chart's entire accessible surface: it paints text only for axis
/// labels, so the summary is what assistive technology actually reads.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartSummary {
    pub series: usize,
    pub points: usize,
    pub kinds: Vec<SeriesKind>,
    /// Axis titles, carried here because the accessible description has to name
    /// the axes — a bare "0 to 2" is meaningless without the unit it is in.
    pub x_title: String,
    pub y_title: String,
    pub x_range: Option<(f64, f64)>,
    pub y_range: Option<(f64, f64)>,
    pub peak: Option<(String, f64)>,
}

impl ChartSummary {
    pub fn describe(&self) -> String {
        if self.series == 0 {
            return "Chart with no data".to_string();
        }
        let kind = match self.kinds.split_first() {
            Some((first, rest)) if rest.iter().all(|other| other == first) => first.noun(),
            _ => "Mixed chart",
        };
        let mut text = format!(
            "{kind} with {} series, {} point{}",
            self.series,
            self.points,
            if self.points == 1 { "" } else { "s" }
        );
        for (axis, title, range) in [
            ("x", &self.x_title, self.x_range),
            ("y", &self.y_title, self.y_range),
        ] {
            let Some((min, max)) = range else { continue };
            // Prefer the caller's title; fall back to the axis letter so the
            // range is never left dangling without a subject.
            let subject = if title.is_empty() {
                axis
            } else {
                title.as_str()
            };
            text.push_str(&format!(
                "; {subject} {} to {}",
                fmt_number(min),
                fmt_number(max)
            ));
        }
        if let Some((name, value)) = &self.peak {
            text.push_str(&format!("; peak {name} {}", fmt_number(*value)));
        }
        text
    }
}

/// A renderer-neutral chart.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Chart {
    series: Vec<ChartSeries>,
    x_axis: AxisSpec,
    y_axis: AxisSpec,
}

impl Chart {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn series(mut self, series: ChartSeries) -> Self {
        self.series.push(series);
        self
    }

    pub fn x_axis(mut self, axis: AxisSpec) -> Self {
        self.x_axis = axis;
        self
    }

    pub fn y_axis(mut self, axis: AxisSpec) -> Self {
        self.y_axis = axis;
        self
    }

    pub fn series_list(&self) -> &[ChartSeries] {
        &self.series
    }

    pub fn point_count(&self) -> usize {
        self.series.iter().map(|series| series.points.len()).sum()
    }

    /// X extent across every series, or the explicit limit when one is set.
    pub fn x_bounds(&self) -> Option<(f64, f64)> {
        if let Some(limits) = self.x_axis.explicit_limits() {
            return Some(limits);
        }
        let values: Vec<f64> = self
            .series
            .iter()
            .flat_map(|series| series.points.iter().map(|point| point.x))
            .filter(|x| x.is_finite())
            .collect();
        extent(&values)
    }

    /// Y extent across every series.
    ///
    /// A magnitude-encoding series (bar, area) forces a zero baseline: a bar
    /// chart drawn from 8 to 12 makes an 11 look nine times a 9, which is a
    /// worse lie than an axis that starts at zero is an inconvenience.
    pub fn y_bounds(&self) -> Option<(f64, f64)> {
        if let Some(limits) = self.y_axis.explicit_limits() {
            return Some(limits);
        }
        let values: Vec<f64> = self
            .series
            .iter()
            .flat_map(|series| series.points.iter().map(|point| point.y))
            .filter(|y| y.is_finite())
            .collect();
        let (min, max) = extent(&values)?;
        if self.series.iter().any(|series| series.kind.is_filled()) {
            Some((min.min(0.0), max.max(0.0)))
        } else {
            Some((min, max))
        }
    }

    /// The plotting box inside `bounds`.
    pub fn plot_rect(bounds: Rect) -> Rect {
        Rect {
            x0: bounds.x0 + AXIS_MARGIN,
            y0: bounds.y0 + AXIS_MARGIN,
            x1: (bounds.x1 - AXIS_MARGIN).max(bounds.x0 + AXIS_MARGIN),
            y1: (bounds.y1 - AXIS_MARGIN).max(bounds.y0 + AXIS_MARGIN),
        }
    }

    /// Data-to-screen scales for `plot`.
    ///
    /// The y range is inverted because screen y grows downward; that is the
    /// only place the flip lives, so no caller has to remember it.
    pub fn scales(&self, plot: Rect) -> Option<(LinearScale, LinearScale)> {
        let (x_min, x_max) = self.x_bounds()?;
        let (y_min, y_max) = self.y_bounds()?;
        Some((
            LinearScale::new(x_min, x_max, plot.x0, plot.x1),
            LinearScale::new(y_min, y_max, plot.y1, plot.y0),
        ))
    }

    /// Tick marks for the y axis, endpoints included, ascending in value —
    /// which is *descending* on screen.
    pub fn y_ticks(&self, plot: Rect) -> Vec<Tick> {
        let Some((min, max)) = self.y_bounds() else {
            return Vec::new();
        };
        let count = self.y_axis.ticks.max(2);
        let intervals = (count - 1) as f64;
        (0..count)
            .map(|index| {
                let t = index as f64 / intervals;
                Tick {
                    value: min + t * (max - min),
                    position: plot.y1 - t * plot.height(),
                }
            })
            .collect()
    }

    /// Bars grow up from the axis baseline, grouped by series within each slot.
    pub fn bars(&self, plot: Rect) -> Vec<Bar> {
        let Some((_, y_scale)) = self.scales(plot) else {
            return Vec::new();
        };
        let slots = self
            .series
            .iter()
            .map(|series| series.points.len())
            .max()
            .unwrap_or(0);
        if slots == 0 || plot.width() <= 0.0 {
            return Vec::new();
        }
        let (y_min, y_max) = self.y_bounds().expect("scales succeeded");
        let baseline = y_scale.map(0.0f64.clamp(y_min, y_max));
        let bar_series = self
            .series
            .iter()
            .filter(|series| series.kind == SeriesKind::Bar)
            .count()
            .max(1);
        let band = plot.width() / slots as f64;
        let sub = band / bar_series as f64;

        let mut out = Vec::new();
        let mut lane = 0usize;
        for (series_index, series) in self.series.iter().enumerate() {
            if series.kind != SeriesKind::Bar {
                continue;
            }
            for (index, point) in series.points.iter().enumerate() {
                let left = plot.x0 + index as f64 * band + lane as f64 * sub + BAR_INSET;
                let right = plot.x0 + (index as f64 + 1.0) * band + lane as f64 * sub - BAR_INSET;
                if right <= left {
                    continue;
                }
                let top = y_scale.map(point.y);
                out.push(Bar {
                    series: series_index,
                    index,
                    value: point.y,
                    geom: Rect {
                        x0: left,
                        y0: top.min(baseline),
                        x1: right,
                        y1: top.max(baseline),
                    },
                });
            }
            lane += 1;
        }
        out
    }

    /// Polyline segments for line and area series, in data space.
    pub fn segments(&self) -> Vec<Segment> {
        let mut out = Vec::new();
        for (series_index, series) in self.series.iter().enumerate() {
            if !matches!(series.kind, SeriesKind::Line | SeriesKind::Area) {
                continue;
            }
            for pair in series.points.windows(2) {
                out.push(Segment {
                    series: series_index,
                    from: pair[0],
                    to: pair[1],
                });
            }
        }
        out
    }

    /// Point markers for line and scatter series, in screen space.
    pub fn markers(&self, plot: Rect) -> Vec<Marker> {
        let Some((x_scale, y_scale)) = self.scales(plot) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (series_index, series) in self.series.iter().enumerate() {
            if !matches!(series.kind, SeriesKind::Line | SeriesKind::Scatter) {
                continue;
            }
            for (index, point) in series.points.iter().enumerate() {
                out.push(Marker {
                    series: series_index,
                    index,
                    at: Point::new(x_scale.map(point.x), y_scale.map(point.y)),
                });
            }
        }
        out
    }

    /// Strokes every line/area series, one column per device pixel of `plot`.
    ///
    /// Sampling by column rather than by segment is what bounds the cost: a
    /// 10k-point series and a 2-point series both produce one column per pixel.
    pub fn stroke(&self, plot: Rect) -> Vec<StrokeColumn> {
        let Some((x_scale, y_scale)) = self.scales(plot) else {
            return Vec::new();
        };
        let columns = plot.width().ceil() as usize;
        if columns == 0 {
            return Vec::new();
        }
        let mut out = Vec::new();
        for column in 0..columns {
            let left = plot.x0 + column as f64;
            let right = (left + STROKE_THICKNESS * 1.0).min(plot.x1);
            if right <= left {
                continue;
            }
            let data_x = x_scale.invert(left + 0.5);
            for (series_index, series) in self.series.iter().enumerate() {
                if !matches!(series.kind, SeriesKind::Line | SeriesKind::Area) {
                    continue;
                }
                let Some(y_value) = sample(series, data_x) else {
                    continue;
                };
                let screen_y = y_scale.map(y_value);
                if !screen_y.is_finite() {
                    continue;
                }
                out.push(StrokeColumn {
                    series: series_index,
                    geom: Rect {
                        x0: left,
                        y0: screen_y,
                        x1: right,
                        y1: screen_y + STROKE_THICKNESS,
                    },
                });
            }
        }
        out
    }

    pub fn summary(&self) -> ChartSummary {
        ChartSummary {
            series: self.series.len(),
            points: self.point_count(),
            kinds: self.series.iter().map(|series| series.kind).collect(),
            x_title: self.x_axis.title.clone(),
            y_title: self.y_axis.title.clone(),
            x_range: self.x_bounds(),
            y_range: self.y_bounds(),
            peak: self
                .series
                .iter()
                .flat_map(|series| {
                    series
                        .points
                        .iter()
                        .map(|point| (series.name.clone(), point.y))
                })
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal)),
        }
    }
}

fn extent(values: &[f64]) -> Option<(f64, f64)> {
    let mut iter = values.iter().copied();
    let first = iter.next()?;
    Some(iter.fold((first, first), |(lo, hi), value| {
        (lo.min(value), hi.max(value))
    }))
}

/// Linear interpolation between the points bracketing `data_x`.
///
/// Outside the series the nearest endpoint is held, so a stroke spans the whole
/// plot instead of stopping where the data happens to start.
fn sample(series: &ChartSeries, data_x: f64) -> Option<f64> {
    let points = &series.points;
    match points.len() {
        0 => None,
        1 => Some(points[0].y),
        _ => {
            if data_x <= points[0].x {
                return Some(points[0].y);
            }
            let last = points[points.len() - 1];
            if data_x >= last.x {
                return Some(last.y);
            }
            let high = points
                .partition_point(|point| point.x < data_x)
                .clamp(1, points.len() - 1);
            let a = points[high - 1];
            let b = points[high];
            let span = b.x - a.x;
            if span.abs() < f64::EPSILON {
                return Some(b.y);
            }
            let t = (data_x - a.x) / span;
            Some(a.y + t * (b.y - a.y))
        }
    }
}

impl Widget for Chart {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        cx.mark_dirty(id);
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().fill())
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let plot = Self::plot_rect(bounds);
        if plot.width() <= 0.0 || plot.height() <= 0.0 {
            return;
        }
        cx.fill_rect(plot, PLOT_BG);

        for tick in self.y_ticks(plot) {
            cx.fill_rect(
                Rect {
                    x0: plot.x0,
                    y0: tick.position,
                    x1: plot.x1,
                    y1: tick.position + 1.0,
                },
                GRID_LINE,
            );
        }
        for bar in self.bars(plot) {
            if bar.geom.height() <= 0.0 {
                continue;
            }
            cx.fill_rect(bar.geom, series_ink(bar.series));
        }
        for column in self.stroke(plot) {
            cx.fill_rect(column.geom, series_ink(column.series));
        }
        for marker in self.markers(plot) {
            cx.fill_rect(
                Rect {
                    x0: marker.at.x - 1.5,
                    y0: marker.at.y - 1.5,
                    x1: marker.at.x + 1.5,
                    y1: marker.at.y + 1.5,
                },
                series_ink(marker.series),
            );
        }
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        cx.set_label(&self.summary().describe());
        cx.set_bounds(cx.bounds());
    }
}

impl WidgetAny for Chart {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        // A chart is a graphic with an accessible name, which is what `Image`
        // means to a screen reader; there is no chart role to claim.
        let hint = SemanticsHint {
            role: SemanticsRole::Image,
            label: Some(self.summary().describe()),
            state: SemanticsState::ENABLED,
        };
        Some(floom_core::SemanticsHint {
            role: to_core_role(hint.role),
            label: hint.label,
            state: to_core_state(hint.state),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scale_round_trips_through_its_inverse() {
        let scale = LinearScale::new(-5.0, 15.0, 100.0, 0.0);
        for value in [-5.0, 0.0, 7.5, 15.0] {
            assert!((scale.invert(scale.map(value)) - value).abs() < 1e-9);
        }
    }

    #[test]
    fn a_degenerate_range_still_maps_finitely() {
        let scale = LinearScale::new(0.0, 1.0, 5.0, 5.0);
        assert!(scale.map(0.5).is_finite());
        assert!((scale.map(0.5) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn the_accessed_limits_are_the_explicit_ones() {
        let axis = AxisSpec::new("y").limits(-1.0, 1.0);
        assert_eq!(axis.explicit_limits(), Some((-1.0, 1.0)));
        assert_eq!(AxisSpec::new("y").explicit_limits(), None);
    }

    #[test]
    fn summarizing_an_empty_chart_is_not_a_panic() {
        let summary = Chart::new().summary();
        assert_eq!(summary.series, 0);
        assert_eq!(summary.x_range, None);
        assert_eq!(summary.peak, None);
        assert_eq!(summary.describe(), "Chart with no data");
    }

    #[test]
    fn sampling_holds_the_endpoints_outside_the_data() {
        let series = ChartSeries::line("s", [(0.0, 1.0), (10.0, 11.0)]);
        assert_eq!(sample(&series, -100.0), Some(1.0));
        assert_eq!(sample(&series, 100.0), Some(11.0));
        assert_eq!(sample(&series, 5.0), Some(6.0));
    }

    #[test]
    fn a_single_point_series_samples_its_only_value() {
        let series = ChartSeries::line("s", [(3.0, 7.0)]);
        assert_eq!(sample(&series, 0.0), Some(7.0));
        assert_eq!(sample(&series, 100.0), Some(7.0));
        assert_eq!(sample(&ChartSeries::line("e", []), 0.0), None);
    }
}
