// SPDX-License-Identifier: MIT
//! Vector paths, stroke outlines, and clip regions.
//!
//! Everything here is pure geometry in logical pixels — no scene, no backend.
//! The path vocabulary lives at L0 because both the core paint contract
//! (`floom-core`'s `PaintContext`) and the scene IR (`floom-scene`) have to be
//! able to name it, and neither may reach down into a renderer to do so.
//!
//! ## What a path is, and what it is not
//!
//! A [`Path`] is a sequence of subpaths made of lines and Béziers, in absolute
//! logical coordinates. It is deliberately *not* a builder that flattens to
//! points at construction time: the same path is filled by one backend,
//! stroked by another, and encoded as curves by a third, so flattening is a
//! per-consumer decision ([`Path::flatten`]) and not a property of the value.
//!
//! ## Stroking
//!
//! [`Stroke::outline`] turns a path into closed polygons that, filled with the
//! **non-zero winding rule**, are exactly the stroked region. The trick that
//! makes it work: every emitted contour is normalised to positive orientation,
//! so overlapping pieces wind to 2 or more instead of cancelling to zero, and
//! the union of the pieces is what non-zero filling produces. That means a
//! stroked path and a filled path can share one rasteriser.

use crate::color::Color;
use crate::geometry::{Point, Rect, Transform};
use std::sync::Arc;

/// Flattening tolerance, in logical pixels: the maximum distance a flattened
/// polyline may stray from the true curve.
pub const DEFAULT_FLATTEN_TOLERANCE: f64 = 0.25;

/// One step of a [`Path`].
///
/// The first segment of a well-formed path is a `MoveTo`; a path that starts
/// with anything else implicitly starts at the origin, which is what every
/// consumer here assumes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathSeg {
    MoveTo(Point),
    LineTo(Point),
    QuadTo {
        ctrl: Point,
        to: Point,
    },
    CubicTo {
        c1: Point,
        c2: Point,
        to: Point,
    },
    /// Closes the current subpath back to its starting point.
    Close,
}

/// A vector path: subpaths made of lines, quadratic and cubic Béziers.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Path {
    segs: Vec<PathSeg>,
}

/// A flattened subpath.
///
/// `closed` records whether the path said so (a `Close` segment, or a
/// constructor that builds a loop). Filling ignores it — a subpath is always
/// closed for the purposes of the winding rule — but stroking must not: an
/// open subpath gets caps, a closed one gets a joint.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Polyline {
    pub points: Vec<Point>,
    pub closed: bool,
}

impl Polyline {
    pub fn new(points: Vec<Point>, closed: bool) -> Self {
        Self { points, closed }
    }
}

