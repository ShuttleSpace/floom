//! The stage profiler (Phase 23.3).
//!
//! Instruments the pipeline stages the plan names — input, reactive, layout,
//! paint, scene, render, native — with scoped spans, a frame timeline, explicit
//! allocation counters and cache statistics. The timeline's wire format *is*
//! the inspection protocol's [`PerformanceSnapshot`]: one definition, so a
//! devtools frontend never re-implements it.
//!
//! Two honest boundaries:
//!
//! - **Allocation counters are API-driven, not a global allocator hook.** A
//!   global hook counts the whole process — the profiler itself, the host's
//!   other subsystems, and every `Vec` the framework owns for reasons unrelated
//!   to the frame — and it changes global behaviour in release builds. A host
//!   that wants stage-scoped numbers calls [`Profiler::record_alloc`] at its
//!   own allocation points.
//! - **Spans may nest, and the outer span's duration includes the inner
//!   one's.** A frame's wall clock is not the sum of its spans when stages
//!   nest (layout typically contains render work). Closed spans are appended
//!   as they close, so inner spans appear before their outer span.
//!
//! `Profiler` is a cheap handle (`Clone`); spans are guarded by value, which
//! is what makes nested spans possible at all.

#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::rc::Rc;

use floem_inspector::{
    AllocSummary, CacheStats, FrameSummary, JsonSnapshot, PerformanceSnapshot, ProtocolVersion,
    StageSpan,
};

/// A pipeline stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    Input,
    Reactive,
    Layout,
    Paint,
    Scene,
    Render,
    Native,
}

impl Stage {
    /// The protocol's spelling of the stage name.
    pub fn name(self) -> &'static str {
        match self {
            Stage::Input => "input",
            Stage::Reactive => "reactive",
            Stage::Layout => "layout",
            Stage::Paint => "paint",
            Stage::Scene => "scene",
            Stage::Render => "render",
            Stage::Native => "native",
        }
    }
}

type Clock = Rc<dyn Fn() -> u64>;

/// Nanoseconds, from a real clock by default.
fn wall_clock() -> Clock {
    Rc::new(|| std::time::Instant::now().elapsed().as_nanos() as u64)
}

struct OpenSpan {
    stage: Stage,
    started_ns: u64,
}

struct Inner {
    budget_ns: u64,
    clock: Clock,
    now: u64,
    open: Vec<OpenSpan>,
    closed: Vec<StageSpan>,
    frames: Vec<FrameSummary>,
    frame_start: Option<u64>,
    frame_index: u64,
    allocations: AllocSummary,
    caches: Vec<CacheTracker>,
}

impl Inner {
    fn read_clock(&mut self) -> u64 {
        self.now = (self.clock)();
        self.now
    }
}

/// The profiler. A cheap handle; clone it to hand spans to call sites that do
/// not have the original borrow.
pub struct Profiler {
    inner: Rc<RefCell<Inner>>,
}

impl Clone for Profiler {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new(16_666) // 60 Hz
    }
}

impl Profiler {
    pub fn new(budget_ns: u64) -> Self {
        let clock = wall_clock();
        Self::with_clock(budget_ns, move || clock())
    }

    /// A profiler over an injected clock — how the tests get exact durations.
    pub fn with_clock(budget_ns: u64, clock: impl Fn() -> u64 + 'static) -> Self {
        // The clock is not read here: `begin_frame` reads it first, and a
        // constructor-side read would silently shift every scripted duration.
        let clock = Rc::new(clock);
        Self {
            inner: Rc::new(RefCell::new(Inner {
                budget_ns,
                clock,
                now: 0,
                open: Vec::new(),
                closed: Vec::new(),
                frames: Vec::new(),
                frame_start: None,
                frame_index: 0,
                allocations: AllocSummary::default(),
                caches: Vec::new(),
            })),
        }
    }

    /// Start a frame. The frame's wall clock starts here.
    pub fn begin_frame(&self) {
        let mut inner = self.inner.borrow_mut();
        assert!(inner.frame_start.is_none(), "begin_frame inside a frame");
        inner.closed.clear();
        inner.frame_start = Some(inner.read_clock());
    }

