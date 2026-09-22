//! Deterministic animation scheduler with a pluggable clock.

mod clock;
mod kinds;

use kinds::map_compositor;

pub use clock::{Clock, FakeClock};
pub use floem_types::{
    AnimationSpec, CompositorProp, Easing, PresenceChannel, PresenceTransition, RepeatMode,
    RepeatSpec, Transition,
};
pub use kinds::{AnimId, AnimKind, SPRING_SETTLE_SECS, Tween};

/// Stiffness a retarget hands the spring it converts the animation into.
pub const RETARGET_STIFFNESS: f64 = 200.0;

/// Damping for the same spring.
///
/// A hair under critical (ζ ≈ 0.92 at [`RETARGET_STIFFNESS`]): the redirected
/// animation settles without a visible bounce, but it is a spring rather than a
/// lerp, so the momentum it was carrying goes somewhere.
pub const RETARGET_DAMPING: f64 = 26.0;

/// Per-tick output applied to the visual tree / compositor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimSample {
    pub id: AnimId,
    pub value: f64,
    /// Rate of change of `value`, in value-units per second.
    ///
    /// Published rather than kept internal because it is what a gesture hands
    /// over: a fling that ends while the element is still moving has to tell
    /// whatever picks the motion up how fast it was going, and the only place
    /// that number exists is here.
    pub velocity: f64,
    pub compositor: Option<CompositorProp>,
    pub finished: bool,
}

#[derive(Debug)]
struct Running {
    id: AnimId,
    kind: AnimKind,
    start_secs: f64,
}

/// Frame-driven animation scheduler.
///
/// Sampled in the order animations were started, not in hash order: a
/// "deterministic scheduler" whose output order depends on a `HashMap`'s
/// iteration order is deterministic only until someone changes the hasher.
#[derive(Debug, Default)]
pub struct AnimationScheduler {
    next_id: u32,
    running: Vec<Running>,
    /// Indices into `running` that finished during the current tick. A field
    /// rather than a local so sampling an animation costs no allocation.
    finished: Vec<usize>,
}

impl AnimationScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&mut self, kind: AnimKind, now_secs: f64) -> AnimId {
        let id = AnimId(self.next_id);
        self.next_id += 1;
        self.running.push(Running {
            id,
            kind,
            start_secs: now_secs,
        });
        id
    }

    pub fn cancel(&mut self, id: AnimId) {
        self.running.retain(|run| run.id != id);
    }

    /// Cancels everything, so an embedder can drop a frame's worth of animation
    /// without naming each one.
    pub fn cancel_all(&mut self) {
        self.running.clear();
    }

    pub fn len(&self) -> usize {
        self.running.len()
    }

    pub fn is_empty(&self) -> bool {
        self.running.is_empty()
    }

    /// Redirects a running animation toward `to`, keeping the value *and* the
    /// velocity it has right now.
    ///
    /// A tween cannot simply be pointed somewhere else: its value is a function
    /// of `(from, to, duration)`, so moving the target mid-flight either jumps
    /// the value (restarting the clock) or jumps the speed (squashing the
    /// remaining time). The retarget therefore re-seeds the animation as a
    /// spring carrying the sampled velocity, which is what lets a gesture that
    /// hands off mid-flight keep its momentum.
    ///
    /// Timed motion and interrupted motion are different shapes of the same
    /// thing, and this is the seam between them: what comes out has a target and
    /// a velocity, and is over when it settles rather than when a clock says so.
    ///
    /// Returns whether `id` named a running animation.
    pub fn retarget(&mut self, id: AnimId, to: f64, now_secs: f64) -> bool {
        let Some(run) = self.running.iter_mut().find(|run| run.id == id) else {
            return false;
        };
        let elapsed = (now_secs - run.start_secs).max(0.0);
        let (value, velocity) = run.kind.value_velocity(elapsed);
        run.kind = AnimKind::Spring {
            from: value,
            to,
            velocity,
            stiffness: RETARGET_STIFFNESS,
            damping: RETARGET_DAMPING,
            compositor: run.kind.compositor(),
        };
        run.start_secs = now_secs;
        true
    }

    /// Tick all animations at `now_secs` (platform frame clock).
    ///
    /// Allocates the returned vector every call; a frame loop should use
    /// [`AnimationScheduler::tick_into`] with a buffer it keeps.
    pub fn tick(&mut self, now_secs: f64) -> Vec<AnimSample> {
        let mut out = Vec::new();
        self.tick_into(now_secs, &mut out);
        out
    }

    /// Ticks into `out`, clearing it first.
    ///
    /// The allocation-free form, and the one the frame loop uses: an animation
    /// that runs for a minute has hundreds of chances to be the reason a
    /// steady-state frame stopped being allocation-free, and every one of them
    /// is avoidable.
    pub fn tick_into(&mut self, now_secs: f64, out: &mut Vec<AnimSample>) {
        out.clear();
        self.finished.clear();
        for (index, run) in self.running.iter().enumerate() {
            let t = (now_secs - run.start_secs).max(0.0);
            let (value, velocity, done) = run.kind.evaluate(t);
            out.push(AnimSample {
                id: run.id,
                value,
                velocity,
                compositor: map_compositor(run.kind.compositor(), value),
                finished: done,
            });
            if done {
                self.finished.push(index);
            }
        }
        // Back to front, so each removal leaves the indices still waiting to be
        // removed pointing at the elements they named.
        while let Some(index) = self.finished.pop() {
            self.running.remove(index);
        }
    }
}