impl Path {
    pub const fn new() -> Self {
        Self { segs: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.segs.is_empty()
    }

    pub fn segments(&self) -> &[PathSeg] {
        &self.segs
    }

    pub fn move_to(&mut self, to: Point) -> &mut Self {
        self.segs.push(PathSeg::MoveTo(to));
        self
    }

    pub fn line_to(&mut self, to: Point) -> &mut Self {
        self.segs.push(PathSeg::LineTo(to));
        self
    }

    pub fn quad_to(&mut self, ctrl: Point, to: Point) -> &mut Self {
        self.segs.push(PathSeg::QuadTo { ctrl, to });
        self
    }

    pub fn cubic_to(&mut self, c1: Point, c2: Point, to: Point) -> &mut Self {
        self.segs.push(PathSeg::CubicTo { c1, c2, to });
        self
    }

    pub fn close(&mut self) -> &mut Self {
        self.segs.push(PathSeg::Close);
        self
    }

    /// A copy of this path moved by `(dx, dy)`.
    ///
    /// Every point moves, control points included. Translation is the one
    /// transform that commutes with the Bézier basis, so the moved path is the
    /// flattened original moved *exactly* — not a re-approximation of it. That
    /// is what lets a composite translate a self-drawn node by rewriting its
    /// path rather than by giving the renderer a transform it may not have
    /// (see `SceneNode::Group`).
    pub fn translate(&self, dx: f64, dy: f64) -> Self {
        let shift = |p: Point| Point::new(p.x + dx, p.y + dy);
        let segs = self
            .segs
            .iter()
            .map(|seg| match *seg {
                PathSeg::MoveTo(to) => PathSeg::MoveTo(shift(to)),
                PathSeg::LineTo(to) => PathSeg::LineTo(shift(to)),
                PathSeg::QuadTo { ctrl, to } => PathSeg::QuadTo {
                    ctrl: shift(ctrl),
                    to: shift(to),
                },
                PathSeg::CubicTo { c1, c2, to } => PathSeg::CubicTo {
                    c1: shift(c1),
                    c2: shift(c2),
                    to: shift(to),
                },
                PathSeg::Close => PathSeg::Close,
            })
            .collect();
        Self { segs }
    }

    /// A copy of this path with every point mapped through the affine `t`.
    ///
    /// An affine map carries Bézier control points to the control points of
    /// the mapped curve *exactly* — the same property translation has — so
    /// the result is the transformed path, not a re-approximation of one.
    pub fn transform(&self, t: Transform) -> Self {
        if t.is_identity() {
            return self.clone();
        }
        let map = |p: Point| t.map_point(p);
        let segs = self
            .segs
            .iter()
            .map(|seg| match *seg {
                PathSeg::MoveTo(to) => PathSeg::MoveTo(map(to)),
                PathSeg::LineTo(to) => PathSeg::LineTo(map(to)),
                PathSeg::QuadTo { ctrl, to } => PathSeg::QuadTo {
                    ctrl: map(ctrl),
                    to: map(to),
                },
                PathSeg::CubicTo { c1, c2, to } => PathSeg::CubicTo {
                    c1: map(c1),
                    c2: map(c2),
                    to: map(to),
                },
                PathSeg::Close => PathSeg::Close,
            })
            .collect();
        Self { segs }
    }

    /// A path tracing `bounds` clockwise from its top-left corner.
    pub fn rect(bounds: Rect) -> Self {
        let mut path = Self::new();
        path.move_to(Point::new(bounds.x0, bounds.y0))
            .line_to(Point::new(bounds.x1, bounds.y0))
            .line_to(Point::new(bounds.x1, bounds.y1))
            .line_to(Point::new(bounds.x0, bounds.y1))
            .close();
        path
    }

    /// A rectangle with uniformly rounded corners, clockwise.
    ///
    /// `radius` is clamped to half the shorter side, so a call that asks for
    /// more than a pill still produces a valid path rather than a
    /// self-intersecting one.
    pub fn rounded_rect(bounds: Rect, radius: f64) -> Self {
        let radius = radius.clamp(0.0, bounds.width().min(bounds.height()) / 2.0);
        if radius <= 0.0 {
            return Self::rect(bounds);
        }
        // The circle-to-Bézier constant: a quarter arc's control points sit
        // this fraction of the radius along the tangent from each endpoint.
        const K: f64 = 0.552_284_749_830_793_4;
        let (l, t, r, b) = (bounds.x0, bounds.y0, bounds.x1, bounds.y1);
        let mut path = Self::new();
        path.move_to(Point::new(l + radius, t))
            .line_to(Point::new(r - radius, t))
            .cubic_to(
                Point::new(r - radius + K * radius, t),
                Point::new(r, t + radius - K * radius),
                Point::new(r, t + radius),
            )
            .line_to(Point::new(r, b - radius))
            .cubic_to(
                Point::new(r, b - radius + K * radius),
                Point::new(r - radius + K * radius, b),
                Point::new(r - radius, b),
            )
            .line_to(Point::new(l + radius, b))
            .cubic_to(
                Point::new(l + radius - K * radius, b),
                Point::new(l, b - radius + K * radius),
                Point::new(l, b - radius),
            )
            .line_to(Point::new(l, t + radius))
            .cubic_to(
                Point::new(l, t + radius - K * radius),
                Point::new(l + radius - K * radius, t),
                Point::new(l + radius, t),
            )
            .close();
        path
    }

    /// A closed circular path.
    pub fn circle(center: Point, radius: f64) -> Self {
        let r = radius.abs();
        if r <= 0.0 {
            return Self::new();
        }
        Self::ellipse_arc(center, r, r, 0.0, std::f64::consts::TAU)
    }

    /// An elliptical arc starting at `start_rad` and sweeping `sweep_rad`.
    ///
    /// A positive sweep runs clockwise in screen coordinates (y down), which is
    /// the direction a progress ring fills. The sweep is split into quarter
    /// turns before approximating, because a single cubic cannot hold a wider
    /// arc without visible error.
    pub fn ellipse_arc(center: Point, rx: f64, ry: f64, start_rad: f64, sweep_rad: f64) -> Self {
        let (rx, ry) = (rx.abs(), ry.abs());
        let mut path = Self::new();
        if rx <= 0.0 || ry <= 0.0 || sweep_rad == 0.0 {
            return path;
        }
        let quarter = std::f64::consts::FRAC_PI_2;
        // Sign-preserving step count: at least one piece, never zero.
        let pieces = (sweep_rad.abs() / quarter).ceil().max(1.0);
        let step = sweep_rad / pieces;
        // A quadratic approximation of the control-point pull for a piece of
        // angle `step`: for a quarter turn this is exactly K.
        let k = 4.0 / 3.0 * (step / 4.0).tan();

        let at = |angle: f64| Point::new(center.x + rx * angle.cos(), center.y + ry * angle.sin());
        // Cubic control offsets along the local tangent direction, scaled to
        // the piece's radii.
        let ctrl = |angle: f64, sign: f64| {
            let tx = -rx * angle.sin();
            let ty = ry * angle.cos();
            Point::new(
                center.x + rx * angle.cos() + sign * k * tx,
                center.y + ry * angle.sin() + sign * k * ty,
            )
        };

        let mut angle = start_rad;
        path.move_to(at(angle));
        for _ in 0..(pieces as u32) {
            let next = angle + step;
            path.cubic_to(ctrl(angle, 1.0), ctrl(next, -1.0), at(next));
            angle = next;
        }
        path.close();
        path
    }

    /// The control hull of this path.
    ///
    /// For lines this is exact; for Béziers it is the convex hull of the
    /// control points, i.e. a superset of the true extent. That is the safe
    /// direction for both consumers here: a cull that uses it never skips a
    /// node that could have painted, and damage that uses it is never too
    /// small. Returns an empty rect for a path with no segments.
    pub fn bounds(&self) -> Rect {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut include = |p: Point| {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        };
        let mut start = Point::ZERO;
        for seg in &self.segs {
            match *seg {
                PathSeg::MoveTo(p) => {
                    include(p);
                    start = p;
                }
                PathSeg::LineTo(p) => include(p),
                PathSeg::QuadTo { ctrl, to } => {
                    include(ctrl);
                    include(to);
                }
                PathSeg::CubicTo { c1, c2, to } => {
                    include(c1);
                    include(c2);
                    include(to);
                }
                PathSeg::Close => include(start),
            }
        }
        if min_x > max_x {
            return Rect::ZERO;
        }
        Rect {
            x0: min_x,
            y0: min_y,
            x1: max_x,
            y1: max_y,
        }
    }

    /// Flattens every subpath into a polyline within `tolerance` of the curve.
    ///
    /// A subpath that only ever moved (no drawing segment) flattens to a single
    /// point, which stroking reads as the "dot" idiom: a round cap then draws a
    /// disc and a square cap a square.
    pub fn flatten(&self, tolerance: f64) -> Vec<Polyline> {
        let tolerance = if tolerance > 0.0 {
            tolerance
        } else {
            DEFAULT_FLATTEN_TOLERANCE
        };
        let mut out = Vec::new();
        let mut points: Vec<Point> = Vec::new();
        let mut start = Point::ZERO;
        let mut current = Point::ZERO;

        let flush = |points: &mut Vec<Point>, closed: bool, out: &mut Vec<Polyline>| {
            if !points.is_empty() {
                out.push(Polyline::new(std::mem::take(points), closed));
            }
        };

        for seg in &self.segs {
            match *seg {
                PathSeg::MoveTo(p) => {
                    flush(&mut points, false, &mut out);
                    points.push(p);
                    start = p;
                    current = p;
                }
                PathSeg::LineTo(p) => {
                    points.push(p);
                    current = p;
                }
                PathSeg::QuadTo { ctrl, to } => {
                    flatten_quad(current, ctrl, to, tolerance, &mut points, 0);
                    current = to;
                }
                PathSeg::CubicTo { c1, c2, to } => {
                    flatten_cubic(current, c1, c2, to, tolerance, &mut points, 0);
                    current = to;
                }
                PathSeg::Close => {
                    if points.first() != points.last() {
                        points.push(start);
                    }
                    flush(&mut points, true, &mut out);
                    current = start;
                }
            }
        }
        flush(&mut points, false, &mut out);
        out
    }
}

/// How many times a Bézier may be halved before a pathological control polygon
/// is accepted as-is. Without a cap, a degenerate curve (control points that
/// collapse onto the chord but never satisfy the flatness test) would recurse
/// until the stack runs out.
const MAX_FLATTEN_DEPTH: u32 = 16;

fn flatten_quad(
    p0: Point,
    ctrl: Point,
    p1: Point,
    tolerance: f64,
    out: &mut Vec<Point>,
    depth: u32,
) {
    if depth >= MAX_FLATTEN_DEPTH || point_line_distance(ctrl, p0, p1) <= tolerance {
        out.push(p1);
        return;
    }
    let m01 = midpoint(p0, ctrl);
    let m12 = midpoint(ctrl, p1);
    let m = midpoint(m01, m12);
    flatten_quad(p0, m01, m, tolerance, out, depth + 1);
    flatten_quad(m, m12, p1, tolerance, out, depth + 1);
}

fn flatten_cubic(
    p0: Point,
    c1: Point,
    c2: Point,
    p1: Point,
    tolerance: f64,
    out: &mut Vec<Point>,
    depth: u32,
) {
    if depth >= MAX_FLATTEN_DEPTH
        || (point_line_distance(c1, p0, p1) <= tolerance
            && point_line_distance(c2, p0, p1) <= tolerance)
    {
        out.push(p1);
        return;
    }
    let m01 = midpoint(p0, c1);
    let m12 = midpoint(c1, c2);
    let m23 = midpoint(c2, p1);
    let m012 = midpoint(m01, m12);
    let m123 = midpoint(m12, m23);
    let m = midpoint(m012, m123);
    flatten_cubic(p0, m01, m012, m, tolerance, out, depth + 1);
    flatten_cubic(m, m123, m23, p1, tolerance, out, depth + 1);
}

fn midpoint(a: Point, b: Point) -> Point {
    Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
}

/// Distance from `p` to the segment `a`–`b`.
///
/// A degenerate segment (`a == b`) falls back to the point distance, which is
/// what keeps a collapsed control polygon from producing a NaN and recursing
/// forever on a comparison against NaN.
fn point_line_distance(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len_sq = dx * dx + dy * dy;
    if len_sq <= f64::EPSILON {
        return ((p.x - a.x).powi(2) + (p.y - a.y).powi(2)).sqrt();
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len_sq).clamp(0.0, 1.0);
    let proj = Point::new(a.x + t * dx, a.y + t * dy);
    ((p.x - proj.x).powi(2) + (p.y - proj.y).powi(2)).sqrt()
}

/// How an open subpath ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum LineCap {
    /// Stops exactly at the endpoint.
    #[default]
    Butt,
    /// A half-disc of the stroke's half-width beyond the endpoint.
    Round,
    /// A square of the stroke's half-width beyond the endpoint.
    Square,
}

