//! SVG path data -> [`Path`].
//!
//! 2.0 has no SVG pipeline. v1 rendered icon sets through `usvg` inside the
//! renderer (`Renderer::draw_svg`); here the renderer only knows how to fill a
//! [`Path`]. The gap is closed once, in this module, so every kit that ships
//! icons as `<path d="..">` data gets a working icon for the price of one
//! parse (see [`crate::vector_icon::VectorIcon::from_path_data`]).
//!
//! ## What is supported
//!
//! The whole path grammar: `M/m L/l H/h V/v C/c S/s Q/q T/t A/a Z/z`, implicit
//! repetition of a command's argument list (`L 1 2 3 4` is two segments), and
//! the `M`-then-implicit-`L` rule that trips up naive parsers. `S`/`T` reflect
//! the previous control point, and `A` is converted to cubics, because a
//! [`Path`] has lines and Béziers and nothing else.
//!
//! ## What it deliberately is not
//!
//! An SVG *document* parser. There is no `<svg>`, no `transform` attribute, no
//! `fill-rule` switch, no CSS. Icon sets are `<svg viewBox=..><path d=../></svg>`
//! and that is the whole input surface; anything else belongs in a real SVG
//! engine, not in a widget crate.
//!
//! ## Malformed input
//!
//! Parsing stops at the first thing it cannot read and returns the path built
//! so far. An icon set is static data compiled into the binary, so a truncated
//! path is a bug in the data, not a runtime condition to recover from — but
//! drawing half a glyph beats panicking inside `paint`.

use floom_types::{Path, PathSeg, Point};

/// Parse SVG path data into a [`Path`].
///
/// Coordinates land in whatever user space the data was written in; a caller
/// drawing into a box scales afterwards (see [`crate::vector_icon::VectorIcon`]).
///
/// ```
/// use floom_widgets::parse_path_data;
/// use floom_types::PathSeg;
///
/// let path = parse_path_data("M0 0 H10 V10 Z");
/// assert_eq!(
///     path.segments(),
///     &[
///         PathSeg::MoveTo(floom_types::Point::new(0.0, 0.0)),
///         PathSeg::LineTo(floom_types::Point::new(10.0, 0.0)),
///         PathSeg::LineTo(floom_types::Point::new(10.0, 10.0)),
///         PathSeg::Close,
///     ]
/// );
/// ```
pub fn parse_path_data(data: &str) -> Path {
    Parser::new(data).run()
}

/// Everything the parser carries between commands.
///
/// SVG's smooth-curve commands (`S`, `T`) and relative commands are all defined
/// in terms of "the previous command" and "the current point", so the state has
/// to survive across commands — but not across an explicit `MoveTo` or `Close`.
struct Parser<'a> {
    data: &'a str,
    /// Byte offset of the next unread character.
    pos: usize,
    path: Path,
    /// The current point, and where the current subpath started.
    current: Point,
    subpath_start: Point,
    /// Reflection point for `S`/`s`; `None` unless the previous command was a
    /// cubic.
    last_cubic_ctrl: Option<Point>,
    /// Reflection point for `T`/`t`; `None` unless the previous command was a
    /// quadratic.
    last_quad_ctrl: Option<Point>,
}

impl<'a> Parser<'a> {
    fn new(data: &'a str) -> Self {
        Self {
            data,
            pos: 0,
            path: Path::new(),
            current: Point::new(0.0, 0.0),
            subpath_start: Point::new(0.0, 0.0),
            last_cubic_ctrl: None,
            last_quad_ctrl: None,
        }
    }

