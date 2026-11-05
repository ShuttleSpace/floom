# Floom 2.0 P0 Runtime Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Introduce a compatibility-friendly Floom 2.0 runtime foundation with stable element identity, fine-grained invalidation, scheduling, platform-independent Scene, renderer/platform boundaries, and measurable performance regressions without rewriting the existing widget ecosystem.

**Architecture:** Keep the current Floom implementation operational while adding new internal crates and adapters. The new runtime owns persistent Element identity and invalidation; Scene and Renderer are independent from widgets and OS details; Platform exposes a minimal lifecycle/window/surface/event ABI. Existing `View`/`ViewId` APIs are adapted until migration is complete.

**Tech Stack:** Rust 2024, workspace resolver 3, current Floom reactive implementation, `slotmap` or equivalent generational storage already present, `smallvec`, `bitflags`, Taffy 0.9.x adapter, existing Vello/Skia/tiny-skia renderer crates, Criterion for benchmarks.

**Spec:** `docs/superpowers/specs/2026-09-02-floom-2-architecture.md`

## Global Constraints

- Rust edition: 2024.
- Workspace resolver: 3.
- Preserve existing Floom `View` applications through compatibility adapters during P0.
- P0 must not require Android, iOS, HarmonyOS, Web, or Embedded implementations to be complete.
- `floom-scene` must not depend on OS, winit, Android, iOS, Vello, Skia, or WGPU implementation crates.
- `floom-renderer` must depend on Scene abstractions, not widgets or reactive state.
- `floom-platform` must not depend on concrete widget implementations.
- Every P0 feature needs focused tests and at least one benchmark where performance is relevant.
- No new public API is accepted without rustdoc and a compatibility rationale.

---

### Task 1: Freeze the current baseline and add architecture checks

**Files:**
- Create: `docs/superpowers/benchmarks/floom-2-baseline.md`
- Create: `tests/architecture/dependency_boundaries.rs`
- Modify: `Cargo.toml`
- Modify: `.github/workflows/*` relevant Rust CI workflow

**Interfaces:**
- Consumes: current workspace crate graph from `Cargo.toml`.
- Produces: a machine-checkable baseline that later tasks must preserve or intentionally update.

- [ ] **Step 1: Record current workspace and benchmark commands**

Record the current members, default features, renderer features, existing Criterion benches, Rust version, and commands used for `cargo test`, `cargo check`, and benches in `docs/superpowers/benchmarks/floom-2-baseline.md`.

- [ ] **Step 2: Write a failing dependency-boundary test**

The test should inspect Cargo metadata and fail when `floom-scene` or `floom-renderer` later acquires forbidden concrete platform/widget dependencies.

- [ ] **Step 3: Run the test to verify the guard executes**

Run:

```bash
cargo test --test architecture dependency_boundaries
```

Expected: PASS for the current graph or a deliberate FAIL with the exact forbidden dependency reported; keep the initial baseline explicit.

- [ ] **Step 4: Add the CI check**

Run the architecture test with the standard test job.

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/benchmarks/floom-2-baseline.md tests/architecture/dependency_boundaries.rs Cargo.toml .github/workflows
git commit -m "test: establish floom architecture boundaries"
```

---

### Task 2: Add `floom-types` and stable runtime identifiers

**Files:**
- Create: `types/Cargo.toml`
- Create: `types/src/lib.rs`
- Create: `types/src/geometry.rs`
- Create: `types/src/ids.rs`
- Create: `types/src/display.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- Consumes: existing `floom` geometry/ID types where migration needs aliases.
- Produces: `ElementId`, `SceneNodeId`, `ResourceId`, `WindowId`, `Rect`, `Size`, `Insets`, `Transform`, `ScaleFactor`, `DisplayInfo`.

- [ ] **Step 1: Write unit tests for generational IDs and geometry defaults**

Test that an ID from a removed generation cannot be used as the current generation and that geometry values have deterministic zero/default behavior.

- [ ] **Step 2: Run focused tests**

```bash
cargo test -p floom-types
```

- [ ] **Step 3: Implement identifiers and geometry**

Use compact copyable IDs with generation information. Keep the crate free from platform and renderer dependencies.

- [ ] **Step 4: Re-run tests**

```bash
cargo test -p floom-types
cargo check -p floom-types
```

- [ ] **Step 5: Commit**

```bash
git add types Cargo.toml
git commit -m "feat: add floom core types and stable ids"
```

---

### Task 3: Add `floom-scene` retained Scene model

