# Floom 2.0 architecture

Floom 2.0 is a layered workspace: **dependencies point downward only**, and
`scripts/check-architecture-deps.sh` enforces that mechanically. A crate may
depend on its own level and below; an upward edge fails the gate unless it is
allowlisted with a written reason.

This document is the human-readable mirror of that gate. The layer table below
is parsed by `crates/floom2/tests/docs_parity.rs`, which fails if the two drift
apart — so this page cannot quietly become wrong.

Companion pages: [`EXAMPLES.md`](EXAMPLES.md) (the golden examples),
[`MIGRATION.md`](MIGRATION.md) (v1 → 2.0) and
[`RELEASE-POLICY.md`](RELEASE-POLICY.md) (what a release has to pass).

## Layer table

| crate | layer | what it owns |
| --- | --- | --- |
| `floom-types` | L0 | engine-neutral value types: geometry, ids, events vocabulary |
| `floom-legacy-renderer`, `floom-legacy-renderer-vello`, `floom-legacy-renderer-skia`, `floom-legacy-renderer-vger`, `floom-legacy-renderer-tiny-skia`, `floom-legacy-reactive`, `floom-legacy-test` | L0 | the v1 crates, kept below the 2.0 graph so nothing has to depend upward into them |
| `floom-core` | L1 | the element/widget contract: `Widget`, `WidgetAny`, contexts, semantics vocabulary |
| `floom-runtime-core`, `floom-runtime`, `floom-scheduler` | L2 | element arena, dirty propagation, input routing, task ordering |
| `floom-layout`, `floom-style`, `floom-text`, `floom-scene`, `floom-virtual`, `floom-accessibility` | L3 | layout engine, style resolution, text shaping, the retained scene, virtualization geometry, the semantics schema |
| `floom-renderer`, `floom-renderer-software`, `floom-renderer-vello`, `floom-renderer-skia`, `floom-renderer-tui`, `floom-renderer-web` | L4 | the renderer ABI plus concrete backends (including one whose target is a character grid rather than pixels, and one that blits a raster buffer to a `<canvas>`) |
| `floom-platform`, `floom-platform-desktop`, `floom-platform-android`, `floom-platform-ios`, `floom-platform-harmony`, `floom-platform-web`, `floom-platform-embedded`, `floom-platform-tui`, `floom-ui-events-winit` | L4 | the windowing ABI plus concrete backends (including one whose target is a character grid) |
| `floom-native` | L4 | native view composition: capability negotiation, slots, pooling |
| `floom-platform-mobile` | L4 | mobile OS capabilities (camera, location, sensors, vibration) behind one ABI, with `jni` / `objc2` backends |
| `floom-animation`, `floom-focus`, `floom-gesture`, `floom-overlay`, `floom-theme`, `floom-store`, `floom-task`, `floom-forms`, `floom-navigation`, `floom-persistence`, `floom-services`, `floom-error`, `floom-ime`, `floom-editor` | L5 | framework services, widget-free by design |
| `floom-design`, `floom-material`, `floom-cupertino`, `floom-fluent`, `floom-harmony` | L5 | design systems — they emit `floom-style` property values, not paint calls |
| `floom-widgets`, `floom-widgets-data` | L6 | widgets: Tier-0 primitives, controls, composites, data widgets |
| `floom-app` | L7 | the headless app shell: `WidgetHost`, `AppRunner`, input routing, semantics publication |
| `floom-testkit`, `floom-snapshot`, `floom-codegen`, `floom-ui-ir`, `floom-dsl`, `floom-hot-reload`, `floom-inspector`, `floom-devtools`, `floom-profiler`, `floom-agent`, `floom-accuracy` | L8 | tooling and compilers — **not** shipped in an application binary |
| `floom2` | L9 | the public facade: `floom2::prelude`, feature bundles. It aggregates the tooling tier too, which is why it sits above L8 even though it *is* shipped. |

Two crates sit outside the ladder on purpose:

- **`floom`** — the v1 monolith (`src/`). It is unlayered and allowlisted in the
  gate: it mixes app, widget, platform and paint concerns by construction. It is
  the *only* crate the 2.0 stack is allowed to depend on that is not in this
  table, and the only 2.0 → v1 edge still live is the reactive graph
  (`floom-legacy-reactive`), which the gate reports as a note, not a failure.
- **`examples/*`, `crates/ecosystem/*`, `crates/vendor/*`** — not layered (examples are
  leaves; the satellite repos are separate workspaces).

## Dependency diagram