/// How two joined segments meet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum LineJoin {
    /// Extends the outer edges to their intersection, giving up to `Bevel`
    /// once the intersection is further out than `miter_limit` half-widths.
    #[default]
    Miter,
    /// A circular wedge of the stroke's half-width.
    Round,
    /// A flat cut across the corner.
    Bevel,
}

/// A stroked outline: width, ends, corners, colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    /// Total width of the stroke, in logical pixels.
    pub width: f64,
    pub cap: LineCap,
    pub join: LineJoin,
    /// A miter is dropped for a bevel once the point sits further than this
    /// many half-widths from the corner.
    pub miter_limit: f64,
    pub color: Color,
}

impl Stroke {
    /// A stroke of `width` with butt caps and miter joins — the defaults every
    /// 2D API starts from.
    pub fn new(width: f64, color: Color) -> Self {
        Self {
            width,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 4.0,
            color,
        }
    }

    pub fn with_cap(mut self, cap: LineCap) -> Self {
        self.cap = cap;
        self
    }

    pub fn with_join(mut self, join: LineJoin) -> Self {
        self.join = join;
        self
    }

    pub fn with_miter_limit(mut self, miter_limit: f64) -> Self {
        self.miter_limit = miter_limit;
        self
    }

    /// Half the stroke width — the distance the outline sits from the centre.
    pub fn half_width(&self) -> f64 {
        self.width.max(0.0) / 2.0
    }

    /// The outline of `path` stroked with this pen, as closed contours.
    ///
    /// Fill the result with the non-zero winding rule and it is exactly the
    /// stroked region. Every contour is normalised to positive orientation, so
    /// the pieces that overlap at joins and corners add up rather than
    /// cancelling.
    pub fn outline(&self, path: &Path) -> Vec<Polyline> {
        self.outline_with_tolerance(path, DEFAULT_FLATTEN_TOLERANCE)
    }

    /// [`Stroke::outline`] with an explicit curve tolerance.
    ///
    /// The tolerance is a caller concern for the same reason it is on fill: for
    /// a hairline stroke it wants to be a fraction of a pixel, and for a 40px
    /// band a quarter pixel is already far finer than the eye.
    pub fn outline_with_tolerance(&self, path: &Path, tolerance: f64) -> Vec<Polyline> {
        let half = self.half_width();
        if half <= 0.0 || path.is_empty() {
            return Vec::new();
        }
        let mut out: Vec<Polyline> = Vec::new();
        for subpath in path.flatten(tolerance) {
            let mut points = dedup(subpath.points);
            if points.is_empty() {
                continue;
            }
            if points.len() == 1 {
                // The "dot" idiom: a subpath that only moved. A round or square
                // cap still has a shape there; a butt cap has none, which is
                // why this is a per-cap decision and not a blanket disc.
                dot_contour(points[0], half, self.cap, &mut out);
                continue;
            }
            // Flattening a `Close` repeats the subpath's first point at the end
            // so a filled outline is explicitly sealed. For stroking that
            // repeat is a spurious zero-length edge — and, worse, a vertex
            // whose "next" is itself, which has no direction to join along.
            if subpath.closed && points.len() >= 2 && points.first() == points.last() {
                points.pop();
            }
            if points.len() < 2 {
                dot_contour(points[0], half, self.cap, &mut out);
                continue;
            }
            let mut edges: Vec<(Point, Point)> = points.windows(2).map(|w| (w[0], w[1])).collect();
            if subpath.closed {
                edges.push((points[points.len() - 1], points[0]));
            }
            for &(a, b) in &edges {
                let n = offset(a, b, half);
                push_contour(
                    &mut out,
                    &[
                        Point::new(a.x + n.x, a.y + n.y),
                        Point::new(b.x + n.x, b.y + n.y),
                        Point::new(b.x - n.x, b.y - n.y),
                        Point::new(a.x - n.x, a.y - n.y),
                    ],
                );
            }
            let vertex_count = points.len();
            if subpath.closed {
                for i in 0..vertex_count {
                    let prev = points[(i + vertex_count - 1) % vertex_count];
                    let here = points[i];
                    let next = points[(i + 1) % vertex_count];
                    join_contour(
                        prev,
                        here,
                        next,
                        half,
                        self.join,
                        self.miter_limit,
                        &mut out,
                    );
                }
            } else {
                for i in 1..vertex_count - 1 {
                    join_contour(
                        points[i - 1],
                        points[i],
                        points[i + 1],
                        half,
                        self.join,
                        self.miter_limit,
                        &mut out,
                    );
                }
                let start = points[0];
                let after = points[1];
                cap_contour(start, direction(after, start), half, self.cap, &mut out);
                let end = points[vertex_count - 1];
                let before = points[vertex_count - 2];
                cap_contour(end, direction(before, end), half, self.cap, &mut out);
            }
        }
        out
    }
}

/// Drops consecutive duplicate points — a repeated point has no direction, and
/// every normal used below is derived from one.
fn dedup(points: Vec<Point>) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::with_capacity(points.len());
    for p in points {
        if out.last().is_none_or(|last| *last != p) {
            out.push(p);
        }
    }
    out
}

/// Unit vector pointing from `a` towards `b`; `(1, 0)` when they coincide, so
/// a degenerate edge produces a defined (if arbitrary) normal instead of NaN.
fn direction(a: Point, b: Point) -> Point {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = (dx * dx + dy * dy).sqrt();
    if len <= f64::EPSILON {
        Point::new(1.0, 0.0)
    } else {
        Point::new(dx / len, dy / len)
    }
}

/// The left normal of `a`→`b`, scaled by `amount`.
fn offset(a: Point, b: Point, amount: f64) -> Point {
    let d = direction(a, b);
    Point::new(-d.y * amount, d.x * amount)
}

/// Adds a polygon, reversing it when needed so its signed area is positive.
///
/// Non-zero winding counts signed crossings: two positive contours overlap to a
/// winding of 2, still "inside", while one positive and one negative would
/// cancel to 0 and punch a hole in the middle of a join. Normalising every
/// contour to the same orientation is what makes the union come out of a
/// winding rule that has no union operation.
fn push_contour(out: &mut Vec<Polyline>, points: &[Point]) {
    if points.len() < 3 {
        return;
    }
    let mut points = points.to_vec();
    if signed_area(&points) < 0.0 {
        points.reverse();
    }
    // A zero-area contour (a collapsed wedge) contributes nothing but costs a
    // scanline crossing at every row it spans.
    if signed_area(&points) == 0.0 {
        return;
    }
    out.push(Polyline::new(points, true));
}