**Files:**
- Create: `scene/Cargo.toml`
- Create: `scene/src/lib.rs`
- Create: `scene/src/node.rs`
- Create: `scene/src/scene.rs`
- Create: `scene/src/resource.rs`
- Create: `scene/src/damage.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- Consumes: `floom-types`.
- Produces: `Scene`, `SceneNode`, `SceneNodeId`, `DrawOp`, `ResourceId`, `DamageRegion`, `SceneDirty`.

- [ ] **Step 1: Write scene construction and stable-ID tests**

Test insertion, replacement, removal, stable IDs, child ordering, damage accumulation, and full-frame fallback.

- [ ] **Step 2: Run the focused tests and benchmark skeleton**

```bash
cargo test -p floom-scene
```

- [ ] **Step 3: Implement retained nodes**

Use data-oriented node storage and avoid `Box<dyn DrawOp>` collections. Represent common primitives as an enum/data payload.

- [ ] **Step 4: Implement `DamageRegion` operations**

Support union, clear, full-frame mode, and bounded small-rectangle storage without requiring a spatial index in P0.

- [ ] **Step 5: Run tests and dependency checks**

```bash
cargo test -p floom-scene
cargo tree -p floom-scene
```

Confirm that no OS or concrete renderer dependency exists.

- [ ] **Step 6: Commit**

```bash
git add scene Cargo.toml
git commit -m "feat: add retained floom scene"
```

---

### Task 4: Add Runtime Element arena and dirty flags

**Files:**
- Create: `runtime-core/Cargo.toml`
- Create: `runtime-core/src/lib.rs`
- Create: `runtime-core/src/element.rs`
- Create: `runtime-core/src/arena.rs`
- Create: `runtime-core/src/dirty.rs`
- Create: `runtime-core/src/tree.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- Consumes: `floom-types`.
- Produces: `ElementId`, `ElementArena`, `Element`, `DirtyFlags`, parent/child operations, invalidation propagation primitives.

- [ ] **Step 1: Write failing tests for element lifetime**

Cover insertion/removal, stale IDs, reparenting, child order, parent lookup, and generation reuse.

- [ ] **Step 2: Write failing tests for dirty propagation**

Examples:

```rust
assert_eq!(dirty_for_text_content(), DirtyFlags::PAINT);
assert_eq!(dirty_for_font_size(), DirtyFlags::LAYOUT | DirtyFlags::PAINT);
assert_eq!(dirty_for_semantics(), DirtyFlags::SEMANTICS);
```

- [ ] **Step 3: Run tests and verify failures**

```bash
cargo test -p floom-runtime-core
```

Expected: FAIL until the arena and flags are implemented.

- [ ] **Step 4: Implement generational arena**

Use a compact indexed storage strategy; do not store GPU resources inside Element.

- [ ] **Step 5: Implement tree mutation and invalidation**

Dirty propagation must be explicit and bounded. Do not perform a full-tree scan on local invalidation.

- [ ] **Step 6: Run focused tests and a micro-benchmark**

```bash
cargo test -p floom-runtime-core
cargo bench -p floom-runtime-core --bench element_update
```

- [ ] **Step 7: Commit**

```bash
git add runtime-core Cargo.toml
git commit -m "feat: add retained element arena and dirty tracking"
```

---

### Task 5: Bridge current `floom-reactive` into runtime invalidation

**Files:**
- Modify: `reactive/src/lib.rs`
- Modify: `reactive/src/scope.rs`
- Create: `reactive/src/runtime_bridge.rs`
- Modify: `runtime-core/src/lib.rs`
- Create: `runtime-core/src/reactive.rs`

**Interfaces:**
- Consumes: current `Signal`, `Memo`, `Effect`, and `Scope` APIs.
- Produces: a runtime subscriber mechanism that maps reactive invalidations to specific Element/property dirty flags without making reactive code depend on UI internals.

- [ ] **Step 1: Write tests for scoped subscription disposal**

Create a signal, subscribe from an element scope, dispose the scope, mutate the signal, and verify that the disposed subscriber is not notified.

- [ ] **Step 2: Write tests for batching**

Two writes inside a transaction/batch must yield one runtime invalidation dispatch.

- [ ] **Step 3: Run focused tests and confirm failures**

```bash
cargo test -p floom-reactive runtime_bridge
cargo test -p floom-runtime-core reactive
```

- [ ] **Step 4: Implement bridge hooks**

Keep `floom-reactive` independent of concrete Element types. The bridge carries opaque subscriber tokens and a callback/queue owned by runtime-core.

- [ ] **Step 5: Implement scope-bound disposal**

