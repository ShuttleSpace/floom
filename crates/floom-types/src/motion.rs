//! Motion vocabulary — the *shape* of an animation, as plain data.
//!
//! Nothing here drives anything: no clock, no scheduler, no host state. What
//! lives here is what more than one layer has to be able to *name* — a timing
//! curve, a repeat policy, the compositor channel an animation drives, and a
//! declarative description of an animation a widget wants kept running on its
//! own element.
//!
//! It sits in `floom-types` rather than beside the scheduler because the
//! layers that need these names cannot all depend on `floom-animation`:
//! `floom-core` (a widget declares an [`AnimationSpec`] through the `Widget`
//! contract) is below it in the layer order. `floom-animation` re-exports
//! everything here, so `floom_animation::Easing` keeps working and there is
//! still exactly one definition of each type.

/// Maps normalised progress to normalised progress.
///
/// A curve takes the fraction of an animation's duration that has elapsed
/// (`0.0 ..= 1.0`) and returns the fraction of the distance to travel. Every
/// variant satisfies `ease(0) == 0` and `ease(1) == 1` exactly, so a curve can
/// never leave an animation parked off its endpoint; what differs is the shape
/// in between, and a spring is allowed to overshoot past `1.0` on the way.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Easing {
    /// Constant speed. The default: a caller that says nothing about timing
    /// gets the same motion this framework had before curves existed.
    #[default]
    Linear,
    /// The CSS / Flutter cubic-bezier form, with control points `(x1, y1)` and
    /// `(x2, y2)` and implicit endpoints `(0, 0)` and `(1, 1)`.
    ///
    /// `x` values are clamped to `0.0 ..= 1.0` when sampled: outside that
    /// range the curve is no longer a function of time (the same instant maps
    /// to several positions), which has no meaning for an animation.
    CubicBezier { x1: f64, y1: f64, x2: f64, y2: f64 },
    /// The shape of a damped spring, stretched over whatever duration the
    /// caller asked for.
    ///
    /// *Not* the same thing as a spring-driven animation (see
    /// [`crate::AnimationSpec`] vs. the scheduler's spring kind): this variant
    /// has a fixed duration and therefore a definite end, and its stiffness is
    /// expressed over the *normalised* duration, so the same numbers produce
    /// the same shape whether the animation runs for 80 ms or 800 ms. Reach for
    /// it when you want overshoot with a deadline; reach for a real spring when
    /// you want the physics to decide when the motion is over.
    Spring { stiffness: f64, damping: f64 },
}

impl Easing {
    /// CSS `ease-in`: slow to start, fast at the end.
    pub const EASE_IN: Self = Self::CubicBezier {
        x1: 0.42,
        y1: 0.0,
        x2: 1.0,
        y2: 1.0,
    };

    /// CSS `ease-out`: fast to start, slow to settle.
    ///
    /// The right default for something that is *appearing*: it arrives at full
    /// speed and decelerates into place, which reads as responsive, where
    /// `ease-in` on an entrance reads as reluctant.
    pub const EASE_OUT: Self = Self::CubicBezier {
        x1: 0.0,
        y1: 0.0,
        x2: 0.58,
        y2: 1.0,
    };

    /// CSS `ease-in-out`: slow at both ends, fast in the middle.
    pub const EASE_IN_OUT: Self = Self::CubicBezier {
        x1: 0.42,
        y1: 0.0,
        x2: 0.58,
        y2: 1.0,
    };

    /// The curve with explicit control points.
    pub const fn cubic_bezier(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self::CubicBezier { x1, y1, x2, y2 }
    }

    /// Samples the curve at `progress`.
    ///
    /// Out-of-range input is clamped, so a caller that overshoots the
    /// animation's duration gets its endpoint rather than a curve extrapolated
    /// into nonsense.
    pub fn ease(&self, progress: f64) -> f64 {
        let u = progress.clamp(0.0, 1.0);
        match *self {
            Self::Linear => u,
            Self::CubicBezier { x1, y1, x2, y2 } => {
                if u <= 0.0 {
                    return 0.0;
                }
                if u >= 1.0 {
                    return 1.0;
                }
                cubic_bezier_at(x1, y1, x2, y2, u)
            }
            Self::Spring { stiffness, damping } => {
                if u <= 0.0 {
                    return 0.0;
                }
                if u >= 1.0 {
                    return 1.0;
                }
                // Released from rest one unit away from home: the response is
                // the *shape* of the spring, and the caller's duration is the
                // time it gets to complete it. Forcing the endpoint above is
                // what keeps a non-settling spring from leaving the animation
                // stranded away from its target.
                1.0 + damped_spring(-1.0, 0.0, stiffness, damping, u).0
            }
        }
    }
}