    fn run(mut self) -> Path {
        // `None` until the first command letter; afterwards it is the command
        // the next argument run belongs to (SVG repeats the previous command's
        // argument list when the next token is a number, not a letter).
        let mut command: Option<char> = None;
        // The `M`/`m` special case: the *first* argument pair moves, and every
        // repeated one in the same list lines. `M x y x y` is a move followed by
        // a line, which is what the grammar means and what naive parsers get
        // wrong.
        let mut moveto_pending = false;
        loop {
            self.skip_separators();
            match self.peek() {
                Some(c) if is_command(c) => {
                    self.pos += c.len_utf8();
                    command = Some(c);
                    moveto_pending = true;
                }
                // Anything else (including the end of the data) either repeats
                // the previous command's argument list or ends parsing.
                _ => {}
            }
            let Some(cmd) = command else {
                break;
            };
            if !self.run_command(cmd, &mut moveto_pending) {
                break;
            }
            // `Z` takes no arguments, so there is nothing for the implicit
            // repetition rule to read: without this the loop would re-run it
            // forever. Every other command consumed its whole argument list
            // above, so returning `false` there already ended parsing.
            if matches!(base_of(cmd), 'Z') {
                command = None;
            }
        }
        self.path
    }

    /// Runs one command. Returns `false` when the data ran out mid-command, or
    /// hit something unreadable, which ends parsing.
    fn run_command(&mut self, cmd: char, moveto_pending: &mut bool) -> bool {
        let relative = cmd.is_ascii_lowercase();
        let base = base_of(cmd);

        match base {
            'M' => {
                let (Some(x), Some(y)) = (self.number(), self.number()) else {
                    return false;
                };
                let to = self.absolute_point(x, y, relative);
                if *moveto_pending {
                    *moveto_pending = false;
                    self.move_to(to);
                } else {
                    self.path.line_to(to);
                    self.current = to;
                    self.forget_controls();
                }
            }
            'L' => {
                let (Some(x), Some(y)) = (self.number(), self.number()) else {
                    return false;
                };
                let to = self.absolute_point(x, y, relative);
                self.path.line_to(to);
                self.current = to;
                self.forget_controls();
            }
            'H' => {
                let Some(x) = self.number() else {
                    return false;
                };
                let to = Point::new(
                    if relative { self.current.x + x } else { x },
                    self.current.y,
                );
                self.path.line_to(to);
                self.current = to;
                self.forget_controls();
            }
            'V' => {
                let Some(y) = self.number() else {
                    return false;
                };
                let to = Point::new(
                    self.current.x,
                    if relative { self.current.y + y } else { y },
                );
                self.path.line_to(to);
                self.current = to;
                self.forget_controls();
            }
            'C' => {
                let c1 = self.relative_point(relative);
                let c2 = self.relative_point(relative);
                let to = self.relative_point(relative);
                let (Some(c1), Some(c2), Some(to)) = (c1, c2, to) else {
                    return false;
                };
                self.path.cubic_to(c1, c2, to);
                self.current = to;
                self.last_cubic_ctrl = Some(c2);
                self.last_quad_ctrl = None;
            }
            'S' => {
                let c2 = self.relative_point(relative);
                let to = self.relative_point(relative);
                let (Some(c2), Some(to)) = (c2, to) else {
                    return false;
                };
                let c1 = reflect(self.current, self.last_cubic_ctrl);
                self.path.cubic_to(c1, c2, to);
                self.current = to;
                self.last_cubic_ctrl = Some(c2);
                self.last_quad_ctrl = None;
            }
            'Q' => {
                let ctrl = self.relative_point(relative);
                let to = self.relative_point(relative);
                let (Some(ctrl), Some(to)) = (ctrl, to) else {
                    return false;
                };
                self.path.quad_to(ctrl, to);
                self.current = to;
                self.last_quad_ctrl = Some(ctrl);
                self.last_cubic_ctrl = None;
            }
            'T' => {
                let to = self.relative_point(relative);
                let Some(to) = to else {
                    return false;
                };
                let ctrl = reflect(self.current, self.last_quad_ctrl);
                self.path.quad_to(ctrl, to);
                self.current = to;
                self.last_quad_ctrl = Some(ctrl);
                self.last_cubic_ctrl = None;
            }
            'A' => {
                let (Some(rx), Some(ry), Some(rotation)) =
                    (self.number(), self.number(), self.number())
                else {
                    return false;
                };
                // The two flags are single characters, not general numbers: SVG
                // allows `a1 1 0 0110 10` with no separator, so a number parse
                // here would eat the following coordinate.
                let (Some(large_arc), Some(sweep)) = (self.flag(), self.flag()) else {
                    return false;
                };
                let Some(to) = self.relative_point(relative) else {
                    return false;
                };
                arc_to_cubics(
                    &mut self.path,
                    self.current,
                    rx,
                    ry,
                    rotation,
                    large_arc,
                    sweep,
                    to,
                );
                self.current = to;
                self.forget_controls();
            }
            'Z' => {
                self.path.close();
                self.current = self.subpath_start;
                self.forget_controls();
            }
            _ => return false,
        }
        true
    }

