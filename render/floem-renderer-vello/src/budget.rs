//! GPU cache budgets and memory-pressure trimming policy.
//!
//! The presenter holds three kinds of retained GPU-adjacent state, each of
//! which can grow with the scene it is showing:
//!
//! * **geometry** — the per-node encode cache (`SceneFragment`s): cached vello
//!   command lists, rebuilt from the scene on demand;
//! * **resources** — resident image textures (registry-backed brushes and
//!   imported external frames), which hold real pixel bytes;
//! * **raster** — the partial-damage scratch texture, sized to the target.
//!
//! This module is the *policy*: what the budgets are per platform class, and
//! what a trim at a given memory pressure drops. It is deliberately free of
//! wgpu types so it can be unit-tested without a GPU adapter (CI machines and
//! headless builds have none); the presenter ([`crate::gpu`] with the `gpu`
//! feature) carries the mechanism and enforces the policy.

/// Which class of device the budget is sized for. The defaults trade cache
/// memory against re-encode/re-upload work: a desktop GPU can keep a whole
/// screen's worth of geometry resident, an embedded one cannot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformClass {
    Desktop,
    Mobile,
    Web,
    Embedded,
}

impl PlatformClass {
    /// The class of the current build target.
    ///
    /// Desktop here means the winit-backed OSes this workspace ships real
    /// windows on; anything else errs on the side of smaller caches, which is
    /// the safe direction — a too-small budget costs re-encoding, a too-large
    /// one costs memory the device does not have.
    pub fn current() -> Self {
        match std::env::consts::FAMILY {
            "unix" | "windows" => Self::Desktop,
            _ => Self::Web,
        }
    }
}

/// Retained-state budgets, in per-category units: fragment cache entries for
/// geometry, pixel bytes for resources, pixel bytes for the scratch texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuCacheBudget {
    /// Maximum cached node encode fragments. Entries are pure derived state
    /// (re-encodable from the scene), so enforcement drops caching rather
    /// than dropping draws.
    pub max_fragments: u32,
    /// Maximum resident image bytes across registry-backed brushes and
    /// imported external textures.
    pub max_resource_bytes: u64,
    /// Maximum scratch (partial-rasterisation) texture bytes. A target larger
    /// than this falls back to full-frame rasterisation instead of growing
    /// the scratch.
    pub max_scratch_bytes: u64,
}

impl GpuCacheBudget {
    /// Defaults per platform class. Sized so a plausible worst-case screen
    /// fits without trimming, with the smallest classes accepting re-encode
    /// and re-upload churn instead of unbounded retention.
    pub fn for_platform(class: PlatformClass) -> Self {
        match class {
            // A 4K desktop frame is ~33 MB; 256 MB of images and a full-screen
            // scratch leaves room for a photo-heavy screen without pressure.
            PlatformClass::Desktop => Self {
                max_fragments: 8_192,
                max_resource_bytes: 256 * 1024 * 1024,
                max_scratch_bytes: 64 * 1024 * 1024,
            },
            PlatformClass::Mobile => Self {
                max_fragments: 2_048,
                max_resource_bytes: 64 * 1024 * 1024,
                max_scratch_bytes: 16 * 1024 * 1024,
            },
            PlatformClass::Web => Self {
                max_fragments: 1_024,
                max_resource_bytes: 32 * 1024 * 1024,
                max_scratch_bytes: 8 * 1024 * 1024,
            },
            PlatformClass::Embedded => Self {
                max_fragments: 256,
                max_resource_bytes: 8 * 1024 * 1024,
                max_scratch_bytes: 2 * 1024 * 1024,
            },
        }
    }

    /// The budget for the current build target.
    pub fn for_current_platform() -> Self {
        Self::for_platform(PlatformClass::current())
    }

    /// Whether a new fragment may be cached given the current entry count.
    ///
    /// The scene node itself is always drawn — enforcement limits *retention*,
    /// never rendering.
    pub fn may_cache_fragment(&self, current_fragments: usize) -> bool {
        (current_fragments as u64) < u64::from(self.max_fragments)
    }

    /// Whether an image of `new_bytes` may be *retained* given the current
    /// resident bytes. Drawing proceeds either way; a refused brush is built
    /// for this frame and dropped after it.
    pub fn may_cache_resource(&self, current_bytes: u64, new_bytes: u64) -> bool {
        current_bytes.saturating_add(new_bytes) <= self.max_resource_bytes
    }
}

/// How hard the OS (or the embedder) says memory is squeezed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPressure {
    /// Housekeeping level: nothing is dropped proactively.
    Normal,
    /// The system is under pressure: drop everything that is cheap to
    /// reconstruct and expensive to keep — cached geometry re-encodes from the
    /// scene, so it goes first.
    Moderate,
    /// The system is about to evict us: drop everything trimmable, including
    /// resident textures (which cost re-uploads) and the scratch texture.
    Critical,
}

/// What a trim drops. Pure derived state first, resident pixels last — the
/// ordering is the whole point of having levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrimPlan {
    pub drop_fragments: bool,
    pub drop_resources: bool,
    pub release_scratch: bool,
}

impl TrimPlan {
    /// Nothing is dropped (the plan for [`MemoryPressure::Normal`]).
    pub const NOTHING: Self = Self {
        drop_fragments: false,
        drop_resources: false,
        release_scratch: false,
    };

