# Decision: No Legacy View Adapter

**Date:** 2026-09-04  
**Status:** Accepted  
**Branch:** `floom-2.0`

## Context

The Floom 2.0 complete implementation plan (Phase 8) requires wrapping existing `View` / `ViewId` APIs behind a compatibility adapter so current applications keep working during migration.

Project rule for this fork: do not keep backward compatibility; remove obsolete APIs instead of shims or migration layers.

## Decision

**Policy A — no legacy adapter.**

- Skip / delete Phase 8 tasks (`Task 8.1`, `Task 8.2`) from the execution checklist.
- New retained runtime (`ElementId`, Element Tree, Scene, Renderer/Platform ABI) is the only runtime path.
- Existing `View` / `ViewId` public surface may be deleted or replaced without a long-lived compatibility shim when the new runtime lands.
- P0 acceptance item “Legacy adapter runs representative existing apps” is **waived**; replace with “representative demos rebuilt (or deleted) against the new runtime before claiming P0 done.”

## Consequences

- Faster architecture cutover; less dual-stack maintenance.
- Existing Floom apps on this fork will break at the cutover commit; callers must migrate or pin an older revision.
- Plan Global Constraint “Preserve current Floom applications through a compatibility layer…” is **overridden** by this decision for this repository.
- Document this override in AGENT / progress notes when executing later phases.

## Alternatives considered

- **Policy B (plan default):** keep adapter until a deliberate major-version removal — rejected for this fork.