    /// Starts a subpath. `M` also forgets the running control points, which is
    /// what makes `M … S …` treat `S`'s first control point as the current
    /// point rather than reflecting a curve from before the move.
    fn move_to(&mut self, to: Point) {
        self.path.move_to(to);
        self.current = to;
        self.subpath_start = to;
        self.forget_controls();
    }

    /// Drops the smooth-curve reflection state: only a directly preceding cubic
    /// (for `S`) or quadratic (for `T`) may be reflected.
    fn forget_controls(&mut self) {
        self.last_cubic_ctrl = None;
        self.last_quad_ctrl = None;
    }

    fn absolute_point(&self, x: f64, y: f64, relative: bool) -> Point {
        if relative {
            Point::new(self.current.x + x, self.current.y + y)
        } else {
            Point::new(x, y)
        }
    }

    /// One coordinate pair, resolved against the current point when relative.
    fn relative_point(&mut self, relative: bool) -> Option<Point> {
        let x = self.number()?;
        let y = self.number()?;
        Some(self.absolute_point(x, y, relative))
    }

    fn peek(&self) -> Option<char> {
        self.data[self.pos..].chars().next()
    }

    fn skip_separators(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() || c == ',' {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
    }

    /// One number in SVG's grammar: an optional sign, digits with an optional
    /// fraction, and an optional exponent. The *sign is also the separator*,
    /// which is why `.04-.3` is two numbers and not one malformed one.
    fn number(&mut self) -> Option<f64> {
        self.skip_separators();
        let bytes = self.data.as_bytes();
        let start = self.pos;
        let mut i = self.pos;
        if matches!(bytes.get(i), Some(b'+') | Some(b'-')) {
            i += 1;
        }
        let mut saw_digit = false;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            saw_digit = true;
        }
        if bytes.get(i) == Some(&b'.') {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
                saw_digit = true;
            }
        }
        if !saw_digit {
            self.pos = start;
            return None;
        }
        // An `e` only starts an exponent when digits actually follow it —
        // otherwise it is the next command letter (`1e` is nonsense, but a
        // parser that consumed the `e` would eat a valid command).
        if matches!(bytes.get(i), Some(b'e') | Some(b'E')) {
            let mut j = i + 1;
            if matches!(bytes.get(j), Some(b'+') | Some(b'-')) {
                j += 1;
            }
            if j < bytes.len() && bytes[j].is_ascii_digit() {
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                i = j;
            }
        }
        let text = &self.data[start..i];
        self.pos = i;
        text.parse::<f64>().ok()
    }

    /// An arc flag: `0` or `1`, written without a separator.
    fn flag(&mut self) -> Option<bool> {
        self.skip_separators();
        let c = self.peek()?;
        if c == '0' || c == '1' {
            self.pos += 1;
            Some(c == '1')
        } else {
            None
        }
    }
}