/// How a repeating animation goes round again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RepeatMode {
    /// Jump back to the start of the cycle and play it again. A spinner that
    /// sweeps one way and restarts uses this.
    #[default]
    Restart,
    /// Play the cycle backwards every other time — a ping-pong that lands back
    /// where it started after each *pair* of half-cycles.
    Reverse,
}

/// How many times a repeating animation runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RepeatSpec {
    pub mode: RepeatMode,
    /// `None` means forever — the animation never reports finished, which is
    /// what keeps a frame loop awake for as long as it is mounted.
    pub iterations: Option<u32>,
}

impl RepeatSpec {
    /// An endless ping-pong, the shape a breathing/pulsing element wants.
    pub const INFINITE_REVERSE: Self = Self {
        mode: RepeatMode::Reverse,
        iterations: None,
    };

    /// `iterations` full cycles, jumping back to the start each time.
    pub const fn restart(iterations: u32) -> Self {
        Self {
            mode: RepeatMode::Restart,
            iterations: Some(iterations),
        }
    }

    /// `iterations` full there-and-back cycles.
    pub const fn reverse(iterations: u32) -> Self {
        Self {
            mode: RepeatMode::Reverse,
            iterations: Some(iterations),
        }
    }
}

/// A declarative description of a tween: where it goes, how long it takes, and
/// what it drives.
///
/// The vocabulary a *widget* can ask the host for. Deliberately narrower than
/// the scheduler's kind space — a widget can declare a timed, possibly
/// repeating tween, not a physical spring or a keyframe track, because those
/// need parameters (velocity, key tracks) that a widget has no business
/// inventing on its own element.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationSpec {
    /// The animation's principal scalar: where its self-reported value (see the
    /// paint context's `animation_value`) starts and ends.
    ///
    /// Every [`AnimationSpec::channels`] interpolates between its own `from`
    /// and `to` against the same normalised progress this driver describes, so
    /// one curve drives several properties in lock-step (B6).
    pub from: f64,
    pub to: f64,
    pub duration_secs: f64,
    pub easing: Easing,
    /// `None` plays once.
    pub repeat: Option<RepeatSpec>,
    /// The compositor channels this animation drives, each with its own value
    /// span. Empty means the animation drives nothing on its own — the step is
    /// handed to the element's owner to read (see the paint context's
    /// `animation_value`) and draw with, which is how a self-drawn control
    /// animates one part of itself instead of being translated whole.
    ///
    /// The payloads are *templates*, not values: each names a channel, and the
    /// sampled value replaces it on every frame. All channels share the
    /// animation's duration, easing, repeat and key.
    pub channels: Vec<CompositorChannel>,
    /// An opaque identity token for *this* motion, so that two otherwise
    /// identical specs can still be told apart.
    ///
    /// This is the implicit-transition hook (B4): a control whose target value
    /// changed re-declares the *same* `from`/`to`/`duration` (its transition
    /// is always "0 → 1, over this long, with this curve") but bumps `key`, so
    /// the host sees a *new* declaration and restarts the tween rather than
    /// leaving the finished one parked. Without the key, "I changed my target"
    /// would be indistinguishable from "I am re-stating my unchanged motion",
    /// and the transition would never replay. The key does not affect the
    /// sampled value — it only participates in [`AnimationSpec::matches`].
    pub key: u64,
}

impl AnimationSpec {
    /// A one-shot tween that drives nothing by itself.
    pub const fn new(from: f64, to: f64, duration_secs: f64) -> Self {
        Self {
            from,
            to,
            duration_secs,
            easing: Easing::Linear,
            repeat: None,
            channels: Vec::new(),
            key: 0,
        }
    }

    /// A repeating tween that drives nothing by itself.
    pub const fn repeating(from: f64, to: f64, duration_secs: f64, repeat: RepeatSpec) -> Self {
        // `Vec::new` is const-stable, so a const builder can still be offered.
        Self {
            from,
            to,
            duration_secs,
            easing: Easing::Linear,
            repeat: Some(repeat),
            channels: Vec::new(),
            key: 0,
        }
    }

    pub const fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Drives a single compositor channel over this animation's own value span,
    /// so the element moves without a re-layout.
    ///
    /// Shorthand for a one-entry [`Self::with_channels`]: the new channel's
    /// `from`/`to` mirror the driver's, so a single-prop caller behaves exactly
    /// as a single-scalar tween always did.
    pub fn with_prop(mut self, prop: CompositorProp) -> Self {
        self.channels.push(CompositorChannel {
            prop,
            from: self.from,
            to: self.to,
        });
        self
    }

    /// Drives several compositor channels in lock-step — the B6 multi-value
    /// arrangement. Each channel interpolates its own `from`→`to` against the
    /// shared easing, so opacity and translation (say) move together without a
    /// re-layout or a per-widget choreography of separate tweens.
    pub fn with_channels(mut self, channels: impl IntoIterator<Item = CompositorChannel>) -> Self {
        self.channels.extend(channels);
        self
    }

