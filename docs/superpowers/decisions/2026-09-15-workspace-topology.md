# Decision: the workspace topology — where the tree follows spec §5 and where it does not

- **Date:** 2026-09-15
- **Spec:** `docs/superpowers/specs/2026-09-02-floem-2-complete-architecture.md` §5 (Workspace and Crate Topology), §6 (Dependency Direction Rules)
- **Plan:** `docs/superpowers/plans/2026-09-02-floem-2-complete-implementation-plan.md`
- **Status update (2026-09-17):** the rows below whose decision was forced by a
  legacy-crate *name collision* (`render/` vs the legacy `renderer/`, and
  `tooling/floem-testkit` vs the legacy `test/`), and the *naming* half of the
  `reactive/` gap, are no longer collisions — the v1 crates were relocated under
  `legacy/` with a `floem-legacy-` prefix. The rows are left as written to record
  what was decided on 2026-09-15; see the **Addendum** at the end for the current
  state.

## The question

§5 gives a "recommended final topology" and then says, in the same breath, that
"the exact directory layout may be adapted to the existing Floem repository; the
dependency directions below are normative." The tree has been adapted for two
years without anyone writing down which differences are deliberate and which
are drift, so "is this layout right?" has had no answer. This is that answer,
per difference.

## Decision: directories adapt, dependency directions are enforced

Every difference below is either **adopted** (the tree moved to the spec) or
**kept** (the tree stays, and the spec is read as permitting it). What is *not*
negotiated is §6: the dependency directions are now checked mechanically by
`scripts/check-architecture-deps.sh`, which grew a layer table (L0 types → L8
tooling) and fails on any edge from a lower layer to a higher one.

| §5 says | the tree has | decision |
| --- | --- | --- |
| `foundation/{floem-types,floem-core,floem-macros}` | `crates/{floem-types,floem-core}` | **Adopted.** Renamed to `foundation/` on 2026-09-15; `floem-macros` does not exist yet (no proc-macro crate has been needed) |
| `renderer/…` | `render/…` | **Kept.** The legacy crate `renderer/` (package `floem_renderer`) occupies that name at the repo root; two directories differing only in a trailing `r` is worse than the rename. Recorded in the ledger's decision list since 2026-09-04 |
| `reactive/{floem-reactive,floem-reactive-runtime}` | `reactive/` is the **legacy** crate; no 2.0 reactive crate | **Kept, with a gap.** Cannot be created under that name, and is not merely a naming problem: `floem-runtime`, `floem-runtime-core`, `floem-store` and `floem-task` depend on the legacy `floem_reactive`. The script reports this as a NOTE, not a failure — closing it means giving 2.0 its own reactive layer |
| `scene/floem-paint`, `renderer/floem-renderer-wgpu`, `runtime/floem-runtime-dev` | absent | Not built. Listed here so their absence is a decision rather than an oversight |
| `platform/floem-platform-{macos,windows,linux}` | one `platform/floem-platform-desktop` | **Kept.** winit gives one desktop backend; three crates would be three copies of the same event-loop mapping. Split them when a platform needs its own crate (native menus, Win32-specific composition) — D4 in the Phase 16 plan is the first candidate |
| `framework/floem-widget` | `widgets/floem-widgets` | **Kept.** The spec's own §5 lists `widgets/floem-widgets` too; the singular `framework/floem-widget` is the inconsistency |
| `framework/floem-i18n` | absent | Not built (Phase 21) |
| — | `framework/floem-{focus,store,task,services,error}` | **Kept.** Not in §5's list, but each is a framework concern the plan's phases created; the topology predates them. `floem-accessibility` sits at L3 in the layer table rather than with the rest of `framework/`, because platform backends depend on it to publish semantics |
| (not listed at all) | `app/floem-app` | **Spec gap, filled.** §5 has no app layer even though nothing else can host a tree; the shell was added by decision on 2026-09-12 and is the top of the layering (L7) |
| `tooling/floem-test` | `tooling/floem-testkit` | **Kept.** The legacy `test/` crate is *named* `floem-test`; Cargo rejects two workspace members with one name. Renaming the v1 crate is out of scope for this track |
| `native/{webview,video,camera,map}`, `design-systems/*`, `compiler/*`, `widgets/{data,media}` | absent | Phase 17–19+; the arch script already SKIPs them by name |

## Two bugs that had made the boundary check vacuous

Wiring the layer table up revealed that `check-architecture-deps.sh` had not
been checking the 2.0 crates at all, and had been reporting PASS regardless:

1. **Name spelling.** Cargo's resolve graph spells dependencies the way the lib
   target is spelled — `floem_core`, with underscores — while the script's
   rule sets use hyphens. Every intersection was empty, so every rule passed.
   All names are now normalised.
2. **Shadowed package names.** The resolve graph contains a `floem-editor`
   from a sibling checkout outside this repo, and the script took "the first
   node whose name matches" — which was not ours. Nodes are now resolved among
   `workspace_members` first.

Both are fixed, and the check was re-validated by mutating the layer table
(moving `floem-app` to L0) and confirming it fails with a precise message. A
gate that cannot fail is worse than no gate, because it is taken as evidence.

## Consequences

- `crates/` no longer exists; every path dependency and the CI job's package
  list were updated mechanically (`crates/floem-` → `foundation/floem-`).
- The layer table in the script is the authoritative answer to "may crate A
  depend on crate B?" — not §5's picture. Adding a 2.0 crate means adding it to
  that table.
- Dev-dependencies are excluded from the layering rule: `floem-app`
  dev-depending on `floem-testkit`, which depends on `floem-app`, is the shape
  of an integration harness, not an upward edge.
- Crates with no consumers are not an error, but they are now visible: from the
  2.0 tree, `floem-{gesture,persistence,store,task,style,theme,native}` and the
  non-desktop platform backends have no inbound edges. Several are backends
  chosen at runtime (expected); the framework ones are the Phase-15 glue gap.

## Addendum (2026-09-17): the legacy-crate collisions are resolved

The two rows above whose decision was *forced by a name collision* are now moot,
and so is the *naming* half of the `reactive/` gap. Following the
[EpicGames/lore](https://github.com/EpicGames/lore) workspace model, every v1
crate moved under `legacy/` and took a `floem-legacy-` package prefix, and the
`ui-events` bridge moved under `ui-events/`:

| was (path) | now (path) | package name → import identifier |
| --- | --- | --- |
| `renderer/` | `legacy/floem-renderer/` | `floem_renderer` → `floem-legacy-renderer` (`floem_legacy_renderer`) |
| `vello/` | `legacy/floem-renderer-vello/` | `floem_vello_renderer` → `floem-legacy-renderer-vello` |
| `skia/` | `legacy/floem-renderer-skia/` | `floem_skia_renderer` → `floem-legacy-renderer-skia` |
| `vger/` | `legacy/floem-renderer-vger/` | `floem_vger_renderer` → `floem-legacy-renderer-vger` |
| `tiny_skia/` | `legacy/floem-renderer-tiny-skia/` | `floem_tiny_skia_renderer` → `floem-legacy-renderer-tiny-skia` |
| `reactive/` | `legacy/floem-reactive/` | `floem_reactive` → `floem-legacy-reactive` |
| `test/` | `legacy/floem-test/` | `floem_test` → `floem-legacy-test` |
| `ui-events-winit/` | `ui-events/floem-winit/` | `ui-events-floem-winit` → `floem-ui-events-winit` (`floem_ui_events_winit`) |

How that settles the three rows above:

- **`render/…` vs `renderer/`** — the collision is gone. The v1 renderer is now
  `legacy/floem-renderer` (package `floem-legacy-renderer`); the 2.0 ABI keeps
  `render/floem-renderer` (package `floem-renderer`). The two no longer share an
  import name (`floem_legacy_renderer` vs `floem_renderer`), so keeping the v1
  crate at a root-level `renderer/` is no longer required for that reason.
- **`tooling/floem-test` → `tooling/floem-testkit`** — likewise gone: the v1
  harness is `legacy/floem-test` (package `floem-legacy-test`), so `floem-testkit`
  has no name to collide with, and renaming the v1 crate is no longer "out of
  scope."
- **`reactive/` gap** — the *naming* half is resolved the same way. The
  *architectural* half is unchanged: `floem-runtime`, `floem-runtime-core`,
  `floem-store` and `floem-task` still depend on `floem-legacy-reactive`, and the
  script still reports that as a NOTE, not a failure. Giving 2.0 its own reactive
  layer remains open work.

The layer table now carries the legacy crates explicitly at **L0**
(`floem-legacy-*`): anything may depend *down* onto them, nothing may depend *up*
out of them, so a stray `legacy → 2.0` edge fails instead of being silently
skipped. `LEGACY_DEPS` in the script tracks the one legacy edge that survives
(the v1 reactive graph).

Vendored forks moved from `platform-compat/` to `vendor/`, each with a
`NOTICE.md` (upstream URL, rev/version, local delta, exit criterion), and
`notices/` holds the `cargo-deny` / `cargo-about` configuration. The conventions
are written down in `docs/workspace-conventions.md`.