fn signed_area(points: &[Point]) -> f64 {
    let mut sum = 0.0;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        sum += a.x * b.y - b.x * a.y;
    }
    sum / 2.0
}

/// The cap at one end of an open subpath. `away` points out of the subpath.
fn cap_contour(p: Point, away: Point, half: f64, cap: LineCap, out: &mut Vec<Polyline>) {
    match cap {
        LineCap::Butt => {}
        LineCap::Square => {
            let n = Point::new(-away.y * half, away.x * half);
            let tip = Point::new(p.x + away.x * half, p.y + away.y * half);
            push_contour(
                out,
                &[
                    Point::new(p.x + n.x, p.y + n.y),
                    Point::new(tip.x + n.x, tip.y + n.y),
                    Point::new(tip.x - n.x, tip.y - n.y),
                    Point::new(p.x - n.x, p.y - n.y),
                ],
            );
        }
        LineCap::Round => {
            // A fan from the endpoint, spanning the half-turn that faces out
            // of the subpath.
            let base = away.y.atan2(away.x);
            let steps = ARC_STEPS;
            let mut wedge = Vec::with_capacity(steps + 2);
            wedge.push(p);
            for i in 0..=steps {
                let angle = base - std::f64::consts::FRAC_PI_2
                    + std::f64::consts::PI * (i as f64) / (steps as f64);
                wedge.push(Point::new(
                    p.x + half * angle.cos(),
                    p.y + half * angle.sin(),
                ));
            }
            push_contour(out, &wedge);
        }
    }
}

/// The shape at a subpath that only ever moved.
fn dot_contour(p: Point, half: f64, cap: LineCap, out: &mut Vec<Polyline>) {
    match cap {
        LineCap::Butt => {}
        LineCap::Square => cap_contour(p, Point::new(1.0, 0.0), half, LineCap::Square, out),
        LineCap::Round => {
            let steps = ARC_STEPS * 2;
            let mut disc = Vec::with_capacity(steps);
            for i in 0..steps {
                let angle = std::f64::consts::TAU * (i as f64) / (steps as f64);
                disc.push(Point::new(
                    p.x + half * angle.cos(),
                    p.y + half * angle.sin(),
                ));
            }
            push_contour(out, &disc);
        }
    }
}

/// Segments used to approximate a half-turn of a cap or join. Twelve keeps the
/// chord error of a 2.5px join under a fiftieth of a pixel.
const ARC_STEPS: usize = 12;

/// The wedge that fills the gap between two joined segments.
fn join_contour(
    prev: Point,
    here: Point,
    next: Point,
    half: f64,
    join: LineJoin,
    miter_limit: f64,
    out: &mut Vec<Polyline>,
) {
    let d0 = direction(prev, here);
    let d1 = direction(here, next);
    let cross = d0.x * d1.y - d0.y * d1.x;
    // Parallel (straight through) or reversed (a spike back on itself): the
    // segment quads already cover the first, and the second has no outer side
    // to fill.
    if cross.abs() <= 1e-12 {
        return;
    }
    // The outer side of the turn. On a left turn (positive cross in y-down
    // coordinates) the outside is to the right, i.e. the negative left normal.
    let side = if cross > 0.0 { -1.0 } else { 1.0 };
    let n0 = Point::new(-d0.y * side * half, d0.x * side * half);
    let n1 = Point::new(-d1.y * side * half, d1.x * side * half);
    let o0 = Point::new(here.x + n0.x, here.y + n0.y);
    let o1 = Point::new(here.x + n1.x, here.y + n1.y);

    match join {
        LineJoin::Bevel => push_contour(out, &[here, o0, o1]),
        LineJoin::Round => {
            let a0 = (o0.y - here.y).atan2(o0.x - here.x);
            let a1 = (o1.y - here.y).atan2(o1.x - here.x);
            let mut delta = a1 - a0;
            while delta > std::f64::consts::PI {
                delta -= std::f64::consts::TAU;
            }
            while delta < -std::f64::consts::PI {
                delta += std::f64::consts::TAU;
            }
            let steps = ((delta.abs() / std::f64::consts::PI) * ARC_STEPS as f64).ceil() as usize;
            let steps = steps.max(1);
            let mut wedge = Vec::with_capacity(steps + 2);
            wedge.push(here);
            for i in 0..=steps {
                let angle = a0 + delta * (i as f64) / (steps as f64);
                wedge.push(Point::new(
                    here.x + half * angle.cos(),
                    here.y + half * angle.sin(),
                ));
            }
            push_contour(out, &wedge);
        }
        LineJoin::Miter => {
            // Where the two outer edges meet. Line 1 passes through `o0`
            // along `d0`; line 2 through `o1` along `d1`. Solving
            // `o0 + t*d0 = o1 + s*d1` and crossing both sides with `d1`
            // eliminates `s` and leaves one division.
            let denom = d0.x * d1.y - d0.y * d1.x;
            if denom.abs() <= 1e-12 {
                push_contour(out, &[here, o0, o1]);
                return;
            }
            let (ex, ey) = (o1.x - o0.x, o1.y - o0.y);
            let t = (ex * d1.y - ey * d1.x) / denom;
            let miter = Point::new(o0.x + d0.x * t, o0.y + d0.y * t);
            let reach = ((miter.x - here.x).powi(2) + (miter.y - here.y).powi(2)).sqrt()
                / half.max(f64::EPSILON);
            if reach > miter_limit.max(1.0) {
                // Too long a spike: the join degrades to a bevel rather than
                // shooting a needle across the scene.
                push_contour(out, &[here, o0, o1]);
            } else {
                push_contour(out, &[here, o0, miter, o1]);
            }
        }
    }
}

/// What an element asks its subtree to be clipped to.
///
/// In the **element's own coordinates**: `Box` means the element's box, and a
/// path is measured from the element's top-left corner. The host resolves it
/// against laid-out bounds, which is why this cannot simply be a
/// [`ClipRegion`] — a widget does not know its own box when it describes
/// itself.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ClipShape {
    /// The element's own box, with corners rounded by `radius`.
    #[default]
    Box,
    /// The element's own box with rounded corners.
    RoundedBox { radius: f64 },
    /// An element-local path.
    Path(Arc<Path>),
}

/// A clip region in absolute logical coordinates.
///
/// The resolved form of [`ClipShape`]: the host intersects nothing, it just
/// hands each element's own region to the renderer, and the renderer keeps the
/// stack. That keeps the geometry exact — the intersection of two rounded
/// rectangles is not a rounded rectangle, so a host that folded the chain into
/// one shape would have to approximate what a stack gets right for free.
#[derive(Debug, Clone, PartialEq)]
pub enum ClipRegion {
    /// Axis-aligned rectangle.
    Rect(Rect),
    /// Rectangle with rounded corners.
    RoundedRect { bounds: Rect, radius: f64 },
    /// An arbitrary path. `origin` is where the path's own coordinates begin,
    /// so a shared path can be placed without being rebuilt.
    Path {
        bounds: Rect,
        origin: Point,
        path: Arc<Path>,
    },
}