    /// Tags this declaration with an identity key. See the field's docs for
    /// what a changed key means to the host.
    pub const fn with_key(mut self, key: u64) -> Self {
        self.key = key;
        self
    }

    /// Whether two specs would produce the same motion.
    ///
    /// What the host reconciles a widget's declaration against: an animation
    /// whose spec is unchanged is left running rather than restarted, so a
    /// widget re-declaring its motion on every update does not restart it every
    /// update. The [`AnimationSpec::key`] is part of the comparison: two
    /// otherwise identical specs with different keys are different motions.
    pub fn matches(&self, other: &Self) -> bool {
        self == other
    }
}

/// A style property's implicit transition: "when this property's value
/// changes, animate from the old value to the new one over `duration_secs`
/// with `easing`, instead of jumping".
///
/// This is the B4 "change-the-value-and-it-animates" layer, the Compose
/// `animate*AsState` / Flutter `Animated*` counterpart. A control that carries
/// a [`Transition`] does not hold its own clock — it declares the *shape* of
/// the transition and reads the host's sampled progress back through
/// `PaintContext::animation_value`, interpolating between its old and new
/// value in its paint routine. The host restarts the underlying tween whenever
/// the control's declaration key changes (see [`AnimationSpec::key`]), which
/// is what turns "the target moved" into "play the transition again".
///
/// Deliberately the *narrow* shape: a duration and a curve, nothing more. The
/// minimum viable set (colour, size, opacity, translation) all interpolate
/// through one scalar `0..=1` progress, so a single type serves every one of
/// them; a property that needs a different shape is out of scope for the
/// implicit layer and belongs to the imperative `WidgetHost::animate` door.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transition {
    pub duration_secs: f64,
    pub easing: Easing,
}

impl Transition {
    pub const fn new(duration_secs: f64) -> Self {
        Self {
            duration_secs,
            easing: Easing::Linear,
        }
    }

    pub const fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }
}

/// Which channel a [`PresenceTransition`] drives.
///
/// One per transition, not a list: a single element carries a single
/// compositor sample (see [`CompositorProp`]), so a fade *and* a slide at once
/// is not something this layer can express. Pick the one the presence reads
/// as; if none of these is it, reach for the imperative door instead.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PresenceChannel {
    /// Opacity. `0 → 1` on the way in.
    #[default]
    Fade,
    /// Uniform scale about the box's centre: `from → 1` on the way in.
    Grow { from: f32 },
    /// Offset along one axis, in points: `distance → 0` on the way in, so a
    /// positive distance slides *up* (or in from the left) into place.
    Slide { axis: SlideAxis, distance: f32 },
}

/// Which axis a [`PresenceChannel::Slide`] travels along.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SlideAxis {
    /// Horizontal, so the content travels along x.
    #[default]
    X,
    /// Vertical, so the content travels along y.
    Y,
}

impl PresenceChannel {
    /// The default start and end scalars of this channel, `(from, to)` for a
    /// transition that is playing *into* presence. Playing out of presence
    /// swaps them, which is what makes one description serve both directions.
    fn endpoints(&self) -> (f64, f64) {
        match *self {
            Self::Fade => (0.0, 1.0),
            Self::Grow { from } => (f64::from(from), 1.0),
            Self::Slide { distance, .. } => (f64::from(distance), 0.0),
        }
    }

    /// The compositor channel this channel runs on.
    ///
    /// The payload is a placeholder: the sampled value replaces it every
    /// frame, so only the variant matters here.
    fn prop(&self) -> CompositorProp {
        match *self {
            Self::Fade => CompositorProp::Opacity(0.0),
            Self::Grow { .. } => CompositorProp::Scale(1.0),
            Self::Slide { axis, .. } => match axis {
                SlideAxis::X => CompositorProp::TranslateX(0.0),
                SlideAxis::Y => CompositorProp::TranslateY(0.0),
            },
        }
    }
}

/// What plays when a subtree appears or disappears.
///
/// The implicit layer ([`Transition`]) answers "this value changed, get there
/// smoothly". Presence answers a different question: *the widget's value did
/// not change, its **existence** did* — there is no old value to animate from,
/// only a before (absent) and an after (present). And "absent" cannot be
/// painted, so leaving has to *defer* it: something that is animating out is
/// still mounted, and unmounts when the motion ends.
///
/// One direction-neutral description: [`Self::spec`] resolves it against the
/// direction being played, so an enter and its exit cannot drift apart in
/// duration or curve — the two things a caller would otherwise restate and
/// then forget to update together.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PresenceTransition {
    pub channel: PresenceChannel,
    pub duration_secs: f64,
    pub easing: Easing,
}

