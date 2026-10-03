//! Paint brushes: solid colours plus the two gradients every UI needs.
//!
//! A [`Brush`] is what a shape is *filled with*. `Color` stays the type of
//! "a colour" everywhere else; a brush is only needed at the last step, where
//! a painter asks "what is at this pixel". Keeping the two apart means the
//! ~everywhere `Color` parameters survive unchanged and gradient support is
//! opt-in at the paint site, exactly as in Compose (`color:` vs `brush:`
//! overloads) and Flutter (`Paint..shader`).
//!
//! Two gradient shapes, not five: linear and radial cover buttons, progress
//! fills, scrim fades and hero washes; a sweep gradient is a decorative special
//! case that no 2.0 widget has asked for yet. Adding one later is a new variant
//! plus one `sample` arm, not a redesign.

use crate::{Color, Point};

/// A drop shadow behind a shape: colour, blur radius, offset.
///
/// The scene carries the *request*; each backend answers with its best blur.
/// A shadow is blur + an offset silhouette, so a backend without a blur
/// primitive can still draw the offset silhouette and count what it could not
/// do (the degradation is recorded, never silent).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadow {
    /// Silhouette colour — usually a translucent black.
    pub color: Color,
    /// Blur radius in logical px. `0.0` is a hard-edged silhouette.
    pub blur: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    /// How far the silhouette is dilated (or eroded, when negative) *before*
    /// the blur is applied, in logical px.
    ///
    /// This is CSS's fourth `box-shadow` length — `0 4px 6px 2px #000` has a
    /// spread of `2px` — and it dilates the *shape*, it does not widen the
    /// blur: a rounded box grows on every side with its corner radius growing
    /// to match, so the silhouette stays a rounded box instead of becoming a
    /// rectangle with soft corners. Negative spread erodes it.
    ///
    /// Order matters and is CSS's: offset, then spread, then blur. Applying
    /// spread after the blur would read as "fatter shadow", which is a
    /// different and much rarer effect.
    ///
    /// A backend that cannot dilate must ignore it rather than approximate it
    /// with blur, and record the degradation — a wrong shadow is worse than an
    /// absent one.
    pub spread: f64,
}

/// Backend support for `spread`.
///
/// `floom-renderer-software` is the reference: it folds `spread` into the
/// silhouette's signed distance field, so the shape is genuinely dilated (or
/// eroded) before the blur runs. The other backends do not implement
/// `draw_shadow` at all yet — they carry the request through the scene so it
/// survives to reach a backend that can answer it, which is the same
/// "carried rather than dropped" posture vello's shadow test asserts. When one
/// of them grows shadow support it must dilate rather than widen the blur, and
/// must count what it could not do rather than approximating silently.
impl Shadow {
    /// How far past the shape's own bounds the shadow can reach.
    ///
    /// The offset shifts the whole silhouette; spread dilates it; the blur
    /// smears the result outward by roughly its radius. Negative spread pulls
    /// the silhouette *in*, so it cannot extend the reach and is clamped at
    /// zero here. Conservative by construction — a damage region built from
    /// this must contain the shadow, never clip it.
    pub fn reach(&self) -> f64 {
        self.offset_x.abs().max(self.offset_y.abs()) + self.blur + self.spread.max(0.0)
    }
}

/// A post-processing filter over **already-drawn** content.
///
/// This is the other half of the blur story, and the reason it cannot just be
/// folded into [`Shadow`]: a shadow is a blurred silhouette *drawn behind one
/// shape*, while a filter is something done to a band of picture that has
/// already been painted — Compose's `Modifier.blur` / `renderEffect`,
/// Flutter's `ImageFiltered` and `BackdropFilter`. One shape cannot express the
/// other: a filter has to see pixels contributed by several nodes at once, so
/// it lives in the draw order rather than inside a node.
///
/// The scene carries the *request*; each backend answers with its best
/// approximation and is expected to count what it could not do (see
/// `shadows_skipped`'s precedent — the degradation is recorded, never silent).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Filter {
    /// Blur, `radius` in logical px. Separate radii per axis because a
    /// horizontal motion smear and a frosted panel are both real requests, and
    /// collapsing them into one number would force a caller to pick.
    Blur { radius_x: f64, radius_y: f64 },
}

