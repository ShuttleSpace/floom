# Decision: crates/floom-native introduced as a Phase 12.2 precursor

- **Date:** 2026-09-07
- **Plan:** `docs/superpowers/plans/2026-09-02-floom-2-complete-implementation-plan.md`
- **Task:** 12.2 (Native view pooling)

## Conflict

Task 12.2 prescribes creating `crates/floom-native/src/pool.rs` (the native view
pool that virtualization depends on), but the crate's `Cargo.toml` **and**
`crates/floom/src/lib.rs` are only specified later, in Phase 16 Task 16.1 ("Define Native
ABI"). As written, Phase 12 cannot be completed without a crate the plan
defines two phases later — a genuine ordering bug.

## Decision

Introduce `crates/floom-native` **now** as a minimal precursor that contains
only the pooling primitive (`pool.rs`: `NativeSlot`, `NativeViewPool`), depending
solely on `floom-types` (for `NativeViewId`) and `floom-virtual`. The full
Native ABI traits (`NativeViewProvider`, `NativeViewController`,
`NativeCapabilities`, `NativeCompositionMode`) remain Phase 16 deliverables and
will be added to this same crate's `lib.rs` in Task 16.1.

This keeps Phase 12 self-contained and lets virtualization exercise native view
recycling immediately, while honoring the plan's intent that a native pool
exists for virtualized rows.

## Consequences

- Phase 16 Task 16.1's "Create: crates/floom-native/Cargo.toml" and
  "crates/floom/src/lib.rs" become "Extend" rather than "Create". No structural conflict.
- The dependency edge `floom-native → floom-virtual` is new but permitted:
  native composition legitimately consumes the virtualization primitive, and it
  crosses no forbidden architecture boundary (scene↛platform, types↛platform/GPU,
  widgets↛concrete-renderer, renderer↛widgets). Verified by
  `scripts/check-architecture-deps.sh` (PASS, no new forbidden edges).
- `NativeViewPool` is keyed by a caller-supplied stable key `K`; it mints
  `NativeViewId`s and recycles a bounded free list. Platform backends attach
  their concrete native handle to a `NativeSlot` and observe `generation`
  bumps to detect recycled slots.