impl PresenceTransition {
    /// A fade over `duration_secs`, linear.
    pub const fn fade(duration_secs: f64) -> Self {
        Self {
            channel: PresenceChannel::Fade,
            duration_secs,
            easing: Easing::Linear,
        }
    }

    /// A grow from `from` (a fraction: `0.9` starts nine tenths of full size)
    /// over `duration_secs`.
    pub const fn grow(duration_secs: f64, from: f32) -> Self {
        Self {
            channel: PresenceChannel::Grow { from },
            duration_secs,
            easing: Easing::Linear,
        }
    }

    /// A slide of `dy` points up into place over `duration_secs`.
    pub const fn slide(duration_secs: f64, dy: f32) -> Self {
        Self {
            channel: PresenceChannel::Slide {
                axis: SlideAxis::Y,
                distance: dy,
            },
            duration_secs,
            easing: Easing::Linear,
        }
    }

    /// A slide of `dx` points in from the left over `duration_secs`.
    ///
    /// The horizontal twin of [`Self::slide`]. A positive `dx` starts the
    /// content that far to the *right* of its resting place and travels left
    /// into it, which is the same sign convention the vertical one uses: a
    /// positive distance is the offset the content starts at.
    pub const fn slide_x(duration_secs: f64, dx: f32) -> Self {
        Self {
            channel: PresenceChannel::Slide {
                axis: SlideAxis::X,
                distance: dx,
            },
            duration_secs,
            easing: Easing::Linear,
        }
    }

    pub const fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Whether there is anything to play. A non-positive duration means "no
    /// transition", which is the caller saying the subtree should simply
    /// appear/vanish — distinct from "animate over zero seconds", a thing the
    /// scheduler cannot express and which would leave the frame loop nothing
    /// to step.
    pub const fn is_instant(&self) -> bool {
        self.duration_secs <= 0.0
    }

    /// The tween to play for `entering` (in) or leaving (out).
    ///
    /// The values are the channel's endpoints, reversed on the way out, so the
    /// same numbers describe the whole round trip.
    pub fn spec(&self, entering: bool) -> AnimationSpec {
        let (from, to) = self.channel.endpoints();
        let (from, to) = if entering { (from, to) } else { (to, from) };
        AnimationSpec::new(from, to, self.duration_secs)
            .with_easing(self.easing)
            .with_prop(self.channel.prop())
    }

    /// The enter tween. See [`Self::spec`].
    pub fn enter_spec(&self) -> AnimationSpec {
        self.spec(true)
    }

    /// The exit tween. See [`Self::spec`].
    pub fn exit_spec(&self) -> AnimationSpec {
        self.spec(false)
    }
}

/// Properties that can be applied to a scene node without re-laying it out.
///
/// The value inside the variant is either the template a scheduler maps its
/// sampled value onto, or the live value a node carries — the same type does
/// both, and which one you are holding is decided by where it came from: a
/// `CompositorProp` in an animation kind is a template, a `CompositorProp`
/// returned by a sampler is a value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompositorProp {
    Opacity(f64),
    TranslateX(f64),
    TranslateY(f64),
    Scale(f64),
    /// Rotation in radians (screen coordinates: positive turns clockwise).
    Rotate(f64),
}

/// One compositor channel of a multi-value arrangement (B6).
///
/// Names a [`CompositorProp`] channel and the `from`/`to` span its value travels
/// over, while all of an [`AnimationSpec`]'s channels share a single curve. The
/// `prop` payload is a *template* — the interpolated value replaces it each
/// frame — so the variant is what names the channel, and `from`/`to` carry the
/// distance to travel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompositorChannel {
    pub prop: CompositorProp,
    pub from: f64,
    pub to: f64,
}

impl CompositorChannel {
    pub const fn new(prop: CompositorProp, from: f64, to: f64) -> Self {
        Self { prop, from, to }
    }
}

/// Cubic-bezier timing curve: solves for the parameter that lands on `x` and
/// returns the corresponding `y`.
///
/// `x1`/`x2` are clamped into `0.0 ..= 1.0` (see [`Easing::CubicBezier`]).
/// Newton's method does the work in the common case; a bisection fallback
/// catches the curves where the derivative vanishes partway, which is exactly
/// where a plain Newton iteration would stall or wander.
pub fn cubic_bezier_at(x1: f64, y1: f64, x2: f64, y2: f64, x: f64) -> f64 {
    let x1 = x1.clamp(0.0, 1.0);
    let x2 = x2.clamp(0.0, 1.0);

    let mut u = x;
    for _ in 0..8 {
        let error = bezier_axis(u, x1, x2) - x;
        if error.abs() < 1e-10 {
            return bezier_axis(u, y1, y2);
        }
        let slope = bezier_axis_slope(u, x1, x2);
        if slope.abs() < 1e-10 {
            break;
        }
        u -= error / slope;
    }

    // The curve is monotone in `u` once the `x` control points are in range,
    // so bracketing converges even where Newton could not.
    let (mut low, mut high) = (0.0, 1.0);
    let mut u = x;
    for _ in 0..64 {
        let current = bezier_axis(u, x1, x2);
        if (current - x).abs() < 1e-12 {
            break;
        }
        if current < x {
            low = u;
        } else {
            high = u;
        }
        u = 0.5 * (low + high);
    }
    bezier_axis(u, y1, y2)
}