impl Filter {
    /// A blur with the same radius on both axes — the common case.
    pub fn blur(radius: f64) -> Self {
        Filter::Blur {
            radius_x: radius,
            radius_y: radius,
        }
    }

    /// A blur with a different radius per axis.
    pub fn blur_xy(radius_x: f64, radius_y: f64) -> Self {
        Filter::Blur { radius_x, radius_y }
    }

    /// Whether this filter would leave the picture unchanged.
    ///
    /// A zero-radius blur has nothing to smear, so a backend must not pay for a
    /// layer to apply it; a negative radius is a caller's typo and reads as the
    /// same thing rather than as "blur inward". Non-finite radii land here too
    /// — a radius no kernel can be built from is no blur at all, and the
    /// alternative (saturating an infinite radius into a buffer size) is a
    /// hang waiting to happen.
    pub fn is_identity(&self) -> bool {
        match *self {
            Filter::Blur { radius_x, radius_y } => {
                !((radius_x > 0.0 && radius_x.is_finite())
                    || (radius_y > 0.0 && radius_y.is_finite()))
            }
        }
    }

    /// How far outside its own area the filtered result can spread, in logical
    /// px — what a damage region has to include so the blur's outer edge is
    /// not clipped.
    ///
    /// Equals the blur radius rather than the kernel radius any one pass uses:
    /// a triple box blur of radius `r/3` spreads by `r`, which is what makes
    /// this number the *whole* spread instead of a per-pass detail. Non-finite
    /// radii reach nothing, for the reason [`Self::is_identity`] gives.
    pub fn reach(&self) -> f64 {
        match *self {
            Filter::Blur { radius_x, radius_y } => {
                let widest = radius_x.max(radius_y);
                if widest.is_finite() {
                    widest.max(0.0)
                } else {
                    0.0
                }
            }
        }
    }
}

/// What a shape is filled with: one colour, or a colour that varies.
///
/// Stops are `(offset, color)` pairs with offsets in `0.0..=1.0`. They are
/// evaluated as-is — unsorted stops are the caller's typo and produce the same
/// folded interpolation a GPU gradient would, so no code path hides it.
#[derive(Debug, Clone, PartialEq)]
pub enum Brush {
    /// One colour everywhere. The `From<Color>` conversion is what keeps the
    /// pre-gradient call sites (and pre-gradient scenes) valid.
    Solid(Color),
    /// A colour ramp along the `from → to` axis; before `from` is the first
    /// stop, past `to` the last.
    Linear {
        stops: Vec<(f32, Color)>,
        from: Point,
        to: Point,
    },
    /// A colour ramp radiating from `center` out to `radius`; the last stop
    /// holds beyond it.
    Radial {
        stops: Vec<(f32, Color)>,
        center: Point,
        radius: f64,
    },
}

impl From<Color> for Brush {
    fn from(color: Color) -> Self {
        Brush::Solid(color)
    }
}

impl Brush {
    /// A linear gradient. Takes a slice so a caller can spell its stops inline.
    pub fn linear(stops: &[(f32, Color)], from: Point, to: Point) -> Self {
        Brush::Linear {
            stops: stops.to_vec(),
            from,
            to,
        }
    }

    /// A radial gradient. Takes a slice so a caller can spell its stops inline.
    pub fn radial(stops: &[(f32, Color)], center: Point, radius: f64) -> Self {
        Brush::Radial {
            stops: stops.to_vec(),
            center,
            radius,
        }
    }

    /// The colour at `p`, in the coordinate space the brush was built in.
    ///
    /// Degenerate geometry does not divide by zero: a zero-length linear axis
    /// and a zero (or negative) radial radius both sample their *first* stop,
    /// which is what "the gradient never got anywhere" should look like.
    pub fn sample(&self, p: Point) -> Color {
        match self {
            Brush::Solid(color) => *color,
            Brush::Linear { stops, from, to } => {
                let axis = Point::new(to.x - from.x, to.y - from.y);
                let length_sq = axis.x * axis.x + axis.y * axis.y;
                let t = if length_sq <= f64::EPSILON {
                    0.0
                } else {
                    (((p.x - from.x) * axis.x + (p.y - from.y) * axis.y) / length_sq)
                        .clamp(0.0, 1.0)
                };
                sample_stops(stops, t as f32)
            }
            Brush::Radial {
                stops,
                center,
                radius,
            } => {
                let t = if *radius <= f64::EPSILON {
                    0.0
                } else {
                    (p.distance_to(*center) / radius).clamp(0.0, 1.0)
                };
                sample_stops(stops, t as f32)
            }
        }
    }

