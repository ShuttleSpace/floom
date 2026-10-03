//! Single-producer / single-consumer scene handoff for a render thread.

use floem_scene::Scene;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// Immutable snapshot handed to the render thread.
#[derive(Debug, Clone)]
pub struct SceneSnapshot {
    pub scene: Scene,
    pub frame_number: u64,
    pub enqueued_at: Instant,
}

/// Instrumentation for queue wait / frame latency.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameHandoffStats {
    pub enqueued: u64,
    pub dequeued: u64,
    pub dropped_oldest: u64,
    pub total_wait_ns: u64,
    pub total_latency_ns: u64,
}

/// Bounded SPSC-style queue (mutex + condvar; safe across threads).
#[derive(Clone)]
pub struct SceneFrameQueue {
    inner: Arc<Inner>,
}

struct Inner {
    capacity: usize,
    state: Mutex<State>,
    not_empty: Condvar,
}

struct State {
    queue: VecDeque<SceneSnapshot>,
    stats: FrameHandoffStats,
    closed: bool,
}

impl SceneFrameQueue {
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            inner: Arc::new(Inner {
                capacity,
                state: Mutex::new(State {
                    queue: VecDeque::with_capacity(capacity),
                    stats: FrameHandoffStats::default(),
                    closed: false,
                }),
                not_empty: Condvar::new(),
            }),
        }
    }

    pub fn stats(&self) -> FrameHandoffStats {
        self.inner.state.lock().unwrap().stats.clone()
    }

    /// Producer: push a scene snapshot. If full, drop the oldest frame.
    pub fn enqueue(&self, scene: Scene, frame_number: u64) {
        let mut guard = self.inner.state.lock().unwrap();
        if guard.closed {
            return;
        }
        if guard.queue.len() >= self.inner.capacity {
            guard.queue.pop_front();
            guard.stats.dropped_oldest += 1;
        }
        guard.queue.push_back(SceneSnapshot {
            scene,
            frame_number,
            enqueued_at: Instant::now(),
        });
        guard.stats.enqueued += 1;
        self.inner.not_empty.notify_one();
    }

    /// Consumer: pop next snapshot, waiting up to `timeout`.
    pub fn dequeue_timeout(&self, timeout: Duration) -> Option<SceneSnapshot> {
        let mut guard = self.inner.state.lock().unwrap();
        let start = Instant::now();
        while guard.queue.is_empty() && !guard.closed {
            let (g, result) = self
                .inner
                .not_empty
                .wait_timeout(guard, timeout.saturating_sub(start.elapsed()))
                .unwrap();
            guard = g;
            if result.timed_out() {
                break;
            }
        }
        let wait_ns = start.elapsed().as_nanos() as u64;
        guard.stats.total_wait_ns = guard.stats.total_wait_ns.saturating_add(wait_ns);
        let snap = guard.queue.pop_front()?;
        guard.stats.dequeued += 1;
        let latency_ns = snap.enqueued_at.elapsed().as_nanos() as u64;
        guard.stats.total_latency_ns = guard.stats.total_latency_ns.saturating_add(latency_ns);
        Some(snap)
    }

    pub fn try_dequeue(&self) -> Option<SceneSnapshot> {
        let mut guard = self.inner.state.lock().unwrap();
        let snap = guard.queue.pop_front()?;
        guard.stats.dequeued += 1;
        let latency_ns = snap.enqueued_at.elapsed().as_nanos() as u64;
        guard.stats.total_latency_ns = guard.stats.total_latency_ns.saturating_add(latency_ns);
        Some(snap)
    }

    pub fn close(&self) {
        let mut guard = self.inner.state.lock().unwrap();
        guard.closed = true;
        self.inner.not_empty.notify_all();
    }
}