impl ClipRegion {
    /// A rectangle, the cheapest clip and the only one with a scissor fast
    /// path in every backend.
    pub fn rect(bounds: Rect) -> Self {
        Self::Rect(bounds)
    }

    /// The region's bounding box. Always an upper bound on what the region
    /// admits, which is what culling and damage need.
    pub fn bounds(&self) -> Rect {
        match self {
            Self::Rect(bounds) | Self::RoundedRect { bounds, .. } => *bounds,
            Self::Path { bounds, .. } => *bounds,
        }
    }

    /// The region moved by `(dx, dy)`.
    ///
    /// A clip travels with the content it clips. An overlay that slides during
    /// its entry animation must carry its clip along, or the content would
    /// slide out from under a window that stayed behind.
    ///
    /// A path region keeps its path and moves the `origin` the path is placed
    /// at: the path is element-local and may be shared between frames, so
    /// rebuilding it per translation would allocate for no change in geometry.
    pub fn translate(&self, dx: f64, dy: f64) -> Self {
        let shift = |r: Rect| Rect {
            x0: r.x0 + dx,
            y0: r.y0 + dy,
            x1: r.x1 + dx,
            y1: r.y1 + dy,
        };
        match self {
            Self::Rect(bounds) => Self::Rect(shift(*bounds)),
            Self::RoundedRect { bounds, radius } => Self::RoundedRect {
                bounds: shift(*bounds),
                radius: *radius,
            },
            Self::Path {
                bounds,
                origin,
                path,
            } => Self::Path {
                bounds: shift(*bounds),
                origin: Point::new(origin.x + dx, origin.y + dy),
                path: Arc::clone(path),
            },
        }
    }

    /// The region mapped through the affine `t`.
    ///
    /// Rotation and scale change a rect clip into something that is no longer
    /// an axis-aligned rect, and this type has no rotated-rect variant — so
    /// the honest answer is the transform's *bounding box*: a clip that admits
    /// everything the mapped region would, possibly a little more. A clip that
    /// under-covers would cut off content the scene says is visible, which is
    /// the worse direction to be wrong in. A path region maps exactly (an
    /// affine takes Bézier control points to Bézier control points).
    pub fn map(&self, t: Transform) -> Self {
        if t.is_identity() {
            return self.clone();
        }
        match self {
            Self::Rect(bounds) => Self::Rect(t.map_rect(*bounds)),
            Self::RoundedRect { bounds, radius } => Self::RoundedRect {
                bounds: t.map_rect(*bounds),
                radius: radius * t.a.abs().max(t.d.abs()),
            },
            Self::Path {
                bounds,
                origin,
                path,
            } => {
                // The path's points are offsets from `origin`, so only the
                // linear part maps them; the translation lands in `origin`.
                let linear = Transform {
                    e: 0.0,
                    f: 0.0,
                    ..t
                };
                Self::Path {
                    bounds: t.map_rect(*bounds),
                    origin: t.map_point(*origin),
                    path: Arc::new(path.transform(linear)),
                }
            }
        }
    }

    /// True when the region admits a pixel centre, evaluated exactly.
    ///
    /// Only used by backends that clip per pixel; a backend with a scissor or
    /// a native clip shader never calls this.
    pub fn contains(&self, p: Point) -> bool {
        match self {
            Self::Rect(bounds) => bounds.contains(p),
            Self::RoundedRect { bounds, radius } => {
                let radius = radius.clamp(0.0, bounds.width().min(bounds.height()) / 2.0);
                let cx = (bounds.x0 + bounds.x1) / 2.0;
                let cy = (bounds.y0 + bounds.y1) / 2.0;
                let qx = (p.x - cx).abs() - (bounds.width() / 2.0 - radius);
                let qy = (p.y - cy).abs() - (bounds.height() / 2.0 - radius);
                let d = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius;
                d <= 0.0
            }
            // A path region defers to its bounding box here. Backends that can
            // only test a point use this; the ones that matter (the software
            // and vello rasterisers) build a coverage mask or a native clip
            // from the path itself.
            Self::Path { bounds, .. } => bounds.contains(p),
        }
    }
}