    /// The one colour, when this brush is solid.
    ///
    /// The fast path every backend takes first: a solid brush must not pay for
    /// per-pixel sampling, and a caller that can only take a colour asks here
    /// rather than guessing.
    pub fn solid(&self) -> Option<Color> {
        match self {
            Brush::Solid(color) => Some(*color),
            _ => None,
        }
    }

    /// Whether every colour in the brush is unchanged by `with_alpha_factor` —
    /// the check that lets a composite fade skip re-building the brush.
    pub fn is_fully_opaque(&self) -> bool {
        match self {
            Brush::Solid(color) => color.a == u8::MAX,
            Brush::Linear { stops, .. } | Brush::Radial { stops, .. } => {
                stops.iter().all(|(_, c)| c.a == u8::MAX)
            }
        }
    }

    /// The brush with every colour's alpha multiplied by `factor` (clamped to
    /// `0.0..=1.0`).
    ///
    /// This is how a compositor fade (an overlay's entry, an element's exit)
    /// dims a gradient exactly the way it dims a solid fill — every stop
    /// scales, so the ramp stays a ramp instead of collapsing to its ends.
    pub fn with_alpha_factor(&self, factor: f64) -> Self {
        let factor = factor.clamp(0.0, 1.0);
        let scale = |c: Color| {
            let a = (f64::from(c.a) * factor).round().clamp(0.0, 255.0) as u8;
            Color::rgba(c.r, c.g, c.b, a)
        };
        match self {
            Brush::Solid(color) => Brush::Solid(scale(*color)),
            Brush::Linear { stops, from, to } => Brush::Linear {
                stops: stops.iter().map(|(o, c)| (*o, scale(*c))).collect(),
                from: *from,
                to: *to,
            },
            Brush::Radial {
                stops,
                center,
                radius,
            } => Brush::Radial {
                stops: stops.iter().map(|(o, c)| (*o, scale(*c))).collect(),
                center: *center,
                radius: *radius,
            },
        }
    }
}

/// Evaluates a stop list at `t` in `0.0..=1.0`.
///
/// Before the first stop and past the last are the edge colours — a gradient
/// clamps, it does not wrap. An *empty* stop list samples transparent, which is
/// the honest answer to "no colours were given" and keeps a bad brush a visual
/// no-op rather than a panic in a paint walk.
fn sample_stops(stops: &[(f32, Color)], t: f32) -> Color {
    let Some(first) = stops.first() else {
        return Color::TRANSPARENT;
    };
    if t <= first.0 {
        return first.1;
    }
    let Some(last) = stops.last() else {
        return Color::TRANSPARENT;
    };
    if t >= last.0 {
        return last.1;
    }
    for pair in stops.windows(2) {
        let (t0, c0) = pair[0];
        let (t1, c1) = pair[1];
        if t <= t1 {
            let span = t1 - t0;
            let u = if span <= f32::EPSILON {
                1.0
            } else {
                (t - t0) / span
            };
            return lerp_color(c0, c1, u);
        }
    }
    last.1
}