    /// Open a stage span. The span closes when the guard drops; guards may
    /// nest, and the outer span's duration includes the inner one's.
    pub fn stage(&self, stage: Stage) -> StageGuard {
        let started_ns = {
            let mut inner = self.inner.borrow_mut();
            inner.read_clock()
        };
        self.inner
            .borrow_mut()
            .open
            .push(OpenSpan { stage, started_ns });
        StageGuard {
            profiler: self.clone(),
            active: true,
        }
    }

    /// End the frame and append its summary to the timeline. Any span still
    /// open is closed here (dropped guards are the normal path; this is the
    /// defensive one).
    pub fn end_frame(&self) -> Option<FrameSummary> {
        let mut inner = self.inner.borrow_mut();
        let start = inner.frame_start.take()?;
        while let Some(span) = inner.open.pop() {
            let duration_ns = inner.read_clock().saturating_sub(span.started_ns);
            inner.closed.push(StageSpan {
                stage: span.stage.name().to_string(),
                duration_ns,
            });
        }
        let total = inner.now.saturating_sub(start);
        let frame = FrameSummary {
            index: inner.frame_index,
            budget_exceeded: total > inner.budget_ns,
            stages: std::mem::take(&mut inner.closed),
        };
        inner.frame_index += 1;
        inner.frames.push(frame.clone());
        Some(frame)
    }

    /// The accumulated timeline.
    pub fn timeline(&self) -> Vec<FrameSummary> {
        self.inner.borrow().frames.clone()
    }

    /// Count one allocation of `bytes` (host-driven; see the crate docs).
    pub fn record_alloc(&self, bytes: u64) {
        let mut inner = self.inner.borrow_mut();
        inner.allocations.allocations += 1;
        inner.allocations.bytes_allocated += bytes;
    }

    /// Count one deallocation. `bytes_allocated` stays cumulative — it is the
    /// total the framework has ever allocated, not the live set.
    pub fn record_dealloc(&self, _bytes: u64) {
        self.inner.borrow_mut().allocations.deallocations += 1;
    }

    pub fn allocations(&self) -> AllocSummary {
        self.inner.borrow().allocations
    }

    /// Register a named cache so its statistics ride along in the snapshot.
    /// The returned handle is the same object the snapshot reads.
    pub fn track_cache(&self, name: &str) -> CacheTracker {
        let tracker = CacheTracker::new(name);
        self.inner.borrow_mut().caches.push(tracker.clone());
        tracker
    }

    /// The whole timeline as the protocol's performance snapshot.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        self.snapshot().to_json()
    }

    /// The structured snapshot, for hosts that post-process before serving.
    pub fn snapshot(&self) -> PerformanceSnapshot {
        let inner = self.inner.borrow();
        PerformanceSnapshot {
            version: ProtocolVersion::current(),
            frames: inner.frames.clone(),
            allocations: inner.allocations,
            caches: inner.caches.iter().map(CacheTracker::stats).collect(),
        }
    }
}

/// Closes its span on drop. Holds a handle, not a borrow, so spans nest.
pub struct StageGuard {
    profiler: Profiler,
    active: bool,
}

impl Drop for StageGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        self.active = false;
        let mut inner = self.profiler.inner.borrow_mut();
        // The last open span is ours: guards drop inside-out by construction.
        let Some(span) = inner.open.pop() else {
            return;
        };
        let duration_ns = inner.read_clock().saturating_sub(span.started_ns);
        inner.closed.push(StageSpan {
            stage: span.stage.name().to_string(),
            duration_ns,
        });
    }
}

/// One named cache's statistics, maintained by the host.
#[derive(Debug, Clone)]
pub struct CacheTracker {
    inner: Rc<RefCell<CacheStats>>,
}

impl CacheTracker {
    pub fn new(name: &str) -> Self {
        Self {
            inner: Rc::new(RefCell::new(CacheStats {
                name: name.to_string(),
                entries: 0,
                hits: 0,
                misses: 0,
                bytes: 0,
            })),
        }
    }

    pub fn hit(&self) {
        self.inner.borrow_mut().hits += 1;
    }

    pub fn miss(&self) {
        self.inner.borrow_mut().misses += 1;
    }

    /// The host reports its own occupancy: the tracker cannot know how many
    /// entries a cache holds without a second implementation of that cache.
    pub fn set_entries(&self, entries: u64, bytes: u64) {
        let mut stats = self.inner.borrow_mut();
        stats.entries = entries;
        stats.bytes = bytes;
    }

    pub fn stats(&self) -> CacheStats {
        self.inner.borrow().clone()
    }
}
