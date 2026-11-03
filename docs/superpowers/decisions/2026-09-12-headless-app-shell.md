# Decision: `crates/floom-app` — a headless app shell (closing the "nothing runs" gap)

- **Date:** 2026-09-12
- **Plan:** `docs/superpowers/plans/2026-09-02-floom-2-complete-implementation-plan.md`
- **Related:** audit note in `.superpowers/sdd/progress-floom-2.md` ("Reality gaps")

## Gap (not in the plan)

Phases 0–13 produced every layer *except* something to hold them together:
`src/` (Floom 1.x) was the only code that could run, the floom-2 crates had no
app shell, and every `examples/*` targeted the legacy root crate. Widgets added
in Phase 14 could therefore only be unit-tested, never actually run.

The plan has no task for this. It is the single highest-value missing piece
because without it every later phase ships unverifiable code.

## Decision

Add a new crate `crates/floom-app` holding `WidgetHost`: a **headless** app shell.

- `mount_root` / `mount` build a widget tree; each node gets an `ElementId`
  from `floom-runtime-core::IdAllocator` and a taffy node from
  `floom-layout`.
- `frame()` runs update → layout → paint → semantics, in that order.
- `dispatch(Event)` hit-tests through `floom-runtime::EventRouter`
  (capture → target → bubble) and stops at the first widget that consumes it.
  Key events go to the focused element instead of a hit-test target.
- Focus is owned by `floom-focus::FocusManager`; pointer presses call
  `request_focus`, and the host pushes the resulting focused flag back into
  the controls.
- Paint output lands in a retained `floom_scene::Scene`; semantics land in a
  `floom_accessibility::SemanticsTree`.

It sits **above** `floom-runtime` / layout / scene / framework / widgets and
depends only downward, so no dependency rule is violated. "Headless" is the
point: no window, no GPU — a real backend later supplies a surface and pumps
`frame()`. That makes the whole thing verifiable on this machine today.

`examples/floom-counter` is the first runnable floom-2 app, and
`crates/floom-app/tests/counter.rs` is the first test that drives a whole app.

## Two real bugs the shell surfaced

1. **Hit-testing was order-dependent.** Every region was registered with `z=0`,
   and `HitTester` keeps the first match at equal `z` — over a `HashMap`, i.e.
   arbitrary. A full-viewport root would randomly win against a button inside
   it. The host now assigns `z = depth * 1_000_000 + mount_index`, so deeper
   nodes win and later siblings beat earlier ones.
2. **The retained scene grew every frame.** Painting allocated fresh scene
   nodes without retiring the previous frame's. The host now tracks the ids it
   produced and removes them at the start of each frame (`scene_does_not_grow_across_frames`
   guards it).

## Consequence: how roles reach the semantics tree

`SemanticsContext` carries only a label, and `floom-core` must not depend on
`floom-accessibility` (that would invert the framework's layering). Rather than
push the problem to a later phase, `floom_widgets::WidgetAny` — the trait the
host already stores — exposes `semantics_hint()`. The host uses it to publish
real roles and states (`CHECKED`, `DISABLED`, `FOCUSABLE`…) into the retained
semantics tree. This is a deliberate, documented seam: if `SemanticsRole` ever
moves down into `floom-core`, `WidgetAny::semantics_hint` can be deleted.