Use existing Scope lifecycle semantics as the cleanup authority rather than introducing a second disposal mechanism.

- [ ] **Step 6: Run the full reactive test suite**

```bash
cargo test -p floom-reactive
cargo test -p floom-runtime-core
```

- [ ] **Step 7: Commit**

```bash
git add reactive runtime-core
git commit -m "feat: bridge fine-grained reactivity into runtime"
```

---

### Task 6: Add frame scheduler and frame state machine

**Files:**
- Create: `runtime-core/src/scheduler.rs`
- Create: `runtime-core/src/frame.rs`
- Create: `runtime-core/src/queue.rs`
- Create: `runtime-core/benches/scheduler.rs`

**Interfaces:**
- Consumes: dirty flags and reactive invalidation events.
- Produces: deterministic frame phases and work queues.

- [ ] **Step 1: Write tests for priority ordering**

Assert the order `Input -> State -> Animation -> Layout -> Paint -> Semantics -> Idle` when all classes are pending.

- [ ] **Step 2: Write tests for frame coalescing**

Three invalidations before a scheduled frame must produce one frame request.

- [ ] **Step 3: Run failing tests**

```bash
cargo test -p floom-runtime-core scheduler
```

- [ ] **Step 4: Implement phase queues**

Use `SmallVec`/bounded vectors for common small-frame work and avoid heap allocation when queues are empty or contain only a handful of items.

- [ ] **Step 5: Implement frame budget metadata**

Track timestamps for each phase without forcing tracing overhead in release builds.

- [ ] **Step 6: Benchmark scheduling**

```bash
cargo bench -p floom-runtime-core --bench scheduler
```

- [ ] **Step 7: Commit**

```bash
git add runtime-core

git commit -m "feat: add frame scheduler"
```

---

### Task 7: Add thin Renderer ABI and adapters

**Files:**
- Create: `renderer-core/Cargo.toml`
- Create: `renderer-core/src/lib.rs`
- Create: `renderer-core/src/target.rs`
- Create: `renderer-core/src/frame.rs`
- Modify: existing `renderer/src/lib.rs`
- Modify: existing `vello/src/*` adapter entrypoints
- Modify: existing `skia/src/*` adapter entrypoints
- Modify: existing `tiny_skia/src/*` adapter entrypoints
- Modify: `Cargo.toml`

**Interfaces:**
- Consumes: `floom-scene` and render target types.
- Produces: `Renderer`, `RenderTarget`, `FrameInfo`, `DamageRegion` interfaces and adapters for existing renderer crates.

- [ ] **Step 1: Write mock renderer tests**

Create an in-memory renderer that records `begin_frame`, `render`, and `present` calls and assert exact order and damage arguments.

- [ ] **Step 2: Run focused tests**

```bash
cargo test -p floom-renderer-core
```

- [ ] **Step 3: Implement the ABI**

Keep it scene-oriented. Do not expose Widget, Element, Signal, winit, Skia, Vello, or WGPU types.

- [ ] **Step 4: Adapt tiny-skia first**

Use the software renderer as the reference implementation because it is easiest to make deterministic for tests.

- [ ] **Step 5: Adapt Vello and Skia without changing widget APIs**

Add adapters around the existing renderer implementations rather than rewriting their internal rendering code.

- [ ] **Step 6: Run renderer tests and an existing painting benchmark**

```bash
cargo test -p floom-renderer-core
cargo test -p floom-tiny-skia-renderer
cargo bench --bench painting
```

- [ ] **Step 7: Commit**

```bash
git add renderer-core renderer skia vello tiny_skia Cargo.toml

git commit -m "feat: introduce renderer scene ABI"
```

---

### Task 8: Add minimal Platform ABI

**Files:**
- Create: `platform/Cargo.toml`
- Create: `platform/src/lib.rs`
- Create: `platform/src/window.rs`
- Create: `platform/src/surface.rs`
- Create: `platform/src/event.rs`
- Create: `platform/src/lifecycle.rs`
- Create: `platform/src/capability.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- Consumes: `floom-types`.
- Produces: `Platform`, `Window`, `Surface`, `PlatformEvent`, `LifecycleEvent`, `PlatformCapabilities`.

- [ ] **Step 1: Write mock-platform tests**

Verify window creation, frame requests, event polling, lifecycle transitions, and capability reporting using a pure in-memory platform.

- [ ] **Step 2: Run tests**

```bash
cargo test -p floom-platform
```

- [ ] **Step 3: Implement the minimal ABI**

Do not place clipboard, camera, WebView, notification, or IME details in the core trait. Those become services/extensions.

- [ ] **Step 4: Add a desktop adapter facade**

Create a thin adapter around the existing winit-based event/window integration so current desktop behavior remains usable.

- [ ] **Step 5: Run desktop examples**

Run the current widget gallery and at least one minimal application through the adapter.

- [ ] **Step 6: Commit**

```bash
git add platform Cargo.toml

