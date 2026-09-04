//! Deterministic animation scheduler with a pluggable clock.

mod clock;
mod kinds;

pub use clock::{Clock, FakeClock};
pub use kinds::{AnimId, AnimKind, CompositorProp, Tween};

use std::collections::HashMap;

/// Per-tick output applied to the visual tree / compositor.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimSample {
    pub id: AnimId,
    pub value: f64,
    pub compositor: Option<CompositorProp>,
    pub finished: bool,
}

#[derive(Debug)]
struct Running {
    kind: AnimKind,
    start_secs: f64,
}

/// Frame-driven animation scheduler.
#[derive(Debug, Default)]
pub struct AnimationScheduler {
    next_id: u32,
    running: HashMap<AnimId, Running>,
}

impl AnimationScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&mut self, kind: AnimKind, now_secs: f64) -> AnimId {
        let id = AnimId(self.next_id);
        self.next_id += 1;
        self.running.insert(
            id,
            Running {
                kind,
                start_secs: now_secs,
            },
        );
        id
    }

    pub fn cancel(&mut self, id: AnimId) {
        self.running.remove(&id);
    }

    pub fn len(&self) -> usize {
        self.running.len()
    }

    pub fn is_empty(&self) -> bool {
        self.running.is_empty()
    }

    /// Tick all animations at `now_secs` (platform frame clock).
    pub fn tick(&mut self, now_secs: f64) -> Vec<AnimSample> {
        let mut out = Vec::new();
        let mut finished = Vec::new();
        for (&id, run) in &self.running {
            let t = (now_secs - run.start_secs).max(0.0);
            let (value, compositor, done) = run.kind.sample(t);
            out.push(AnimSample {
                id,
                value,
                compositor,
                finished: done,
            });
            if done {
                finished.push(id);
            }
        }
        for id in finished {
            self.running.remove(&id);
        }
        out
    }
}