/// The control point a smooth curve reflects: the point mirrored through the
/// current point, or the current point itself when there is nothing to reflect.
fn reflect(current: Point, previous: Option<Point>) -> Point {
    match previous {
        Some(prev) => Point::new(2.0 * current.x - prev.x, 2.0 * current.y - prev.y),
        None => current,
    }
}

fn is_command(c: char) -> bool {
    c.is_ascii_alphabetic()
}

/// The uppercase command letter, i.e. the operation with the relative flag
/// stripped off.
fn base_of(cmd: char) -> char {
    cmd.to_ascii_uppercase()
}

/// Appends the cubics for one elliptical arc, in SVG's endpoint parameterisation
/// (`rx ry rotation large-arc sweep x y`).
///
/// The conversion is the one in the SVG specification's implementation notes:
/// recentre on the midpoint of the chord, rotate into the ellipse's frame, split
/// the sweep into <=90 degree pieces, and rotate each piece's control points
/// back out. Degenerate cases (zero radii, identical endpoints, an ellipse too
/// small to span the chord) collapse to a line, which is what every renderer
/// does and what the specification requires.
fn arc_to_cubics(
    path: &mut Path,
    from: Point,
    rx: f64,
    ry: f64,
    rotation_deg: f64,
    large_arc: bool,
    sweep: bool,
    to: Point,
) {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    if dx == 0.0 && dy == 0.0 {
        // Same endpoint: the arc is empty, and emitting a zero-length line would
        // show up as a stray subpath under the fill rule.
        return;
    }
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    let phi = rotation_deg.to_radians();
    let (sin_phi, cos_phi) = phi.sin_cos();

    // Step 1: (x1', y1') — the chord midpoint relative to the chord's centre,
    // rotated into the ellipse frame. The direction is `from - to`, not
    // `to - from`: with the sign flipped, an exact semicircle (whose centre
    // *is* the chord midpoint) gets a zero-length vector here and its start
    // angle lands on the wrong end of the arc.
    let mid = Point::new(-dx / 2.0, -dy / 2.0);
    let x1p = cos_phi * mid.x + sin_phi * mid.y;
    let y1p = -sin_phi * mid.x + cos_phi * mid.y;

    if rx == 0.0 || ry == 0.0 {
        path.line_to(to);
        return;
    }

    // Step 2: grow radii that are too small to reach the chord (F.6.6).
    let lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }

    // Step 3: the centre, in the ellipse frame (F.6.5).
    let num = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p;
    let den = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
    let mut coef = if den == 0.0 {
        0.0
    } else {
        (num / den).max(0.0).sqrt()
    };
    if large_arc == sweep {
        coef = -coef;
    }
    let cxp = coef * rx * y1p / ry;
    let cyp = -coef * ry * x1p / rx;
    // ...and back out to user space.
    let center = Point::new(
        cos_phi * cxp - sin_phi * cyp + (from.x + to.x) / 2.0,
        sin_phi * cxp + cos_phi * cyp + (from.y + to.y) / 2.0,
    );

    // Step 4: start/end angles, then the sweep angle (F.6.5).
    let angle = |ux: f64, uy: f64, vx: f64, vy: f64| {
        let dot = ux * vx + uy * vy;
        let len = (ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt();
        let mut a = (dot / len).clamp(-1.0, 1.0).acos();
        if ux * vy - uy * vx < 0.0 {
            a = -a;
        }
        a
    };
    let ux = (x1p - cxp) / rx;
    let uy = (y1p - cyp) / ry;
    let vx = (-x1p - cxp) / rx;
    let vy = (-y1p - cyp) / ry;
    let theta1 = angle(1.0, 0.0, ux, uy);
    let mut delta = angle(ux, uy, vx, vy);
    if !sweep && delta > 0.0 {
        delta -= 2.0 * std::f64::consts::PI;
    } else if sweep && delta < 0.0 {
        delta += 2.0 * std::f64::consts::PI;
    }

    // Step 5: at most four cubics, each spanning at most a quarter turn.
    let segments = (delta.abs() / std::f64::consts::FRAC_PI_2).ceil().max(1.0) as usize;
    let step = delta / segments as f64;
    // The circle-to-Bézier constant: the control points of an arc spanning `t`
    // sit `4/3 * tan(t/4)` along the tangents from each endpoint.
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    // A point on the ellipse at parameter `theta`, and the ellipse's tangent
    // direction there -- both in user space, i.e. rotated back by `phi`.
    let on_ellipse = |cos_t: f64, sin_t: f64| {
        Point::new(
            center.x + cos_phi * rx * cos_t - sin_phi * ry * sin_t,
            center.y + sin_phi * rx * cos_t + cos_phi * ry * sin_t,
        )
    };
    let tangent = |cos_t: f64, sin_t: f64| {
        Point::new(
            -cos_phi * rx * sin_t - sin_phi * ry * cos_t,
            -sin_phi * rx * sin_t + cos_phi * ry * cos_t,
        )
    };
    let mut theta = theta1;
    for _ in 0..segments {
        let (sin_0, cos_0) = theta.sin_cos();
        let (sin_1, cos_1) = (theta + step).sin_cos();
        let start = on_ellipse(cos_0, sin_0);
        let end = on_ellipse(cos_1, sin_1);
        let t0 = tangent(cos_0, sin_0);
        let t1 = tangent(cos_1, sin_1);
        path.cubic_to(
            Point::new(start.x + k * t0.x, start.y + k * t0.y),
            Point::new(end.x - k * t1.x, end.y - k * t1.y),
            end,
        );
        theta += step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segs(data: &str) -> Vec<PathSeg> {
        parse_path_data(data).segments().to_vec()
    }

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn absolute_move_line_close() {
        assert_eq!(
            segs("M0 0 L10 0 L10 10 Z"),
            vec![
                PathSeg::MoveTo(p(0.0, 0.0)),
                PathSeg::LineTo(p(10.0, 0.0)),
                PathSeg::LineTo(p(10.0, 10.0)),
                PathSeg::Close,
            ]
        );
    }

    #[test]
    fn a_repeated_moveto_argument_pair_becomes_a_lineto() {
        // `M 1 2 3 4` is "move to 1,2 then line to 3,4" -- the single rule most
        // hand-written SVG parsers get wrong.
        assert_eq!(
            segs("M1 2 3 4 5 6"),
            vec![
                PathSeg::MoveTo(p(1.0, 2.0)),
                PathSeg::LineTo(p(3.0, 4.0)),
                PathSeg::LineTo(p(5.0, 6.0)),
            ]
        );
    }

    #[test]
    fn relative_commands_are_measured_from_the_current_point() {
        assert_eq!(
            segs("m1 1 l2 0 l0 2 z"),
            vec![
                PathSeg::MoveTo(p(1.0, 1.0)),
                PathSeg::LineTo(p(3.0, 1.0)),
                PathSeg::LineTo(p(3.0, 3.0)),
                PathSeg::Close,
            ]
        );
    }

    #[test]
    fn closing_returns_the_current_point_to_the_subpath_start() {
        // A line after `Z` starts from the subpath origin, not from where the
        // curve happened to end.
        assert_eq!(
            segs("M5 5 L10 10 Z L1 1"),
            vec![
                PathSeg::MoveTo(p(5.0, 5.0)),
                PathSeg::LineTo(p(10.0, 10.0)),
                PathSeg::Close,
                PathSeg::LineTo(p(1.0, 1.0)),
            ]
        );
    }

    #[test]
    fn horizontal_and_vertical_shorthands() {
        assert_eq!(
            segs("M0 0 H5 V5 h-5 v-5"),
            vec![
                PathSeg::MoveTo(p(0.0, 0.0)),
                PathSeg::LineTo(p(5.0, 0.0)),
                PathSeg::LineTo(p(5.0, 5.0)),
                PathSeg::LineTo(p(0.0, 5.0)),
                PathSeg::LineTo(p(0.0, 0.0)),
            ]
        );
    }

    #[test]
    fn a_sign_is_a_separator() {
        // Real icon data is full of `10.59 12 5 17.59`: two numbers whose second
        // one is negative with no space before it.
        assert_eq!(
            segs("M10.59 12 5 17.59"),
            vec![
                PathSeg::MoveTo(p(10.59, 12.0)),
                PathSeg::LineTo(p(5.0, 17.59)),
            ]
        );
        assert_eq!(
            segs("M.04-.3"),
            vec![PathSeg::MoveTo(p(0.04, -0.3))],
            "a leading `.` and a glued sign must both parse"
        );
    }

    #[test]
    fn a_smooth_cubic_reflects_the_previous_control_point() {
        // C(0,0 10,0 10,10) then S: the reflected first control point is
        // current (10,10) mirrored through the previous c2 (10,0) -> (10,20).
        assert_eq!(
            segs("M0 0 C0 0 10 0 10 10 S20 20 20 0"),
            vec![
                PathSeg::MoveTo(p(0.0, 0.0)),
                PathSeg::CubicTo {
                    c1: p(0.0, 0.0),
                    c2: p(10.0, 0.0),
                    to: p(10.0, 10.0),
                },
                PathSeg::CubicTo {
                    c1: p(10.0, 20.0),
                    c2: p(20.0, 20.0),
                    to: p(20.0, 0.0),
                },
            ]
        );
    }

    #[test]
    fn a_smooth_cubic_without_a_preceding_curve_uses_the_current_point() {
        assert_eq!(
            segs("M5 5 S10 10 20 20"),
            vec![
                PathSeg::MoveTo(p(5.0, 5.0)),
                PathSeg::CubicTo {
                    c1: p(5.0, 5.0),
                    c2: p(10.0, 10.0),
                    to: p(20.0, 20.0),
                },
            ]
        );
    }

    #[test]
    fn a_line_between_two_curves_clears_the_reflection_point() {
        // The `S` after `L` must not reflect a cubic from before the line.
        assert_eq!(
            segs("M0 0 C0 0 10 0 10 10 L20 20 S30 30 40 40"),
            vec![
                PathSeg::MoveTo(p(0.0, 0.0)),
                PathSeg::CubicTo {
                    c1: p(0.0, 0.0),
                    c2: p(10.0, 0.0),
                    to: p(10.0, 10.0),
                },
                PathSeg::LineTo(p(20.0, 20.0)),
                PathSeg::CubicTo {
                    c1: p(20.0, 20.0),
                    c2: p(30.0, 30.0),
                    to: p(40.0, 40.0),
                },
            ]
        );
    }

    #[test]
    fn quadratics_and_their_smooth_form() {
        assert_eq!(
            segs("M0 0 Q5 5 10 0 T20 0"),
            vec![
                PathSeg::MoveTo(p(0.0, 0.0)),
                PathSeg::QuadTo {
                    ctrl: p(5.0, 5.0),
                    to: p(10.0, 0.0),
                },
                PathSeg::QuadTo {
                    ctrl: p(15.0, -5.0),
                    to: p(20.0, 0.0),
                },
            ]
        );
    }

    #[test]
    fn an_arc_becomes_cubics_that_end_on_the_endpoint() {
        // A half-circle of radius 5 from (0,0) to (10,0) splits into two cubics.
        let path = parse_path_data("M0 0 A5 5 0 0 1 10 0");
        let segments = path.segments();
        assert_eq!(
            segments.len(),
            3,
            "one move plus two quarter-turn cubics, got {segments:?}"
        );
        match segments.last().unwrap() {
            PathSeg::CubicTo { to, .. } => {
                assert!(
                    (to.x - 10.0).abs() < 1e-9 && to.y.abs() < 1e-9,
                    "endpoint was {to:?}"
                );
            }
            other => panic!("expected a cubic, got {other:?}"),
        }
    }

    #[test]
    fn an_arc_sweep_flag_flips_the_side_of_the_chord() {
        // SVG's y axis points *down*, so sweep=1 (a positive-angle sweep, which
        // is clockwise on screen) bulges below the chord. Getting this backwards
        // silently mirrors every arc in an icon set, so it is worth a test.
        let clockwise = parse_path_data("M0 0 A5 5 0 0 1 10 0");
        let counter = parse_path_data("M0 0 A5 5 0 0 0 10 0");
        let peak = |path: &Path| {
            path.segments()
                .iter()
                .filter_map(|s| match s {
                    PathSeg::CubicTo { c1, .. } => Some(c1.y),
                    _ => None,
                })
                .fold(f64::NEG_INFINITY, f64::max)
        };
        // Why negative y for sweep=1: the spec defines it as the
        // positive-angle direction, `theta` increasing. Starting at the
        // leftmost point of the circle, `theta` is pi, and increasing it moves
        // toward 3pi/2 where `y = cy - ry` -- negative, because SVG's y axis
        // points down. Measured from the parsed path, not reasoned about.
        assert!(peak(&clockwise) < 0.0, "sweep=1 bulges toward negative y");
        assert!(peak(&counter) > 0.0, "sweep=0 is its mirror image");
    }

    #[test]
    fn an_arc_with_a_zero_radius_is_a_line() {
        assert_eq!(
            segs("M0 0 A0 0 0 0 1 10 10"),
            vec![
                PathSeg::MoveTo(p(0.0, 0.0)),
                PathSeg::LineTo(p(10.0, 10.0)),
            ]
        );
    }

    #[test]
    fn commas_and_newlines_are_separators_too() {
        assert_eq!(
            segs("M0,0\nL10\r\n,10"),
            vec![
                PathSeg::MoveTo(p(0.0, 0.0)),
                PathSeg::LineTo(p(10.0, 10.0)),
            ]
        );
    }

    #[test]
    fn exponents_parse() {
        assert_eq!(segs("M1e1 2E-1"), vec![PathSeg::MoveTo(p(10.0, 0.2))]);
    }

    #[test]
    fn malformed_data_stops_parsing_instead_of_panicking() {
        // Nothing here may panic, and each case is pinned on what it *keeps*:
        // a parser that stops one command early and one that keeps going both
        // produce a path, so counting is the only way to tell them apart.
        assert_eq!(parse_path_data("").segments().len(), 0);
        assert_eq!(parse_path_data("M").segments().len(), 0);
        assert_eq!(parse_path_data("M 1").segments().len(), 0);
        assert_eq!(parse_path_data("M 1 1 L").segments().len(), 1);
        // `z` really is a command, so `zzz` is three closes rather than garbage.
        assert_eq!(parse_path_data("zzz").segments().len(), 3);
        // An unknown command is not a move and is not skipped over either: the
        // data is untrusted from here, so parsing stops.
        assert_eq!(parse_path_data("M 0 0 X 5 5").segments().len(), 1);
        assert_eq!(parse_path_data("M 0 0 A5 5 0").segments().len(), 1);
    }

    #[test]
    fn a_real_material_icon_path_parses_to_segments() {
        // The `close` glyph from the Material set floem-ui ships, unedited: if
        // this parses, the grammar covers the kit's data.
        let path = parse_path_data(
            "M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z",
        );
        // The `close` glyph is all straight edges: one move, eleven lines, one
        // close. Counting them exactly is the point -- it proves implicit
        // repetition ran to the end of the argument list rather than stopping
        // after one pair.
        assert_eq!(path.segments().len(), 13, "{:?}", path.segments());
        assert!(
            matches!(path.segments().last(), Some(PathSeg::Close)),
            "the trailing `z` must survive: {:?}",
            path.segments()
        );
    }
}
