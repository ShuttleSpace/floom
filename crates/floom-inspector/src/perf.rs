//! The performance snapshot.

use serde::{Deserialize, Serialize};

use crate::{JsonSnapshot, ProtocolVersion};

/// One instrumented stage inside one frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageSpan {
    /// `"input"`, `"reactive"`, `"layout"`, `"paint"`, `"scene"`, `"render"`,
    /// `"native"`.
    pub stage: String,
    pub duration_ns: u64,
}

/// One frame's stage breakdown, with the budget verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameSummary {
    pub index: u64,
    pub budget_exceeded: bool,
    pub stages: Vec<StageSpan>,
}

/// Explicit allocation counters (see `floem-profiler` for why they are
/// API-driven rather than a global allocator hook).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AllocSummary {
    pub allocations: u64,
    pub deallocations: u64,
    pub bytes_allocated: u64,
}

impl AllocSummary {
    /// Live allocations: negative means the counters were fed out of order.
    pub fn live(&self) -> i64 {
        self.allocations as i64 - self.deallocations as i64
    }
}

/// One named cache's statistics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheStats {
    pub name: String,
    pub entries: u64,
    pub hits: u64,
    pub misses: u64,
    pub bytes: u64,
}

impl CacheStats {
    /// Hit rate over the accesses actually recorded; `None` when idle, so an
    /// empty cache is not reported as a 100% hit rate.
    pub fn hit_rate(&self) -> Option<f64> {
        let total = self.hits + self.misses;
        if total == 0 {
            return None;
        }
        Some(self.hits as f64 / total as f64)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    pub version: ProtocolVersion,
    pub frames: Vec<FrameSummary>,
    pub allocations: AllocSummary,
    pub caches: Vec<CacheStats>,
}

impl JsonSnapshot for PerformanceSnapshot {}