/// One axis of a cubic bezier whose endpoints are `0` and `1`.
fn bezier_axis(t: f64, p1: f64, p2: f64) -> f64 {
    let c = 3.0 * p1;
    let b = 3.0 * p2 - 6.0 * p1;
    let a = 1.0 - 3.0 * p2 + 3.0 * p1;
    ((a * t + b) * t + c) * t
}

/// Derivative of [`bezier_axis`] with respect to its parameter.
fn bezier_axis_slope(t: f64, p1: f64, p2: f64) -> f64 {
    let c = 3.0 * p1;
    let b = 3.0 * p2 - 6.0 * p1;
    let a = 1.0 - 3.0 * p2 + 3.0 * p1;
    (3.0 * a * t + 2.0 * b) * t + c
}

/// Analytic response of a viscously damped spring, `t` seconds after being
/// released from `offset` (relative to rest) with `velocity`.
///
/// Returns `(offset, velocity)` at `t`. This is the single implementation
/// behind both [`Easing::Spring`] — a fixed shape stretched over a duration —
/// and the scheduler's spring animation kind, which has a velocity of its own
/// and takes as long as it takes. Keeping one formula means the two cannot
/// drift apart, and it lets a retarget hand the spring the velocity the tween
/// it replaced was carrying.
///
/// The three damping regimes are solved separately because they are genuinely
/// different functions: approximating the overdamped case with the critically
/// damped form (as an earlier version did) leaves the animation visibly wrong
/// whenever damping is more than critical — common for anything meant to settle
/// without bounce.
pub fn damped_spring(
    offset: f64,
    velocity: f64,
    stiffness: f64,
    damping: f64,
    t: f64,
) -> (f64, f64) {
    let omega = stiffness.max(0.0).sqrt();
    if omega <= 0.0 {
        // No restoring force: the offset just drifts at its initial speed.
        return (offset + velocity * t, velocity);
    }
    let zeta = damping / (2.0 * omega);

    if (zeta - 1.0).abs() < 1e-9 {
        // Critically damped: the fastest approach that does not overshoot.
        let a = omega;
        let decay = (-a * t).exp();
        let c = velocity + a * offset;
        let x = decay * (offset + c * t);
        return (x, decay * (c - a * (offset + c * t)));
    }

    if zeta < 1.0 {
        let a = zeta * omega;
        let wd = omega * (1.0 - zeta * zeta).sqrt();
        let decay = (-a * t).exp();
        let (sin, cos) = (wd * t).sin_cos();
        let b = (velocity + a * offset) / wd;
        let x = decay * (offset * cos + b * sin);
        let v = decay * ((b * wd - a * offset) * cos - (offset * wd + a * b) * sin);
        return (x, v);
    }

    // Overdamped: two real modes, and the slow one is what you see in any
    // range worth sampling.
    let a = zeta * omega;
    let r = omega * (zeta * zeta - 1.0).sqrt();
    let (s1, s2) = (-a - r, -a + r);
    let denominator = s1 - s2;
    let c1 = (velocity - s2 * offset) / denominator;
    let c2 = (s1 * offset - velocity) / denominator;
    let (e1, e2) = ((s1 * t).exp(), (s2 * t).exp());
    (c1 * e1 + c2 * e2, s1 * c1 * e1 + s2 * c2 * e2)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sampled densely enough that a wrong branch or a stalled solver shows up.
    fn samples(easing: Easing) -> Vec<f64> {
        (0..=100)
            .map(|i| easing.ease(f64::from(i) / 100.0))
            .collect()
    }

    #[test]
    fn every_curve_lands_exactly_on_its_endpoints() {
        for easing in [
            Easing::Linear,
            Easing::EASE_IN,
            Easing::EASE_OUT,
            Easing::EASE_IN_OUT,
            Easing::Spring {
                stiffness: 200.0,
                damping: 12.0,
            },
            Easing::Spring {
                stiffness: 200.0,
                damping: 60.0,
            },
        ] {
            assert_eq!(easing.ease(0.0), 0.0, "{easing:?} at 0");
            assert_eq!(easing.ease(1.0), 1.0, "{easing:?} at 1");
            // Out of range clamps rather than extrapolating.
            assert_eq!(easing.ease(-0.5), 0.0, "{easing:?} below range");
            assert_eq!(easing.ease(1.5), 1.0, "{easing:?} above range");
        }
    }

    /// A cubic bezier whose control points sit exactly a third and two thirds
    /// along the diagonal *is* the identity: `x(u) == u == y(u)`, so solving
    /// for the parameter is a no-op and the curve must return its input. That
    /// makes the three-quarter points checkable without magic numbers, and it
    /// exercises the solver rather than the fast paths.
    #[test]
    fn the_diagonal_curve_is_the_identity_at_every_sampled_point() {
        let diagonal = Easing::cubic_bezier(1.0 / 3.0, 1.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0);
        for t in [0.25, 0.5, 0.75] {
            let got = diagonal.ease(t);
            assert!(
                (got - t).abs() < 1e-9,
                "diagonal curve at {t} returned {got}"
            );
        }
    }

    /// A curve through `(0.5, 0)` and `(0.5, 1)` is symmetric about the middle
    /// of the unit square, so the two quarter points must add up to one. This
    /// is the point-wise check that catches a solver returning the *y* of the
    /// wrong parameter.
    #[test]
    fn a_symmetric_curve_is_antisymmetric_about_its_middle() {
        let symmetric = Easing::cubic_bezier(0.5, 0.0, 0.5, 1.0);
        let sum = symmetric.ease(0.25) + symmetric.ease(0.75);
        assert!((sum - 1.0).abs() < 1e-9, "quarter points summed to {sum}");
        let middle = symmetric.ease(0.5);
        assert!((middle - 0.5).abs() < 1e-9, "middle was {middle}");
    }

    #[test]
    fn the_named_curves_accelerate_and_decelerate_where_they_claim_to() {
        assert!(
            Easing::EASE_IN.ease(0.25) < 0.25,
            "ease-in must still be behind at a quarter of the way"
        );
        assert!(
            Easing::EASE_OUT.ease(0.25) > 0.25,
            "ease-out must already be ahead at a quarter of the way"
        );
        assert!(
            Easing::EASE_IN_OUT.ease(0.5) - 0.5 < 1e-9,
            "ease-in-out is symmetric, so it is level at the midpoint"
        );
        assert_eq!(Easing::Linear.ease(0.25), 0.25);
        assert_eq!(Easing::Linear.ease(0.75), 0.75);
    }

    /// The curves must be distinguishable — a sampler that quietly ignored its
    /// curve would pass every endpoint assertion above.
    #[test]
    fn different_curves_disagree_in_the_middle() {
        let linear = Easing::Linear.ease(0.25);
        assert!((Easing::EASE_IN.ease(0.25) - linear).abs() > 0.05);
        assert!((Easing::EASE_OUT.ease(0.25) - linear).abs() > 0.05);
        assert!((Easing::EASE_IN.ease(0.75) - Easing::EASE_OUT.ease(0.75)).abs() > 0.05);
    }

    #[test]
    fn travelling_curves_never_go_backwards() {
        for easing in [
            Easing::Linear,
            Easing::EASE_IN,
            Easing::EASE_OUT,
            Easing::EASE_IN_OUT,
        ] {
            let values = samples(easing);
            for pair in values.windows(2) {
                assert!(
                    pair[1] >= pair[0] - 1e-12,
                    "{easing:?} went backwards: {} -> {}",
                    pair[0],
                    pair[1]
                );
            }
        }
    }

    #[test]
    fn an_underdamped_spring_easing_overshoots_and_a_stiff_one_does_not() {
        let bouncy = Easing::Spring {
            stiffness: 200.0,
            damping: 8.0,
        };
        let highest = samples(bouncy).into_iter().fold(f64::MIN, f64::max);
        assert!(
            highest > 1.02,
            "an underdamped spring easing is supposed to overshoot; peak was {highest}"
        );

        let flat = Easing::Spring {
            stiffness: 200.0,
            damping: 80.0,
        };
        let highest = samples(flat).into_iter().fold(f64::MIN, f64::max);
        assert!(
            highest <= 1.0 + 1e-9,
            "a heavily damped spring easing must not overshoot; peak was {highest}"
        );
    }

    /// The spring and the physical spring kind share one solver, so the damped
    /// regimes have to be continuous across the critical boundary — a jump
    /// there is what an approximation would hide.
    #[test]
    fn the_damping_regimes_meet_at_critical_damping() {
        // ω = 10, so ζ = damping / 20 and ζ = 1 sits at damping = 20.
        let just_under = damped_spring(1.0, 0.0, 100.0, 20.0 - 1e-6, 0.1).0;
        let just_over = damped_spring(1.0, 0.0, 100.0, 20.0 + 1e-6, 0.1).0;
        assert!(
            (just_under - just_over).abs() < 1e-6,
            "critical damping is a discontinuity: {just_under} vs {just_over}"
        );
    }

    #[test]
    fn a_spring_with_velocity_carries_it() {
        // Released at rest, the offset can only move toward home; released with
        // velocity away from home it first moves the other way.
        let at_rest = damped_spring(-1.0, 0.0, 100.0, 20.0, 0.01).0;
        assert!(at_rest > -1.0, "at rest it advances toward home");

        let thrown = damped_spring(-1.0, 60.0, 100.0, 20.0, 0.01).0;
        assert!(
            thrown > at_rest,
            "a velocity away from home must carry the offset past the resting case: \
             {thrown} vs {at_rest}"
        );
    }

    #[test]
    fn a_spec_matches_only_an_identical_spec() {
        let spec = AnimationSpec::repeating(0.0, 1.0, 1.2, RepeatSpec::restart(3));
        assert!(spec.matches(&spec));
        assert!(!spec.matches(&AnimationSpec::repeating(
            0.0,
            1.0,
            1.2,
            RepeatSpec::reverse(3)
        )));
        assert!(!spec.matches(&AnimationSpec::repeating(
            0.0,
            1.0,
            1.3,
            RepeatSpec::restart(3)
        )));
    }

    #[test]
    fn repeat_specs_carry_the_iteration_count() {
        assert_eq!(RepeatSpec::restart(3).iterations, Some(3));
        assert_eq!(RepeatSpec::restart(3).mode, RepeatMode::Restart);
        assert_eq!(RepeatSpec::reverse(2).mode, RepeatMode::Reverse);
        assert_eq!(RepeatSpec::INFINITE_REVERSE.iterations, None);
    }

    /// The B6 public builder: `with_channels` lays several channels onto one
    /// shared driver, each free to span its own value range rather than only the
    /// driver's — the path `with_prop`'s one-channel shorthand cannot take.
    #[test]
    fn with_channels_builds_a_multi_channel_spec() {
        let spec = AnimationSpec::new(0.0, 1.0, 0.5).with_channels([
            CompositorChannel::new(CompositorProp::Opacity(0.0), 0.0, 1.0),
            CompositorChannel::new(CompositorProp::TranslateY(0.0), 0.0, 40.0),
            CompositorChannel::new(CompositorProp::Rotate(0.0), -180.0, 180.0),
        ]);
        assert_eq!(spec.channels.len(), 3);
        // Each channel keeps its own span, independent of the driver's 0→1.
        assert_eq!(
            spec.channels,
            vec![
                CompositorChannel::new(CompositorProp::Opacity(0.0), 0.0, 1.0),
                CompositorChannel::new(CompositorProp::TranslateY(0.0), 0.0, 40.0),
                CompositorChannel::new(CompositorProp::Rotate(0.0), -180.0, 180.0),
            ]
        );
        assert_eq!(spec.duration_secs, 0.5, "the driver's own fields survive");
    }

    // --- PresenceTransition -------------------------------------------------

    #[test]
    fn the_default_presence_channel_is_a_fade() {
        let fade = PresenceTransition::fade(0.2);
        assert_eq!(fade.channel, PresenceChannel::Fade);
        assert_eq!(fade.duration_secs, 0.2);
    }

    /// The enter/exit of one transition must differ as data, or the host
    /// cannot tell "appearing" from "disappearing" and would collapse the two
    /// into a single never-restarting declaration. This is the property the
    /// presence widget leans on.
    #[test]
    fn enter_and_exit_are_different_specs_for_the_same_transition() {
        let fade = PresenceTransition::fade(0.2);
        let enter = fade.enter_spec();
        let exit = fade.exit_spec();
        assert_ne!(enter, exit, "enter and exit must be distinguishable");
        assert_eq!(enter.from, 0.0);
        assert_eq!(enter.to, 1.0);
        assert_eq!(exit.from, 1.0);
        assert_eq!(exit.to, 0.0);
        // The channel is the same for both; only the scalars flip.
        assert_eq!(
            enter.channels.first().map(|c| c.prop),
            exit.channels.first().map(|c| c.prop)
        );
        assert_eq!(enter.duration_secs, exit.duration_secs);
    }

    /// A fade drives opacity; the placeholder payload is irrelevant and
    /// identical across directions.
    #[test]
    fn a_fade_rides_the_opacity_channel() {
        let fade = PresenceTransition::fade(0.15);
        match fade.enter_spec().channels.as_slice() {
            [CompositorChannel {
                prop: CompositorProp::Opacity(_),
                ..
            }] => {}
            other => panic!("a fade must drive opacity, got {other:?}"),
        }
    }

    #[test]
    fn grow_and_slide_resolve_to_their_channels_and_endpoints() {
        let grow = PresenceTransition::grow(0.3, 0.9);
        let grow_spec = grow.enter_spec();
        assert!((grow_spec.from - 0.9).abs() < 1e-6);
        assert_eq!(grow_spec.to, 1.0);
        let grow_channel = grow_spec.channels.as_slice();
        assert_eq!(grow_channel.len(), 1, "grow drives one channel");
        assert_eq!(grow_channel[0].prop, CompositorProp::Scale(1.0));
        assert!(
            (grow_channel[0].from - 0.9).abs() < 1e-4 && (grow_channel[0].to - 1.0).abs() < 1e-6,
            "grow spans 0.9 → 1.0: {:?}",
            grow_channel
        );

        let slide = PresenceTransition::slide(0.25, 16.0);
        assert_eq!(slide.enter_spec().from, 16.0);
        assert_eq!(slide.enter_spec().to, 0.0);
        assert_eq!(
            slide.enter_spec().channels,
            vec![CompositorChannel::new(
                CompositorProp::TranslateY(0.0),
                16.0,
                0.0
            )]
        );
        // Sliding out starts from 0 and ends at the offset.
        assert_eq!(slide.exit_spec().from, 0.0);
        assert_eq!(slide.exit_spec().to, 16.0);
    }

    #[test]
    fn a_non_positive_duration_is_instant() {
        assert!(PresenceTransition::fade(0.0).is_instant());
        assert!(PresenceTransition::fade(-0.1).is_instant());
        assert!(!PresenceTransition::fade(0.001).is_instant());
    }

    #[test]
    fn easing_follows_the_transition() {
        let eased = PresenceTransition::fade(0.2).with_easing(Easing::EASE_OUT);
        assert_eq!(eased.enter_spec().easing, Easing::EASE_OUT);
        assert_eq!(eased.exit_spec().easing, Easing::EASE_OUT);
    }

    #[test]
    fn default_presence_channel_is_a_fade() {
        assert_eq!(PresenceTransition::fade(0.2).channel, PresenceChannel::Fade);
        let unspecified = PresenceTransition {
            channel: PresenceChannel::Fade,
            duration_secs: 0.2,
            easing: Easing::Linear,
        };
        assert_eq!(unspecified.channel, PresenceChannel::default());
    }

    #[test]
    fn a_fade_enters_from_invisible_and_exits_to_it() {
        let t = PresenceTransition::fade(0.2).with_easing(Easing::EASE_OUT);
        let enter = t.enter_spec();
        let exit = t.exit_spec();
        // Opacity runs 0 → 1 coming in and 1 → 0 going out.
        assert_eq!((enter.from, enter.to), (0.0, 1.0));
        assert_eq!((exit.from, exit.to), (1.0, 0.0));
        assert_eq!(
            enter.channels.first().map(|c| c.prop),
            Some(CompositorProp::Opacity(0.0))
        );
        assert_eq!(
            exit.channels.first().map(|c| c.prop),
            Some(CompositorProp::Opacity(0.0))
        );
        assert_eq!(enter.duration_secs, 0.2);
        assert_eq!(enter.easing, Easing::EASE_OUT);
        // The two directions must not be the same motion, or the host would
        // never restart the transition when the subtree flips back.
        assert!(!enter.matches(&exit), "enter and exit must differ");
    }

    #[test]
    fn a_grow_enters_at_the_start_scale_and_returns_to_it() {
        let t = PresenceTransition::grow(0.2, 0.5);
        let enter = t.enter_spec();
        let exit = t.exit_spec();
        assert_eq!((enter.from, enter.to), (0.5, 1.0));
        assert_eq!((exit.from, exit.to), (1.0, 0.5));
        assert_eq!(
            enter.channels.first().map(|c| c.prop),
            Some(CompositorProp::Scale(1.0))
        );
        assert!(!enter.matches(&exit), "enter and exit must differ");
    }

    #[test]
    fn a_slide_enters_from_the_offset_and_returns_to_it() {
        let t = PresenceTransition::slide(0.2, 16.0);
        let enter = t.enter_spec();
        let exit = t.exit_spec();
        assert_eq!((enter.from, enter.to), (16.0, 0.0));
        assert_eq!((exit.from, exit.to), (0.0, 16.0));
        assert_eq!(
            enter.channels.first().map(|c| c.prop),
            Some(CompositorProp::TranslateY(0.0))
        );
        assert!(!enter.matches(&exit), "enter and exit must differ");
    }

    #[test]
    fn an_instant_presence_is_no_transition_at_all() {
        assert!(PresenceTransition::fade(0.0).is_instant());
        assert!(PresenceTransition::fade(-1.0).is_instant());
        assert!(!PresenceTransition::fade(0.001).is_instant());
    }
}