/// Per-channel interpolation in straight (non-premultiplied) alpha, the same
/// space `Color` itself is stored in — so a gradient across transparent colours
/// keeps their hue instead of darkening toward black.
fn lerp_color(a: Color, b: Color, u: f32) -> Color {
    let mix = |x: u8, y: u8| -> u8 {
        let v = f32::from(x) + (f32::from(y) - f32::from(x)) * u;
        v.round().clamp(0.0, 255.0) as u8
    };
    Color::rgba(mix(a.r, b.r), mix(a.g, b.g), mix(a.b, b.b), mix(a.a, b.a))
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Color = Color::rgb(255, 0, 0);
    const BLUE: Color = Color::rgb(0, 0, 255);

    #[test]
    fn a_colour_becomes_a_solid_brush_and_back() {
        let brush: Brush = RED.into();
        assert_eq!(brush, Brush::Solid(RED));
        assert_eq!(brush.solid(), Some(RED));
        assert_eq!(brush.sample(Point::new(100.0, -3.0)), RED);
    }

    #[test]
    fn a_linear_gradient_samples_its_axis_by_projection() {
        // Horizontal ramp across x = 0..100: the sample point is projected onto
        // the axis, so a point off the axis (same x) lands at the same t.
        let brush = Brush::linear(
            &[(0.0, RED), (1.0, BLUE)],
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
        );
        assert_eq!(brush.sample(Point::new(0.0, 40.0)), RED);
        assert_eq!(brush.sample(Point::new(100.0, 40.0)), BLUE);
        // Half-way in x is half-way in colour regardless of y.
        let mid = brush.sample(Point::new(50.0, 7.0));
        assert_eq!((mid.r, mid.b), (128, 128), "midpoint splits the channels");
        // Clamped past both ends.
        assert_eq!(brush.sample(Point::new(-30.0, 0.0)), RED);
        assert_eq!(brush.sample(Point::new(130.0, 0.0)), BLUE);
    }

    #[test]
    fn a_diagonal_axis_weights_both_coordinates() {
        // from (0,0) to (100,100): the ramp parameter is the *projection*
        // onto the axis, not x. (50,0) projects to a quarter of the axis,
        // (100,0) to half, and (50,50) — on the axis — to exactly half.
        let brush = Brush::linear(
            &[(0.0, RED), (1.0, BLUE)],
            Point::new(0.0, 0.0),
            Point::new(100.0, 100.0),
        );
        let quarter = brush.sample(Point::new(50.0, 0.0));
        assert_eq!((quarter.r, quarter.b), (191, 64));
        let half = brush.sample(Point::new(100.0, 0.0));
        assert_eq!((half.r, half.b), (128, 128));
        let on_axis = brush.sample(Point::new(50.0, 50.0));
        assert_eq!((on_axis.r, on_axis.b), (128, 128));
    }

    #[test]
    fn a_radial_gradient_samples_by_distance_from_its_center() {
        let brush = Brush::radial(&[(0.0, RED), (1.0, BLUE)], Point::new(50.0, 50.0), 50.0);
        assert_eq!(brush.sample(Point::new(50.0, 50.0)), RED);
        assert_eq!(brush.sample(Point::new(100.0, 50.0)), BLUE);
        let mid = brush.sample(Point::new(50.0 + 25.0, 50.0));
        assert_eq!((mid.r, mid.b), (128, 128));
        // Past the radius the last stop holds.
        assert_eq!(brush.sample(Point::new(50.0, 200.0)), BLUE);
    }

    #[test]
    fn degenerate_geometry_samples_the_first_stop_instead_of_dividing_by_zero() {
        let linear = Brush::linear(
            &[(0.0, RED), (1.0, BLUE)],
            Point::new(5.0, 5.0),
            Point::new(5.0, 5.0),
        );
        assert_eq!(linear.sample(Point::new(0.0, 0.0)), RED);
        let radial = Brush::radial(&[(0.0, RED), (1.0, BLUE)], Point::new(5.0, 5.0), 0.0);
        assert_eq!(radial.sample(Point::new(9.0, 9.0)), RED);
    }

    #[test]
    fn stops_clamp_before_the_first_and_after_the_last_offset() {
        // Offsets 0.2..0.8: everything outside that band is the edge colour.
        let brush = Brush::linear(
            &[(0.2, RED), (0.8, BLUE)],
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
        );
        assert_eq!(brush.sample(Point::new(10.0, 0.0)), RED);
        assert_eq!(brush.sample(Point::new(90.0, 0.0)), BLUE);
        let mid = brush.sample(Point::new(50.0, 0.0));
        assert_eq!((mid.r, mid.b), (128, 128));
    }

    #[test]
    fn an_empty_stop_list_is_transparent_not_a_panic() {
        let brush = Brush::linear(&[], Point::ZERO, Point::new(10.0, 0.0));
        assert_eq!(brush.sample(Point::new(5.0, 0.0)), Color::TRANSPARENT);
    }

    #[test]
    fn alpha_scaling_touches_every_stop_and_no_channel_else() {
        let brush = Brush::linear(
            &[(0.0, RED), (1.0, BLUE)],
            Point::ZERO,
            Point::new(10.0, 0.0),
        );
        let faded = brush.with_alpha_factor(0.5);
        assert_eq!(faded.solid(), None, "a scaled gradient is no longer solid");
        let (r0, b0) = match &faded {
            Brush::Linear { stops, .. } => (stops[0].1, stops[1].1),
            _ => unreachable!(),
        };
        assert_eq!((r0.r, r0.g, r0.b, r0.a), (255, 0, 0, 128));
        assert_eq!((b0.r, b0.g, b0.b, b0.a), (0, 0, 255, 128));
        // Out-of-range factors clamp instead of wrapping.
        let clamp = brush.with_alpha_factor(2.0);
        assert!(clamp.is_fully_opaque());
        assert!(brush.is_fully_opaque());
        let transparent = Brush::Solid(Color::rgba(1, 2, 3, 0));
        assert!(!transparent.is_fully_opaque());
    }

    #[test]
    fn a_zero_radius_blur_is_the_identity_filter() {
        // A backend must be able to skip the layer entirely for these, so the
        // answer has to come from the value and not from a radius comparison at
        // each call site.
        assert!(Filter::blur(0.0).is_identity());
        assert!(Filter::blur_xy(0.0, 0.0).is_identity());
        // A negative radius is a caller's typo, and reads as "no blur" rather
        // than as something that blurs inward.
        assert!(Filter::blur(-4.0).is_identity());
        assert!(Filter::blur_xy(-1.0, -1.0).is_identity());
        assert!(!Filter::blur(0.5).is_identity());
        // Non-finite radii cannot fail open into a kernel no buffer can hold.
        assert!(Filter::blur(f64::NAN).is_identity());
        assert!(Filter::blur(f64::INFINITY).is_identity());
        assert_eq!(Filter::blur(f64::INFINITY).reach(), 0.0);
    }

    #[test]
    fn a_blur_reaches_its_radius_not_its_per_pass_kernel() {
        // The trap this pins: a triple box blur of radius `r/3` spreads by
        // `r`, so `reach` has to be the *whole* spread. A damage region built
        // from a per-pass kernel radius would clip the blur's outer edge.
        assert_eq!(Filter::blur(9.0).reach(), 9.0);
        // Two axes, one number: the widest one wins, or a horizontal smear
        // would be cut off vertically instead.
        assert_eq!(Filter::blur_xy(10.0, 4.0).reach(), 10.0);
        assert_eq!(Filter::blur_xy(4.0, 10.0).reach(), 10.0);
    }

    #[test]
    fn the_two_axes_stay_distinguishable() {
        // Collapsing them into one radius would make a uniform blur and a
        // one-axis smear equal — and then unexpressible.
        assert_ne!(Filter::blur(6.0), Filter::blur_xy(6.0, 1.0));
        assert_eq!(Filter::blur(6.0), Filter::blur_xy(6.0, 6.0));
        // Copy, so a scene node can hold one by value without a lifetime.
        let filter = Filter::blur(3.0);
        let copied = filter;
        assert_eq!(filter, copied);
    }

    #[test]
    fn interpolating_through_transparent_keeps_the_hue() {
        // The trap the state-layer bug taught: lerping channels from a
        // transparent black would pull toward black. Straight-alpha
        // interpolation keeps each endpoint's own RGB; only the coverage ramps.
        let brush = Brush::linear(
            &[
                (0.0, Color::rgba(255, 0, 0, 0)),
                (1.0, Color::rgba(255, 0, 0, 255)),
            ],
            Point::ZERO,
            Point::new(10.0, 0.0),
        );
        let mid = brush.sample(Point::new(5.0, 0.0));
        assert_eq!((mid.r, mid.g, mid.b, mid.a), (255, 0, 0, 128));
    }
}
