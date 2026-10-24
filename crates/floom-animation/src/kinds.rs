//! Tween / spring / keyframe / repeat kinds + compositor-only props.

// The vocabulary itself lives in `floom-types`, so that layers which cannot
// depend on this crate (`floom-core`, where a widget declares what it wants)
// can still name it. The crate root re-exports all of it.
use floom_types::{CompositorChannel, CompositorProp, Easing, RepeatMode, damped_spring};
use smallvec::SmallVec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnimId(pub u32);

#[derive(Debug, Clone, PartialEq)]
pub struct Tween {
    pub from: f64,
    pub to: f64,
    pub duration_secs: f64,
    /// The shape of the journey from `from` to `to`.
    ///
    /// [`Easing::Linear`] reproduces this framework's original behaviour
    /// exactly, which is why it is the default: adding a curve is opt-in.
    pub easing: Easing,
    /// The compositor channels this tween drives, each interpolating its own
    /// `from`→`to` against the tween's shared curve (B6 multi-value).
    pub channels: Vec<CompositorChannel>,
}

impl Tween {
    pub fn new(from: f64, to: f64, duration_secs: f64) -> Self {
        Self {
            from,
            to,
            duration_secs,
            easing: Easing::Linear,
            channels: Vec::new(),
        }
    }

    pub fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Drives one compositor channel over this tween's own value span.
    pub fn with_compositor(mut self, prop: CompositorProp) -> Self {
        self.channels.push(CompositorChannel {
            prop,
            from: self.from,
            to: self.to,
        });
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnimKind {
    Tween(Tween),
    Spring {
        from: f64,
        to: f64,
        /// Speed at the moment the spring is seeded, in value-units per second.
        ///
        /// This is what lets a fling hand its momentum to the spring that
        /// settles it, and what a retarget carries over from the animation it
        /// replaced. Zero is a spring released from rest.
        velocity: f64,
        stiffness: f64,
        damping: f64,
        channels: Vec<CompositorChannel>,
    },
    Keyframe {
        keys: Vec<(f64, f64)>, // (time_secs, value)
        channels: Vec<CompositorChannel>,
    },
    /// Plays `inner` over and over, optionally a bounded number of times.
    ///
    /// The cycle is `inner`'s own span (see [`AnimKind::span_secs`]); an
    /// inner kind with no span — a spring, which ends when it settles rather
    /// than when a clock says so — cannot be repeated, and is played once
    /// instead. That is a documented degradation rather than a silent one: a
    /// spring inside a repeat is almost always a mistake in the caller, but
    /// inventing a period for it would be a worse one.
    Repeat {
        inner: Box<AnimKind>,
        /// `None` repeats forever — the animation then never reports finished,
        /// which is what keeps the frame loop awake for a spinner.
        iterations: Option<u32>,
        mode: RepeatMode,
    },
}

impl AnimKind {
    /// Sample at local time `t` seconds since start. Returns (value, compositor, finished).
    pub fn sample(&self, t: f64) -> (f64, SmallVec<[CompositorProp; 2]>, bool) {
        let (value, _velocity, finished) = self.evaluate(t);
        (value, self.sample_compositors(value), finished)
    }

    /// Sample value *and* velocity.
    ///
    /// A retarget needs both: the value is where the animation carries on from,
    /// and the velocity is what it carries on *with*.
    pub fn value_velocity(&self, t: f64) -> (f64, f64) {
        let (value, velocity, _) = self.evaluate(t);
        (value, velocity)
    }

    /// The compositor values this kind holds at a given driver `value` (B6).
    ///
    /// A tween interpolates each channel over its own `from`→`to` against the
    /// tween's shared progress; a spring or keyframe maps the raw value onto
    /// every channel it carries, since those kinds have no value span of their
    /// own. Empty means the kind drives nothing on its own.
    pub fn sample_compositors(&self, value: f64) -> SmallVec<[CompositorProp; 2]> {
        match self {
            Self::Tween(tw) => {
                let progress = if (tw.to - tw.from).abs() > f64::EPSILON {
                    ((value - tw.from) / (tw.to - tw.from)).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                tw.channels
                    .iter()
                    .map(|c| map_one(c.prop, c.from + progress * (c.to - c.from)))
                    .collect()
            }
            Self::Spring { channels, .. } | Self::Keyframe { channels, .. } => {
                channels.iter().map(|c| map_one(c.prop, value)).collect()
            }
            Self::Repeat { inner, .. } => inner.sample_compositors(value),
        }
    }

    /// The compositor channels this kind drives.
    ///
    /// A repeat drives whatever its inner kind drives — the channel belongs to
    /// the motion, not to the fact that it happens more than once.
    pub fn channels(&self) -> Vec<CompositorChannel> {
        match self {
            Self::Tween(tw) => tw.channels.clone(),
            Self::Spring { channels, .. } => channels.clone(),
            Self::Keyframe { channels, .. } => channels.clone(),
            Self::Repeat { inner, .. } => inner.channels(),
        }
    }

    /// The *first* compositor channel this kind drives, if any — the single
    /// named channel a single-prop caller asked for.
    pub fn compositor(&self) -> Option<CompositorProp> {
        self.channels().first().map(|c| c.prop)
    }

    /// How long one pass of this kind lasts, where that is knowable.
    ///
    /// For a repeat, one pass is its inner kind's span times its iteration
    /// count — which is what makes a repeat nestable: an outer repeat cycles
    /// over the whole inner animation, not over one of its cycles.
    ///
    /// `None` means "not knowable in advance": a spring is over when it
    /// settles rather than when a clock says so, and an endless repeat never
    /// ends at all.
    pub fn span_secs(&self) -> Option<f64> {
        match self {
            Self::Tween(tw) => Some(tw.duration_secs.max(MIN_CYCLE_SECS)),
            Self::Keyframe { keys, .. } => keys.last().map(|(t, _)| t.max(MIN_CYCLE_SECS)),
            Self::Spring { .. } => None,
            Self::Repeat {
                inner,
                iterations,
                mode,
            } => {
                let cycle = inner.span_secs()?;
                let period = match mode {
                    RepeatMode::Restart => cycle,
                    RepeatMode::Reverse => cycle * 2.0,
                };
                iterations.map(|n| period * f64::from(n))
            }
        }
    }

    pub(crate) fn evaluate(&self, t: f64) -> (f64, f64, bool) {
        match self {
            Self::Tween(tw) => {
                let d = tw.duration_secs.max(1e-9);
                let u = (t / d).clamp(0.0, 1.0);
                let eased = tw.easing.ease(u);
                let v = tw.from + (tw.to - tw.from) * eased;
                // The velocity of a curved tween is its curve's slope times the
                // distance over the duration. A retarget hands this to a
                // spring, so a tween interrupted at speed does not stop dead.
                // A central difference keeps the slope meaningful where the
                // curve flattens at its ends, which is exactly where a forward
                // difference loses its precision.
                let low = (u - SLOPE_STEP).max(0.0);
                let high = (u + SLOPE_STEP).min(1.0);
                let slope = if high > low {
                    (tw.easing.ease(high) - tw.easing.ease(low)) / (high - low)
                } else {
                    0.0
                };
                let velocity = slope * (tw.to - tw.from) / d;
                (v, velocity, u >= 1.0)
            }
            Self::Spring {
                from,
                to,
                velocity,
                stiffness,
                damping,
                ..
            } => {
                let (offset, speed) = damped_spring(from - to, *velocity, *stiffness, *damping, t);
                let done = t > SPRING_SETTLE_SECS || (offset.abs() < 1e-3 && speed.abs() < 1e-2);
                (to + offset, speed, done)
            }
            Self::Keyframe { keys, .. } => {
                if keys.is_empty() {
                    return (0.0, 0.0, true);
                }
                if t <= keys[0].0 {
                    return (keys[0].1, 0.0, false);
                }
                for w in keys.windows(2) {
                    let (t0, v0) = w[0];
                    let (t1, v1) = w[1];
                    if t <= t1 {
                        let span = t1 - t0;
                        let u = if span.abs() < 1e-12 {
                            1.0
                        } else {
                            (t - t0) / span
                        };
                        let v = v0 + (v1 - v0) * u;
                        let velocity = if span.abs() < 1e-12 {
                            0.0
                        } else {
                            (v1 - v0) / span
                        };
                        return (v, velocity, false);
                    }
                }
                (keys.last().unwrap().1, 0.0, true)
            }
            Self::Repeat {
                inner,
                iterations,
                mode,
            } => {
                let Some(cycle) = inner.span_secs().map(|c| c.max(MIN_CYCLE_SECS)) else {
                    return inner.evaluate(t);
                };
                let period = match mode {
                    RepeatMode::Restart => cycle,
                    RepeatMode::Reverse => cycle * 2.0,
                };
                let total = iterations.map(|n| period * f64::from(n));
                if total == Some(0.0) {
                    // Zero iterations is a finished animation that never left
                    // its start value, not a division by a zero-length run.
                    let (value, velocity, _) = inner.evaluate(0.0);
                    return (value, velocity, true);
                }
                let finished = total.is_some_and(|total| t >= total);
                let elapsed = total.map_or(t, |total| t.min(total));
                let mut phase = elapsed % period;
                if finished && *mode == RepeatMode::Restart {
                    // A completed restart stopped on the cycle's *end*, which
                    // wrapping to zero would otherwise report as its start.
                    phase = cycle;
                }
                let (inner_t, backwards) = match mode {
                    RepeatMode::Restart => (phase, false),
                    RepeatMode::Reverse => {
                        if phase <= cycle {
                            (phase, false)
                        } else {
                            (period - phase, true)
                        }
                    }
                };
                let (value, velocity, _) = inner.evaluate(inner_t);
                // Going backwards means the velocity points backwards too.
                (
                    value,
                    if backwards { -velocity } else { velocity },
                    finished,
                )
            }
        }
    }
}

/// Lifts a widget's declaration into the scheduler's vocabulary.
///
/// The one place the declarative spec and the scheduler's kind space meet, so
/// the mapping between them exists once: a declaration is a tween, optionally
/// wrapped in a repeat, and nothing else — the parameters a spring or a keyframe
/// needs (velocity, key tracks) are ones a widget has no business inventing on
/// its own element.
impl From<&floom_types::AnimationSpec> for AnimKind {
    fn from(spec: &floom_types::AnimationSpec) -> Self {
        let tween = Tween {
            from: spec.from,
            to: spec.to,
            duration_secs: spec.duration_secs,
            easing: spec.easing,
            channels: spec.channels.clone(),
        };
        match spec.repeat {
            None => Self::Tween(tween),
            Some(repeat) => Self::Repeat {
                inner: Box::new(Self::Tween(tween)),
                iterations: repeat.iterations,
                mode: repeat.mode,
            },
        }
    }
}

/// Step for the central difference that recovers a tween's slope.
///
/// Large enough that the bezier solver's own tolerance does not dominate the
/// difference, small enough that the curve is still locally linear over it.
const SLOPE_STEP: f64 = 1e-4;

/// A spring is declared over when it has been running this long regardless of
/// how it looks — the backstop that keeps a stiff or near-undamped spring from
/// holding the frame loop open forever.
pub const SPRING_SETTLE_SECS: f64 = 2.0;

/// Shortest cycle a repeat will accept, so a zero-length tween cannot turn the
/// phase arithmetic into a division by zero.
const MIN_CYCLE_SECS: f64 = 1e-9;

pub(crate) fn map_one(prop: CompositorProp, v: f64) -> CompositorProp {
    match prop {
        CompositorProp::Opacity(_) => CompositorProp::Opacity(v),
        CompositorProp::TranslateX(_) => CompositorProp::TranslateX(v),
        CompositorProp::TranslateY(_) => CompositorProp::TranslateY(v),
        CompositorProp::Scale(_) => CompositorProp::Scale(v),
        CompositorProp::Rotate(_) => CompositorProp::Rotate(v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tween(from: f64, to: f64, duration: f64, easing: Easing) -> AnimKind {
        AnimKind::Tween(Tween {
            from,
            to,
            duration_secs: duration,
            easing,
            channels: Vec::new(),
        })
    }

    /// A retarget carries a tween's speed forward, so the sampled velocity has
    /// to be the real slope — not a constant, and not zero.
    #[test]
    fn a_tweens_sampled_velocity_is_its_actual_slope() {
        let linear = tween(0.0, 100.0, 1.0, Easing::Linear);
        assert!((linear.value_velocity(0.5).1 - 100.0).abs() < 0.1);

        let curved = tween(0.0, 100.0, 1.0, Easing::EASE_IN);
        let start = curved.value_velocity(0.05).1;
        let end = curved.value_velocity(0.95).1;
        assert!(
            end > start * 3.0,
            "an ease-in tween must be moving much faster at the end: {start} -> {end}"
        );
    }

    #[test]
    fn a_spring_with_initial_velocity_moves_against_its_offset() {
        // Released at the origin travelling toward the target: a spring seeded
        // with zero velocity would only ever move toward the target, so the
        // overshoot is only reachable with the velocity carrying it.
        let carried = AnimKind::Spring {
            from: 0.0,
            to: 10.0,
            velocity: 40.0,
            stiffness: 100.0,
            damping: 10.0,
            channels: Vec::new(),
        };
        let peak = (0..200)
            .map(|i| carried.sample(f64::from(i) / 100.0).0)
            .fold(f64::MIN, f64::max);
        assert!(
            peak > 10.5,
            "carried momentum must overshoot the target; peak was {peak}"
        );
    }

    #[test]
    fn a_repeat_of_a_kind_with_no_span_plays_it_once() {
        // Documented degradation: a spring has no span, so a repeat around it
        // runs the spring and finishes when the spring does.
        let repeated = AnimKind::Repeat {
            inner: Box::new(AnimKind::Spring {
                from: 0.0,
                to: 1.0,
                velocity: 0.0,
                stiffness: 100.0,
                damping: 20.0,
                channels: Vec::new(),
            }),
            iterations: None,
            mode: RepeatMode::Restart,
        };
        assert!(repeated.span_secs().is_none());
        let (value, _, finished) = repeated.sample(3.0);
        assert!(finished, "a spring inside a repeat still ends");
        assert!(
            (value - 1.0).abs() < 0.01,
            "and ends on the spring's target"
        );
    }

    /// The declarative spec and the scheduler's kind space have to agree. A
    /// widget that declared a repeating, eased, compositor-driving tween and
    /// got a default one instead would still animate — it would just animate
    /// the wrong thing, which no "did it move" assertion can see.
    #[test]
    fn a_declared_spec_becomes_the_kind_it_describes() {
        use floom_types::{AnimationSpec, RepeatSpec};

        let spec = AnimationSpec::repeating(0.25, 1.0, 0.8, RepeatSpec::reverse(4))
            .with_easing(Easing::EASE_OUT)
            .with_prop(CompositorProp::Opacity(0.0));
        let kind = AnimKind::from(&spec);

        assert!(matches!(
            kind,
            AnimKind::Repeat {
                iterations: Some(4),
                mode: RepeatMode::Reverse,
                ..
            }
        ));
        assert!(matches!(
            kind.compositor(),
            Some(CompositorProp::Opacity(_))
        ));
        // One pass is the inner span times the iteration count: 0.8 s, four
        // times over.
        assert_eq!(kind.span_secs(), Some(0.8 * 2.0 * 4.0));

        // Sampled the way the host samples it: a quarter into the first cycle
        // is a quarter of the eased way from 0.25 to 1.0.
        let (value, _, _) = kind.sample(0.2);
        let expected = 0.25 + 0.75 * Easing::EASE_OUT.ease(0.25);
        assert!(
            (value - expected).abs() < 1e-9,
            "declared 0.25 -> 1.0 eased; got {value}, expected {expected}"
        );
    }

    /// Three channels spill past the two-slot inline buffer, and every one of
    /// them still interpolates over *its own* from→to against the shared driver,
    /// not the driver's range. Rotate is folded through too, so a non-opacity,
    /// non-translate prop does not fall out of the sample.
    #[test]
    fn three_channels_each_sample_over_their_own_span() {
        let tween = AnimKind::Tween(Tween {
            from: 0.0,
            to: 1.0,
            duration_secs: 1.0,
            easing: Easing::Linear,
            channels: vec![
                CompositorChannel::new(CompositorProp::Opacity(0.0), 0.0, 1.0),
                CompositorChannel::new(CompositorProp::TranslateX(0.0), 0.0, 100.0),
                CompositorChannel::new(CompositorProp::Rotate(0.0), -180.0, 180.0),
            ],
        });

        let (_, comps, _) = tween.sample(0.5);
        // Three entries: past the two-slot inline capacity, so the small vec has
        // had to spill to the heap rather than silently truncating.
        assert_eq!(comps.len(), 3, "all three channels survive: {comps:?}");
        // Each is interpolated against its own span — not the driver's 0→1.
        assert!(
            matches!(
                comps.as_slice(),
                [CompositorProp::Opacity(o), CompositorProp::TranslateX(x), CompositorProp::Rotate(r)]
                if (*o - 0.5).abs() < 1e-6
                && (*x - 50.0).abs() < 1e-6
                && (*r - 0.0).abs() < 1e-6
            ),
            "channels hit 50% of each own span: {comps:?}"
        );

        // The driver's own value remains independent of the channels.
        let (value, _, _) = tween.sample(0.5);
        assert!(
            (value - 0.5).abs() < 1e-9,
            "driver value is untouched: {value}"
        );
    }
}
