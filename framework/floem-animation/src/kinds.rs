//! Tween / spring / keyframe kinds + compositor-only props.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnimId(pub u32);

/// Properties that can run on the compositor without layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompositorProp {
    Opacity(f64),
    TranslateX(f64),
    TranslateY(f64),
    Scale(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tween {
    pub from: f64,
    pub to: f64,
    pub duration_secs: f64,
    pub compositor: Option<CompositorProp>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnimKind {
    Tween(Tween),
    Spring {
        from: f64,
        to: f64,
        stiffness: f64,
        damping: f64,
        compositor: Option<CompositorProp>,
    },
    Keyframe {
        keys: Vec<(f64, f64)>, // (time_secs, value)
        compositor: Option<CompositorProp>,
    },
}

impl AnimKind {
    /// Sample at local time `t` seconds since start. Returns (value, compositor, finished).
    pub fn sample(&self, t: f64) -> (f64, Option<CompositorProp>, bool) {
        match self {
            Self::Tween(tw) => {
                let d = tw.duration_secs.max(1e-9);
                let u = (t / d).clamp(0.0, 1.0);
                let v = tw.from + (tw.to - tw.from) * u;
                let comp = map_compositor(tw.compositor, v);
                (v, comp, u >= 1.0)
            }
            Self::Spring {
                from,
                to,
                stiffness,
                damping,
                compositor,
            } => {
                // Critically-damped-ish analytic approximation for tests.
                let omega = stiffness.sqrt();
                let zeta = damping / (2.0 * omega.max(1e-9));
                let x0 = from - to;
                let decay = (-zeta * omega * t).exp();
                let x = if zeta < 1.0 {
                    let wd = omega * (1.0 - zeta * zeta).max(0.0).sqrt();
                    decay * (x0 * (wd * t).cos())
                } else {
                    decay * x0
                };
                let v = to + x;
                let done = t > 2.0 || x.abs() < 1e-3;
                (v, map_compositor(*compositor, v), done)
            }
            Self::Keyframe { keys, compositor } => {
                if keys.is_empty() {
                    return (0.0, *compositor, true);
                }
                if t <= keys[0].0 {
                    let v = keys[0].1;
                    return (v, map_compositor(*compositor, v), false);
                }
                for w in keys.windows(2) {
                    let (t0, v0) = w[0];
                    let (t1, v1) = w[1];
                    if t <= t1 {
                        let u = if (t1 - t0).abs() < 1e-12 {
                            1.0
                        } else {
                            (t - t0) / (t1 - t0)
                        };
                        let v = v0 + (v1 - v0) * u;
                        return (v, map_compositor(*compositor, v), false);
                    }
                }
                let v = keys.last().unwrap().1;
                (v, map_compositor(*compositor, v), true)
            }
        }
    }
}

fn map_compositor(template: Option<CompositorProp>, v: f64) -> Option<CompositorProp> {
    template.map(|c| match c {
        CompositorProp::Opacity(_) => CompositorProp::Opacity(v),
        CompositorProp::TranslateX(_) => CompositorProp::TranslateX(v),
        CompositorProp::TranslateY(_) => CompositorProp::TranslateY(v),
        CompositorProp::Scale(_) => CompositorProp::Scale(v),
    })
}