    /// Everything trimmable (the plan for [`MemoryPressure::Critical`]).
    pub const EVERYTHING: Self = Self {
        drop_fragments: true,
        drop_resources: true,
        release_scratch: true,
    };
}

/// Decides what a trim at `pressure` drops. Usage is consulted so a level
/// never claims to have freed anything the cache did not hold — a Moderate
/// trim with an empty geometry cache still reports `drop_fragments`, but the
/// telemetry counts what actually went away.
pub fn plan_trim(
    _budget: &GpuCacheBudget,
    fragments: usize,
    resource_bytes: u64,
    scratch_bytes: u64,
    pressure: MemoryPressure,
) -> TrimPlan {
    match pressure {
        MemoryPressure::Normal => TrimPlan::NOTHING,
        MemoryPressure::Moderate => TrimPlan {
            drop_fragments: fragments > 0,
            drop_resources: false,
            release_scratch: false,
        },
        MemoryPressure::Critical => TrimPlan {
            drop_fragments: fragments > 0,
            drop_resources: resource_bytes > 0,
            release_scratch: scratch_bytes > 0,
        },
    }
}

/// Cache-trimming telemetry. Counted in every build (tests and the profiler
/// read it); surfaced as a log line in dev builds only — release builds pay
/// the counters, not the printing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CacheTrimTelemetry {
    /// Trims that dropped at least one thing.
    pub trims: u64,
    /// Encode fragments dropped by trims or budget enforcement.
    pub fragments_dropped: u64,
    /// Resident image brushes dropped by trims.
    pub resources_dropped: u64,
    /// Caching requests refused because a budget was already full.
    pub caching_refused: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Budgets must shrink with the device class — a phone is not a desktop —
    /// and every category must shrink together. Ordering is the contract;
    /// the absolute numbers are tuning, not semantics.
    #[test]
    fn budgets_shrink_with_device_class() {
        let desktop = GpuCacheBudget::for_platform(PlatformClass::Desktop);
        let mobile = GpuCacheBudget::for_platform(PlatformClass::Mobile);
        let web = GpuCacheBudget::for_platform(PlatformClass::Web);
        let embedded = GpuCacheBudget::for_platform(PlatformClass::Embedded);

        for (big, small) in [(desktop, mobile), (mobile, web), (web, embedded)] {
            assert!(big.max_fragments > small.max_fragments);
            assert!(big.max_resource_bytes > small.max_resource_bytes);
            assert!(big.max_scratch_bytes > small.max_scratch_bytes);
        }
        assert!(PlatformClass::current() != PlatformClass::Embedded);
    }

    /// Fragment caching is capped by entry count and refused at the limit —
    /// but only for retention, never for drawing.
    #[test]
    fn fragment_caching_stops_at_the_cap() {
        let budget = GpuCacheBudget {
            max_fragments: 3,
            max_resource_bytes: 1 << 20,
            max_scratch_bytes: 1 << 20,
        };
        assert!(budget.may_cache_fragment(0));
        assert!(budget.may_cache_fragment(2));
        assert!(!budget.may_cache_fragment(3), "cap is inclusive");
    }

    /// Resource caching is capped in bytes, and a brush that would not fit is
    /// refused without breaking the accounting for later, smaller images.
    #[test]
    fn resource_caching_stops_at_the_byte_cap() {
        let budget = GpuCacheBudget {
            max_fragments: 16,
            max_resource_bytes: 1024,
            max_scratch_bytes: 1 << 20,
        };
        assert!(budget.may_cache_resource(0, 768));
        assert!(!budget.may_cache_resource(768, 768), "would exceed the cap");
        assert!(
            budget.may_cache_resource(768, 256),
            "an exactly-fitting image is admitted"
        );
        assert!(!budget.may_cache_resource(1024, 1));
    }

    /// Trim levels drop progressively: geometry first (pure derived state),
    /// resident pixels last (they cost re-uploads), and a Normal trim drops
    /// nothing at all.
    #[test]
    fn trim_levels_drop_progressively() {
        let budget = GpuCacheBudget::for_platform(PlatformClass::Desktop);
        let usage = (100usize, 1_000u64, 2_000u64);

        assert_eq!(
            plan_trim(&budget, usage.0, usage.1, usage.2, MemoryPressure::Normal),
            TrimPlan::NOTHING,
            "a normal-pressure trim is a no-op"
        );

        let moderate = plan_trim(&budget, usage.0, usage.1, usage.2, MemoryPressure::Moderate);
        assert!(moderate.drop_fragments, "geometry goes first");
        assert!(!moderate.drop_resources, "textures survive a moderate trim");
        assert!(!moderate.release_scratch);

        let critical = plan_trim(&budget, usage.0, usage.1, usage.2, MemoryPressure::Critical);
        assert_eq!(critical, TrimPlan::EVERYTHING);
    }

    /// A trim of an already-empty cache reports a plan that drops nothing —
    /// the levels only claim what is actually held.
    #[test]
    fn trimming_an_empty_cache_drops_nothing() {
        let budget = GpuCacheBudget::for_platform(PlatformClass::Desktop);
        let critical = plan_trim(&budget, 0, 0, 0, MemoryPressure::Critical);
        assert_eq!(
            critical,
            TrimPlan {
                drop_fragments: false,
                drop_resources: false,
                release_scratch: false,
            },
            "nothing held means nothing to drop, even at Critical"
        );
    }
}