/// Resolves an element's declared clip into absolute coordinates.
///
/// `bounds` is the element's laid-out box; a `Box` shape uses it verbatim and a
/// path is placed at its origin.
pub fn resolve_clip(shape: &ClipShape, bounds: Rect) -> Option<ClipRegion> {
    if bounds.is_empty() {
        return None;
    }
    Some(match shape {
        ClipShape::Box => ClipRegion::Rect(bounds),
        ClipShape::RoundedBox { radius } => {
            if *radius <= 0.0 {
                ClipRegion::Rect(bounds)
            } else {
                ClipRegion::RoundedRect {
                    bounds,
                    radius: *radius,
                }
            }
        }
        ClipShape::Path(path) => ClipRegion::Path {
            bounds,
            origin: bounds.origin(),
            path: Arc::clone(path),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Size;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
        Rect { x0, y0, x1, y1 }
    }

    fn min_distance_from(points: &[Polyline], p: Point) -> f64 {
        points
            .iter()
            .flat_map(|poly| poly.points.iter())
            .map(|q| ((q.x - p.x).powi(2) + (q.y - p.y).powi(2)).sqrt())
            .fold(f64::INFINITY, f64::min)
    }

    /// A reference non-zero winding test, used to ask geometric questions about
    /// an outline ("does the stroked region cover this point?") without
    /// rasterising it.
    ///
    /// Deliberately a second implementation of the fill rule rather than a call
    /// into a renderer: these tests are about the *outline geometry*, so they
    /// need an oracle that is not the thing under test. It is the standard
    /// crossing count — an upward crossing of the ray to the right adds one, a
    /// downward one subtracts one — and it is only ever used on the closed
    /// contours `Stroke::outline` is documented to return.
    fn covered(contours: &[Polyline], p: Point) -> bool {
        let mut winding = 0i32;
        for contour in contours {
            let pts = &contour.points;
            for i in 0..pts.len() {
                let a = pts[i];
                let b = pts[(i + 1) % pts.len()];
                let side = (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y);
                if a.y <= p.y {
                    if b.y > p.y && side > 0.0 {
                        winding += 1;
                    }
                } else if b.y <= p.y && side < 0.0 {
                    winding -= 1;
                }
            }
        }
        winding != 0
    }

    // -- flattening -------------------------------------------------------

    #[test]
    fn flattening_a_line_subpath_keeps_its_ends() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(10.0, 0.0))
            .line_to(Point::new(10.0, 5.0));
        let flat = path.flatten(DEFAULT_FLATTEN_TOLERANCE);
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].points.first(), Some(&Point::new(0.0, 0.0)));
        assert_eq!(flat[0].points.last(), Some(&Point::new(10.0, 5.0)));
        assert_eq!(flat[0].points.len(), 3, "lines need no subdivision");
    }

    /// A cubic that is actually a straight line must not be subdivided: the
    /// flatness test measures distance to the chord, and control points on it
    /// mean zero.
    #[test]
    fn a_collinear_cubic_flattens_to_its_endpoint() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0)).cubic_to(
            Point::new(3.0, 0.0),
            Point::new(7.0, 0.0),
            Point::new(10.0, 0.0),
        );
        let flat = path.flatten(DEFAULT_FLATTEN_TOLERANCE);
        assert_eq!(
            flat[0].points,
            vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0)]
        );
    }

    /// The flattened polyline must stay within the tolerance of the true curve.
    /// Sampled against the Bernstein form, which is a second implementation on
    /// purpose: the point is to check the subdivision against the curve it
    /// claims to approximate, not against itself.
    #[test]
    fn flattened_cubics_stay_within_tolerance() {
        let (p0, c1, c2, p1) = (
            Point::new(0.0, 0.0),
            Point::new(0.0, 40.0),
            Point::new(60.0, 40.0),
            Point::new(60.0, 0.0),
        );
        let mut path = Path::new();
        path.move_to(p0).cubic_to(c1, c2, p1);
        let tolerance = 0.1;
        let flat = path.flatten(tolerance);
        let points = &flat[0].points;

        let bezier = |t: f64| {
            let u = 1.0 - t;
            Point::new(
                u * u * u * p0.x
                    + 3.0 * u * u * t * c1.x
                    + 3.0 * u * t * t * c2.x
                    + t * t * t * p1.x,
                u * u * u * p0.y
                    + 3.0 * u * u * t * c1.y
                    + 3.0 * u * t * t * c2.y
                    + t * t * t * p1.y,
            )
        };
        let mut worst: f64 = 0.0;
        for i in 0..=400 {
            let sample = bezier(i as f64 / 400.0);
            let nearest = points
                .windows(2)
                .map(|w| point_line_distance(sample, w[0], w[1]))
                .fold(f64::INFINITY, f64::min);
            worst = worst.max(nearest);
        }
        assert!(
            worst <= tolerance,
            "flattened curve strayed {worst} from the true curve (tolerance {tolerance})"
        );
    }

    #[test]
    fn close_marks_the_subpath_and_rejoins_its_start() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(4.0, 0.0))
            .line_to(Point::new(4.0, 4.0))
            .close();
        let flat = path.flatten(DEFAULT_FLATTEN_TOLERANCE);
        assert!(flat[0].closed);
        assert_eq!(flat[0].points.first(), flat[0].points.last());
    }

    // -- constructors and bounds ------------------------------------------

    #[test]
    fn a_circle_is_bounded_by_its_radius() {
        let circle = Path::circle(Point::new(10.0, 10.0), 5.0);
        let bounds = circle.bounds();
        assert!((bounds.x0 - 5.0).abs() < 1e-9, "{bounds:?}");
        assert!((bounds.y0 - 5.0).abs() < 1e-9, "{bounds:?}");
        assert!((bounds.x1 - 15.0).abs() < 1e-9, "{bounds:?}");
        assert!((bounds.y1 - 15.0).abs() < 1e-9, "{bounds:?}");
    }

    /// A quarter-turn cubic with the standard constant overshoots its circle by
    /// about 0.027% — invisible, but a test that asserted exactness would be
    /// asserting a falsehood about the construction. Sample the flattened curve
    /// instead and check every point really is on the circle.
    #[test]
    fn a_circle_points_lie_on_the_circle() {
        let circle = Path::circle(Point::ZERO, 10.0);
        let flat = circle.flatten(0.01);
        for p in &flat[0].points {
            let r = (p.x * p.x + p.y * p.y).sqrt();
            assert!((r - 10.0).abs() < 0.01, "point {p:?} at radius {r}");
        }
    }

    /// A pill, not a rounded rectangle with an absurd radius. The control group
    /// is the square-cornered rect beside it: its nearest point to the corner
    /// *is* the corner, so "the corner got cut off" is a claim that can fail.
    #[test]
    fn a_rounded_rect_clamps_its_radius_and_cuts_its_corner() {
        let bounds = rect(0.0, 0.0, 20.0, 10.0);
        let out = Path::rounded_rect(bounds, 50.0).flatten(0.01);
        // A clamped radius of 5 puts the arc's closest approach at
        // `5 - 5/sqrt(2)` ≈ 1.46 from the corner.
        let nearest = min_distance_from(&out, Point::new(0.0, 0.0));
        assert!(
            (nearest - 5.0 * (2.0f64.sqrt() - 1.0)).abs() < 0.05,
            "nearest point to the corner is {nearest}"
        );
        // Nothing may leave the box, and the corners really are gone.
        for poly in &out {
            for p in &poly.points {
                assert!(p.x >= -1e-9 && p.x <= 20.0 + 1e-9, "{p:?}");
                assert!(p.y >= -1e-9 && p.y <= 10.0 + 1e-9, "{p:?}");
            }
        }

        let square = Path::rect(bounds).flatten(0.01);
        assert_eq!(
            min_distance_from(&square, Point::new(0.0, 0.0)),
            0.0,
            "control: an unrounded rect does reach its corner"
        );
    }

    #[test]
    fn an_arc_walks_the_sweep_it_was_asked_for() {
        let arc = Path::ellipse_arc(Point::ZERO, 10.0, 10.0, 0.0, std::f64::consts::PI);
        let flat = arc.flatten(0.01);
        let first = flat[0].points.first().copied().unwrap();
        assert!(
            (first.x - 10.0).abs() < 0.01 && first.y.abs() < 0.01,
            "{first:?}"
        );
        // Half a turn lands on the opposite point.
        let last_drawn = flat[0].points[flat[0].points.len() - 2];
        assert!(
            (last_drawn.x + 10.0).abs() < 0.01 && last_drawn.y.abs() < 0.01,
            "{last_drawn:?}"
        );
    }

    #[test]
    fn an_empty_path_has_zero_bounds_and_no_contours() {
        let path = Path::new();
        assert!(path.is_empty());
        assert_eq!(path.bounds(), Rect::ZERO);
        assert!(path.flatten(DEFAULT_FLATTEN_TOLERANCE).is_empty());
    }

    // -- stroking ----------------------------------------------------------

    fn segment() -> Path {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(20.0, 0.0));
        path
    }

    fn corner() -> Path {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(20.0, 0.0))
            .line_to(Point::new(20.0, 20.0));
        path
    }

    /// The core claim: the outline, filled non-zero, *is* the stroked region.
    /// Every assertion below is phrased as "this point is / is not covered",
    /// which is the property the renderers depend on.
    #[test]
    fn a_butt_cap_covers_up_to_the_endpoint_and_no_further() {
        let out = Stroke::new(4.0, Color::BLACK).outline(&segment());
        assert_eq!(out.len(), 1, "one quad, no cap shapes: {out:?}");
        assert!(covered(&out, Point::new(19.9, 0.0)));
        assert!(covered(&out, Point::new(10.0, 1.9)));
        assert!(!covered(&out, Point::new(10.0, 2.1)));
        assert!(
            !covered(&out, Point::new(20.1, 0.0)),
            "butt stops at the end"
        );
    }

    /// A square cap's *corner* is what tells it apart from a round one: both
    /// reach the same x, only the square fills the box corner.
    #[test]
    fn a_square_cap_fills_the_box_corner_a_round_cap_leaves_empty() {
        let half = 2.0;
        // A probe inside the cap's own half-width in both directions: a disc
        // of radius `half` reaches it, so it says nothing about shape.
        let probe = Point::new(20.0 + half * 0.3, half * 0.5);
        let square = Stroke::new(4.0, Color::BLACK)
            .with_cap(LineCap::Square)
            .outline(&segment());
        let round = Stroke::new(4.0, Color::BLACK)
            .with_cap(LineCap::Round)
            .outline(&segment());
        let corner = Point::new(20.0 + half * 0.9, half * 0.9);
        assert!(covered(&square, corner), "a square cap has corners");
        assert!(
            !covered(&round, corner),
            "a round cap is a half-disc: its corner is outside"
        );
        assert!(covered(&square, probe) && covered(&round, probe));
        assert!(covered(&square, Point::new(20.0 + half * 0.99, 0.0)));
        assert!(covered(&round, Point::new(20.0 + half * 0.99, 0.0)));
        assert!(!covered(&round, Point::new(20.0 + half * 1.01, 0.0)));
    }

    /// The three joins differ in exactly one place: how far out along the
    /// corner bisector the region reaches. On a right angle a bevel stops at
    /// the half-width, a miter at `half*sqrt(2)`, and a round join between.
    #[test]
    fn joins_reach_further_out_in_the_order_bevel_round_miter() {
        let half = 3.0;
        let at = |join: LineJoin| {
            let pen = Stroke::new(half * 2.0, Color::BLACK).with_join(join);
            pen.outline(&corner())
        };
        // Probe on the outward diagonal of the (20, 0) corner.
        let d = std::f64::consts::FRAC_1_SQRT_2;
        let probe = |factor: f64| Point::new(20.0 + d * half * factor, -d * half * factor);

        let bevel = at(LineJoin::Bevel);
        let round = at(LineJoin::Round);
        let miter = at(LineJoin::Miter);

        // All three fill the corner itself.
        for (name, out) in [("bevel", &bevel), ("round", &round), ("miter", &miter)] {
            assert!(
                covered(out, Point::new(19.5, -0.5)),
                "{name} misses the corner"
            );
        }
        // Just past the half-width only the miter is still there.
        assert!(
            !covered(&bevel, probe(1.05)),
            "bevel reaches past the half-width"
        );
        assert!(
            !covered(&round, probe(1.05)),
            "round reaches past the half-width"
        );
        assert!(covered(&miter, probe(1.05)), "miter should fill the notch");
        // And the miter stops before the right angle's exact tip.
        assert!(covered(&miter, probe(1.40)));
        assert!(!covered(&miter, probe(1.43)), "no miter past half*sqrt(2)");
    }

    /// The miter limit is not decoration: past it the join has to give way, or
    /// a hairpin turn paints a needle across the whole scene.
    #[test]
    fn a_miter_past_the_limit_falls_back_to_a_bevel() {
        let half = 3.0;
        let d = std::f64::consts::FRAC_1_SQRT_2;
        let probe = Point::new(20.0 + d * half * 1.2, -d * half * 1.2);
        let pen = |limit: f64| {
            Stroke::new(half * 2.0, Color::BLACK)
                .with_join(LineJoin::Miter)
                .with_miter_limit(limit)
        };
        assert!(covered(&pen(10.0).outline(&corner()), probe));
        assert!(
            !covered(&pen(1.0).outline(&corner()), probe),
            "a miter past its limit must bevel"
        );
    }

    /// A subpath that only moved is a dot, and only a cap that has a shape
    /// draws one.
    #[test]
    fn a_moved_only_subpath_strokes_as_a_dot_only_with_a_shaped_cap() {
        let dot = || {
            let mut path = Path::new();
            path.move_to(Point::new(5.0, 5.0));
            path
        };
        let butt = Stroke::new(4.0, Color::BLACK).outline(&dot());
        assert!(butt.is_empty(), "a butt cap has no shape at a bare point");
        let round = Stroke::new(4.0, Color::BLACK)
            .with_cap(LineCap::Round)
            .outline(&dot());
        assert_eq!(round.len(), 1);
        assert!(covered(&round, Point::new(5.0, 5.0)));
        assert!(covered(&round, Point::new(6.9, 5.0)));
        assert!(!covered(&round, Point::new(7.1, 5.0)));
        let square = Stroke::new(4.0, Color::BLACK)
            .with_cap(LineCap::Square)
            .outline(&dot());
        assert_eq!(square.len(), 1);
        assert!(
            covered(&square, Point::new(6.9, 6.9)),
            "square corners exist"
        );
        assert!(!covered(&round, Point::new(6.9, 6.9)), "the disc's do not");
    }

    /// A closed subpath gets joints, not caps. The contour count is what says
    /// so directly — four sides plus four joins, where emitting caps as well
    /// would make it ten — and the probe pins the corner treatment to a *round
    /// join* (reach = the half-width) rather than a miter (reach =
    /// `half*sqrt(2)`), which is the other way this could go wrong silently.
    #[test]
    fn a_closed_subpath_gets_joints_instead_of_caps() {
        let half = 1.0;
        let square = Path::rect(rect(0.0, 0.0, 10.0, 10.0));
        let pen = Stroke::new(half * 2.0, Color::BLACK)
            .with_cap(LineCap::Round)
            .with_join(LineJoin::Round);
        let out = pen.outline(&square);
        assert_eq!(out.len(), 8, "{} contours", out.len());
        // The band hugs the outline on an edge…
        assert!(covered(&out, Point::new(5.0, -0.9)));
        assert!(!covered(&out, Point::new(5.0, -1.1)));
        // …and at the corner reaches only the half-width outward, where a
        // miter would have filled out to `half*sqrt(2)`.
        let d = std::f64::consts::FRAC_1_SQRT_2;
        assert!(covered(
            &out,
            Point::new(-d * half * 0.95, -d * half * 0.95)
        ));
        assert!(!covered(&out, Point::new(-d * half * 1.2, -d * half * 1.2)));
    }

    /// Every contour must wind the same way, or non-zero filling would punch
    /// holes where the pieces overlap.
    #[test]
    fn stroke_outlines_are_orientated_consistently() {
        let pen = Stroke::new(6.0, Color::BLACK)
            .with_cap(LineCap::Round)
            .with_join(LineJoin::Round);
        for contour in pen.outline(&corner()) {
            assert!(
                signed_area(&contour.points) > 0.0,
                "contour wound the wrong way: {:?}",
                contour.points
            );
        }
    }

    #[test]
    fn a_zero_width_stroke_has_no_outline() {
        let pen = Stroke::new(0.0, Color::BLACK);
        assert!(pen.outline(&segment()).is_empty());
    }

    // -- clip regions ------------------------------------------------------

    #[test]
    fn a_box_clip_resolves_to_the_elements_own_bounds() {
        let bounds = rect(4.0, 6.0, 24.0, 26.0);
        let resolved = resolve_clip(&ClipShape::Box, bounds).unwrap();
        assert_eq!(resolved, ClipRegion::Rect(bounds));
    }

    #[test]
    fn a_zero_radius_rounded_clip_degrades_to_a_rect() {
        let bounds = rect(0.0, 0.0, 10.0, 10.0);
        let resolved = resolve_clip(&ClipShape::RoundedBox { radius: 0.0 }, bounds).unwrap();
        assert_eq!(resolved, ClipRegion::Rect(bounds));
    }

    #[test]
    fn a_rounded_clip_cuts_its_corners_but_not_its_edges() {
        let bounds = rect(0.0, 0.0, 20.0, 20.0);
        let resolved = resolve_clip(&ClipShape::RoundedBox { radius: 5.0 }, bounds).unwrap();
        assert!(
            !resolved.contains(Point::new(0.4, 0.4)),
            "corner is clipped"
        );
        assert!(
            resolved.contains(Point::new(10.0, 0.5)),
            "edge midpoint is not"
        );
        assert!(resolved.contains(Point::new(10.0, 10.0)));
        assert!(!resolved.contains(Point::new(-1.0, 10.0)));
        assert_eq!(resolved.bounds(), bounds);
    }

    #[test]
    fn a_path_clip_is_placed_at_the_elements_origin() {
        let bounds = rect(10.0, 10.0, 30.0, 30.0);
        let shape = ClipShape::Path(Arc::new(Path::circle(Point::new(10.0, 10.0), 10.0)));
        let resolved = resolve_clip(&shape, bounds).unwrap();
        match resolved {
            ClipRegion::Path {
                bounds: b, origin, ..
            } => {
                assert_eq!(origin, Point::new(10.0, 10.0));
                assert_eq!(b, bounds);
            }
            other => panic!("expected a path region, got {other:?}"),
        }
    }

    #[test]
    fn an_empty_box_resolves_to_no_clip() {
        assert!(resolve_clip(&ClipShape::Box, Rect::ZERO).is_none());
    }

    // -- translation ------------------------------------------------------

    #[test]
    fn translating_a_path_moves_control_points_and_not_just_the_endpoints() {
        // A cubic whose control points are the only thing that makes it bulge:
        // a translate that moved endpoints alone would leave the bulge behind,
        // and the flattened points would no longer be a shifted copy.
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 10.0)).cubic_to(
            Point::new(4.0, -6.0),
            Point::new(16.0, 26.0),
            Point::new(20.0, 10.0),
        );
        let moved = path.translate(7.0, -3.0);
        let before = path.flatten(DEFAULT_FLATTEN_TOLERANCE);
        let after = moved.flatten(DEFAULT_FLATTEN_TOLERANCE);
        assert_eq!(before.len(), after.len());
        for (a, b) in before[0].points.iter().zip(&after[0].points) {
            assert_eq!(*b, Point::new(a.x + 7.0, a.y - 3.0));
        }
        // Control: the shifted curve really is somewhere else.
        assert_ne!(after[0].points, before[0].points);
    }

    #[test]
    fn translating_a_path_moves_the_bounds_it_reports() {
        let path = Path::rect(rect(10.0, 20.0, 30.0, 40.0));
        assert_eq!(
            path.translate(5.0, -5.0).bounds(),
            rect(15.0, 15.0, 35.0, 35.0)
        );
    }

    #[test]
    fn a_translated_rect_clip_admits_the_shifted_window() {
        let clip = ClipRegion::Rect(rect(0.0, 0.0, 10.0, 10.0));
        let moved = clip.translate(20.0, 0.0);
        assert!(
            moved.contains(Point::new(25.0, 5.0)),
            "the shifted window is open"
        );
        assert!(!moved.contains(Point::new(5.0, 5.0)), "the old one is not");
        assert_eq!(moved.bounds(), rect(20.0, 0.0, 30.0, 10.0));
    }

    #[test]
    fn a_translated_path_clip_reuses_its_path_and_places_it_elsewhere() {
        let path = Arc::new(Path::circle(Point::new(0.0, 0.0), 5.0));
        let clip = ClipRegion::Path {
            bounds: rect(0.0, 0.0, 10.0, 10.0),
            origin: Point::new(0.0, 0.0),
            path: Arc::clone(&path),
        };
        let moved = clip.translate(3.0, 4.0);
        match moved {
            ClipRegion::Path {
                origin, path: p, ..
            } => {
                assert_eq!(origin, Point::new(3.0, 4.0));
                assert!(Arc::ptr_eq(&p, &path), "the placement moved, not the shape");
            }
            other => panic!("expected a path region, got {other:?}"),
        }
    }

    /// An affine maps a path exactly: rotation about a point carries every
    /// vertex where the matrix says, and the identity map is a no-op clone.
    #[test]
    fn path_transform_maps_points_and_identity_clones() {
        let mut path = Path::new();
        path.move_to(Point::new(1.0, 0.0))
            .line_to(Point::new(2.0, 0.0));
        let quarter = Transform::rotate_around(std::f64::consts::FRAC_PI_2, Point::new(0.0, 0.0));
        let mapped = path.transform(quarter);
        let pts: Vec<Point> = mapped
            .segments()
            .iter()
            .filter_map(|seg| match seg {
                PathSeg::MoveTo(p) | PathSeg::LineTo(p) => Some(*p),
                _ => None,
            })
            .collect();
        assert_eq!(pts.len(), 2);
        assert!((pts[0].x - 0.0).abs() < 1e-9 && (pts[0].y - 1.0).abs() < 1e-9);
        assert!((pts[1].x - 0.0).abs() < 1e-9 && (pts[1].y - 2.0).abs() < 1e-9);

        // Identity maps to the same geometry...
        let same = path.transform(Transform::IDENTITY);
        assert_eq!(same, path);
        // ...and scales reach every control point, quad included.
        let mut curve = Path::new();
        curve
            .move_to(Point::new(0.0, 0.0))
            .quad_to(Point::new(1.0, 0.0), Point::new(2.0, 0.0));
        let scaled = curve.transform(Transform::scale(3.0, 3.0));
        assert!(
            matches!(
                scaled.segments(),
                [
                    PathSeg::MoveTo(p0),
                    PathSeg::QuadTo { ctrl, to },
                ] if *p0 == Point::new(0.0, 0.0)
                    && *ctrl == Point::new(3.0, 0.0)
                    && *to == Point::new(6.0, 0.0)
            ),
            "the quad's control points scaled with it"
        );
    }

    /// A rect region mapped under a quarter turn becomes its rotated
    /// bounding box; a path region maps its points relative to its origin.
    #[test]
    fn clip_region_map_covers_the_mapped_area() {
        let rect = Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(4.0, 2.0));
        let quarter = Transform::rotate(std::f64::consts::FRAC_PI_2);
        let mapped = ClipRegion::Rect(rect).map(quarter);
        let b = mapped.bounds();
        // (x, y) → (−y, x): [0..4]×[0..2] → [−2..0]×[0..4].
        assert!((b.x0 - (-2.0)).abs() < 1e-9 && (b.x1 - 0.0).abs() < 1e-9);
        assert!((b.y0 - 0.0).abs() < 1e-9 && (b.y1 - 4.0).abs() < 1e-9);

        // Identity is a no-op.
        assert_eq!(
            ClipRegion::Rect(rect).map(Transform::IDENTITY),
            ClipRegion::Rect(rect)
        );

        // A path region keeps its points relative to a mapped origin.
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(1.0, 0.0));
        let segs = path.segments().to_vec();
        let region = ClipRegion::Path {
            bounds: Rect::from_origin_size(Point::new(10.0, 10.0), Size::new(1.0, 1.0)),
            origin: Point::new(10.0, 10.0),
            path: Arc::new(path),
        };
        let shift = Transform::translate(5.0, 7.0);
        match region.map(shift) {
            ClipRegion::Path {
                origin, path: p, ..
            } => {
                assert_eq!(origin, Point::new(15.0, 17.0));
                // The offsets are unchanged: the placement moved, not the shape.
                assert_eq!(p.segments(), segs.as_slice());
            }
            other => panic!("expected a path region, got {other:?}"),
        }
    }
}