```text
                         ┌─────────────────────────────┐
   L9  facade           │ floom2  (prelude, bundles)  │
                         └──────────────┬──────────────┘
        ┌───────────────────────────────┼──────────────────────────────┐
        │                               │                              │
   L8  tooling ─ floom-testkit, floom-snapshot, floom-inspector,       │
                 floom-devtools, floom-profiler, floom-codegen,        │
                 floom-ui-ir, floom-dsl, floom-hot-reload, floom-agent │
                                        │                              │
   L7  app shell ─────────────── floom-app ────────────────────────────┘
                                        │
   L6  widgets ────────── floom-widgets ── floom-widgets-data
                                        │
   L5  framework ── animation, focus, gesture, overlay, theme, store,
                    task, forms, navigation, persistence, services,
                    error, ime, editor
                    design systems: design, material, cupertino, fluent, harmony
                                        │
   L4  ABI + backends ── renderer ─┬── renderer-software
                        │          ├── renderer-vello  ── wgpu
                        │          └── renderer-skia
                        ├── platform ─┬── platform-desktop (winit)
                        │             ├── platform-android / ios / harmony
                        │             └── platform-web / platform-embedded
                        └── native (composition, slots, pooling)
                                        │
   L3  layout, style, text, scene, virtual, accessibility
                                        │
   L2  runtime-core (arena, dirty) ── runtime (routing) ── scheduler
                                        │
   L1  core (Widget contract, contexts)
                                        │
   L0  types ── and, below everything, the v1 legacy crates
```

Arrows point down. `floom2` is the only crate that may reach L8; no crate below
it may.

## The three ABIs

The layering exists so that the parts with the most platform-specific code are
replaceable without touching the parts with the most logic:

- **Renderer ABI (`floom-renderer`)** — a scene in, pixels out. Widgets name the
  ABI, never a backend; `floom-app` is the one crate allowed to name both.
- **Platform ABI (`floom-platform`)** — windows, surfaces, clipboard, menus.
  Same rule: widgets never name a backend.
- **Native ABI (`floom-native`)** — embedding an OS view. A provider declares
  capabilities, and the controller *negotiates* a composition mode per
  participant; a mode the backend cannot do is downgraded, never silently
  granted (a participant granted an impossible mode draws nothing, which looks
  exactly like a participant that was never asked for).

## Contributor guide

### Add a widget

1. Implement `floom_core::Widget` (+ `WidgetAny` for anything that lives in a
   type-erased tree) in `crates/floom-widgets`, or in a sibling crate if it is
   a data/media widget.
2. Keep it renderer- and platform-neutral: the gate fails a widget crate that
   names a concrete backend or an OS crate. Paint through `PaintContext`;
   declare semantics through `SemanticsHint`.
3. Re-export it from the crate root, and — if it belongs in the beginner
   surface — add it to `floom2::prelude` *and* to the inventory test
   (`crates/floom2/tests/prelude_surface.rs`).
4. Test it with the shared harness (`floom-widgets::testkit`:
   `AllocatingCreateContext`, `RecordingUpdateContext`, `SceneRecorder`,
   `dispatch`) — do not hand-roll recorders.

### Add a renderer or platform backend

1. Implement the ABI trait from `floom-renderer` (or `floom-platform`) in a new
   `crates/floom-renderer-<name>` / `crates/floom-platform-<name>` crate.
2. Register it in **three** places: the root `Cargo.toml` `members` list, the
   same file's `default-members` list, and the `LAYERS` table in
   `scripts/check-architecture-deps.sh`. (If you add a whole new top-level
   directory, add it to `LIB_DIRS` in that script too — the completeness guard
   only walks the directories listed there, and an unlisted directory is
   checked by nobody.)
3. Add the contract suite: backends are verified against the ABI's own tests,
   not only against their own.
4. Expose it as a facade feature bundle so an app opts in and nothing else links
   it (see `floom2`'s `renderer-gpu` / `desktop`).

### Add a native plugin

Implement `NativeViewProvider` (8 methods, one of them defaulted) — see
`examples/floom2-native-plugin`, which writes one and then proves negotiation,
lifecycle and pooling. A plugin advertises only what it can actually do; the
host downgrades requests to the advertised set.

### Add a crate

Beyond the three registrations above: keep dependencies downward, do not name
concrete backends from below L4, and update `docs/2.0/ARCHITECTURE.md` — the
parity test will fail until you do.

### Add an example

Add `examples/floom2-<topic>/`, make it run headless and assert the invariant
it demonstrates, then document it in `docs/2.0/EXAMPLES.md`. The parity test
requires both directions: no undocumented example, no documented example that
does not exist.