git commit -m "feat: add minimal floom platform abi"
```

---

### Task 9: Introduce `View` compatibility adapter

**Files:**
- Modify: current `crates/floom/src/view.rs` or the current file defining `View`
- Modify: current `crates/floom/src/view_id.rs` or the current file defining `ViewId`
- Create: `src/runtime_adapter.rs`
- Modify: `crates/floom/src/lib.rs`
- Modify: existing view tests

**Interfaces:**
- Consumes: legacy `View`, `ViewId`, existing style/layout/event/paint behavior.
- Produces: an `Element`-backed compatibility path where legacy views continue to render through the new runtime without forcing immediate widget migration.

- [ ] **Step 1: Add a compatibility smoke test**

Use the current counter example shape and assert that state changes still update only the affected label/value element.

- [ ] **Step 2: Run the current relevant tests**

```bash
cargo test
```

- [ ] **Step 3: Implement the adapter**

Map existing `ViewId` operations to `ElementId` where practical. Keep the old public signatures unchanged in P0.

- [ ] **Step 4: Add a debug assertion for invalidation scope**

For the compatibility counter, record the number of invalidated elements and assert that unrelated siblings are not marked dirty by a local text signal update.

- [ ] **Step 5: Run examples and existing benchmarks**

```bash
cargo run --example widget-gallery
cargo bench --bench view_creation
cargo bench --bench event_dispatch
cargo bench --bench painting
cargo bench --bench layout
```

- [ ] **Step 6: Commit**

```bash
git add src

git commit -m "refactor: bridge legacy views to retained runtime"
```

---

### Task 10: Add P0 integration benchmarks and memory regression harness

**Files:**
- Create: `benches/runtime_update.rs`
- Create: `benches/scene_update.rs`
- Create: `benches/element_tree.rs`
- Create: `benches/allocations.rs`
- Modify: `Cargo.toml`
- Modify: `docs/superpowers/benchmarks/floom-2-baseline.md`

**Interfaces:**
- Consumes: P0 runtime, scene, reactive bridge, and renderer adapters.
- Produces: repeatable benchmark cases for local updates, large element trees, scene edits, and common allocation counts.

- [ ] **Step 1: Add the local reactive update benchmark**

Measure one signal update affecting one text node in a 1k-element tree and separately in a 10k-element tree. The benchmark must report work/allocations rather than only total wall time where instrumentation permits.

- [ ] **Step 2: Add scene mutation benchmarks**

Measure replacing one text node, transforming one node, and changing an image resource reference.

- [ ] **Step 3: Add element tree benchmarks**

Measure insertion, removal, reparenting, and lookup for 1k and 10k elements.

- [ ] **Step 4: Add allocation instrumentation**

Use a scoped test allocator or existing Rust allocation instrumentation to report bytes/allocations for the hot operations.

- [ ] **Step 5: Run all P0 benchmarks**

```bash
cargo bench --bench runtime_update
cargo bench --bench scene_update
cargo bench --bench element_tree
cargo bench --bench allocations
```

- [ ] **Step 6: Record the baseline instead of inventing hard device-independent thresholds**

Update `docs/superpowers/benchmarks/floom-2-baseline.md` with the measured reference hardware, compiler, profile, renderer, and values.

- [ ] **Step 7: Commit**

```bash
git add benches Cargo.toml docs/superpowers/benchmarks/floom-2-baseline.md

git commit -m "perf: add floom 2 runtime regression benchmarks"
```

---

## P0 Exit Criteria

P0 is complete only when all of the following are true:

1. Existing Floom examples continue to build and the compatibility counter remains functional.
2. `ElementId` is stable and stale IDs are rejected safely.
3. Local reactive invalidation does not require a full UI tree scan.
4. Scheduler coalesces multiple invalidations into one frame.
5. Scene can be updated independently of Widget types.
6. Renderer ABI can execute a Scene through a deterministic software/reference backend.
7. Platform ABI can be driven by a pure mock platform and a desktop adapter.
8. Dependency-boundary tests pass.
9. P0 benchmark suite has recorded a reference baseline.
10. No public API claims hard memory/CPU targets without a measured device-specific baseline.
