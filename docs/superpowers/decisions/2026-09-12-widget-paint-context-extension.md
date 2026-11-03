# Decision: extend `floom-core` paint/semantics contracts for Phase 14.1

- **Date:** 2026-09-12
- **Plan:** `docs/superpowers/plans/2026-09-02-floom-2-complete-implementation-plan.md`
- **Task:** 14.1 (Primitive widgets)

## Conflict

Task 14.1 only lists `crates/floom-widgets/src/primitives.rs` as a new file,
but the `Widget` contract it must implement cannot express a single Tier-0
primitive:

- `PaintContext::fill_rect(rect)` had **no colour parameter**, so `Divider`
  (a coloured rule) and `Container` (a background) were inexpressible.
- There was no text op and no image op, so `Text`, `Icon` and `Image` had
  nothing to emit.
- `SemanticsContext` had `set_bounds` but no accessor for the element bounds,
  so a widget could not report bounds for the node it describes.

## Decision

Extend `crates/floom-core/src/context.rs` in place:

- `PaintContext::fill_rect(rect, color)` — rect ops now carry colour.
- `PaintContext::fill_rounded_rect(rect, radius, color)` — **default impl**
  delegates to `fill_rect`, so backends without a rounded-rect primitive need
  no change.
- `PaintContext::fill_text(origin, text, size, color)` and
  `draw_image(bounds, image)` — **default no-op impls**; shaped text stays in
  `floom-text`, this op carries the unshaped string for glyph-run interning.
- `SemanticsContext::bounds()` — accessor so a widget can report the bounds of
  the node it describes.

`ResourceId` is re-exported from `floom-core` so widget crates need not depend
on `floom-types` merely for the image handle.

This is safe because `floom-core` had **no other consumer in the workspace**
(verified: no `floom_core` / `floom-core` reference outside the crate itself
and the new widgets crate), so no implementation is broken by the change.

## Consequence: roles do not flow through `SemanticsContext` yet

`SemanticsContext` still carries only a label. Primitives therefore expose a
`SemanticsHint { role, label }` (role taken from
`floom_accessibility::SemanticsRole`) and publish the label through the context;
the role is consumed by the runtime's semantics bridge when it lowers widgets
into `floom_accessibility::SemanticsTree`. Extending `SemanticsContext` with
role/state/action is deferred to that bridge task rather than done here, to
keep this change reviewable.

## Consequences

- `crates/floom-core/tests/contracts.rs` updated for the new signatures.
- `scripts/check-architecture-deps.sh` was silently skipping the new widgets
  crate: `WIDGET_HINTS` only contained the legacy underscore spelling
  `floom_widgets`. Fixed to cover `floom-widgets` (plus future
  `floom-widgets-data` / `-media`), to print an OK line when a crate *is*
  checked, and to also forbid widget → platform/windowing dependencies.
- Snapshot tests use inline textual snapshots produced by
  `floom_widgets::testkit` instead of adding an external snapshot crate; the
  paint recorder additionally lowers every op into a real `floom_scene::Scene`
  so the retained-scene path is exercised, not just the op list.
