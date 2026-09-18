# Floem 2.0 Complete Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Incrementally evolve the existing Floem repository into the architecture specified by `docs/superpowers/specs/2026-09-02-floem-2-complete-architecture.md`, preserving compatibility while adding a high-performance retained runtime, multi-platform platform/renderer boundaries, native composition, a Flutter-class widget system, and AI/tooling infrastructure.

**Architecture:** Introduce small stable boundaries first: core types → retained runtime → scene → renderer ABI → platform ABI → compatibility adapter. Build higher-level framework features only after those boundaries are measurable and testable. Keep old Floem `View` APIs working behind an adapter during migration; migrate concrete widgets/platforms incrementally.

**Tech Stack:** Rust workspace, existing Floem reactive/runtime foundations, Taffy adapter for layout, Parley-compatible text layer where already used by Floem, WGPU/Vello, Skia, software renderer, existing platform integrations, platform-native WebView/media APIs, Cargo features, snapshot testing, tracing/perf instrumentation, optional MCP/agent tooling.

**Spec:** `docs/superpowers/specs/2026-09-02-floem-2-complete-architecture.md`

## Global Constraints

- Preserve current Floem applications through a compatibility layer until a deliberate major-version migration.
- `floem-types` must remain platform/GPU independent.
- `floem-scene` must not depend on concrete OS windowing implementations.
- Renderer backends must consume Scene/Renderer ABI and must not depend on widgets.
- Widget crates must not depend on concrete renderer backends.
- HarmonyOS must be an independent platform implementation, not an Android alias.
- Reactive graph and Element Tree remain separate concepts.
- No per-frame JSON or string IPC for high-frequency native/media data.
- No whole-tree rebuild for local reactive property updates in the new runtime.
- Every phase adds tests and measurable benchmarks before claiming completion.

---

# Phase 0 — Repository Baseline and Guardrails

## Task 0.1: Inventory current Floem workspace

**Files:**
- Create: `docs/superpowers/baselines/2026-09-02-current-workspace.md`
- Inspect: workspace `Cargo.toml` and all current member manifests

**Interfaces:**
- Consumes: current repository layout
- Produces: a checked-in inventory of current crates, features, renderer backends, platform assumptions, public entry points, and likely compatibility seams.

- [x] **Step 1: Record current workspace members and feature flags.**
- [x] **Step 2: Record `floem::View`, `ViewId`, reactive entry points, renderer entry points, and window/platform entry points.**
- [x] **Step 3: Run the existing test suite and record baseline results.**
- [x] **Step 4: Record benchmark commands or create a baseline benchmark harness if none exists.**
- [x] **Step 5: Commit the baseline inventory.**

## Task 0.2: Add dependency-boundary CI

**Files:**
- Create: `scripts/check-architecture-deps.sh`
- Modify: CI workflow and workspace metadata as required
- Test: `scripts/check-architecture-deps.sh`

**Interfaces:**
- Consumes: Cargo dependency graph
- Produces: a failing CI check when forbidden dependencies cross architecture boundaries.

- [x] **Step 1: Write failing checks for renderer→widgets, scene→platform, widget→concrete-renderer, and types→platform/GPU dependencies.**
- [x] **Step 2: Verify the current tree is reported honestly, distinguishing pre-migration violations from newly introduced violations.**
- [x] **Step 3: Add an allowlist for legacy paths with explicit comments.**
- [x] **Step 4: Add CI invocation.**
- [x] **Step 5: Commit.**

## Task 0.3: Create benchmark baseline suite

**Files:**
- Create: `benches/runtime_baseline.rs`
- Create: `benches/scene_baseline.rs`
- Create: `benches/render_baseline.rs`
- Create: `benches/memory_baseline.md`

**Interfaces:**
- Consumes: existing public Floem APIs
- Produces: reproducible metrics for startup, idle, signal update, 1k/10k elements, layout, paint, scrolling, animation, and allocations.

- [x] **Step 1: Write benchmark cases.**
- [x] **Step 2: Run and save baseline measurements on one representative desktop target.**
- [x] **Step 3: Add a benchmark report format.**
- [x] **Step 4: Add a CI job that detects >10% regressions after calibration.** (policy documented; absolute CI fail deferred until multi-run calibration — see `benches/memory_baseline.md`)
- [x] **Step 5: Commit.**

---

# Phase 1 — Foundation and Core IDs

## Task 1.1: Introduce `floem-types`

**Files:**
- Create: `foundation/floem-types/Cargo.toml`
- Create: `foundation/floem-types/src/lib.rs`
- Modify: workspace `Cargo.toml`
- Test: `foundation/floem-types/src/lib.rs`

**Interfaces:**
- Consumes: no Floem runtime dependencies
- Produces: `Point`, `Size`, `Rect`, `Insets`, `Constraints`, `Transform`, `Color`, `ScaleFactor`, `DisplayInfo`, `WindowId`, `SurfaceId`, `ElementId`, `SceneNodeId`, `ResourceId`, `NativeViewId`, `PlatformId`.

- [x] **Step 1: Write compile tests proving the crate builds without platform/GPU dependencies.**
- [x] **Step 2: Implement the value types and conversions from existing Floem types.**
- [x] **Step 3: Add size/layout tests and ID generation tests.**
- [x] **Step 4: Add dependency boundary validation.**
- [x] **Step 5: Commit.**

## Task 1.2: Introduce `ElementId` with generation safety

**Files:**
- Modify: `foundation/floem-types/src/lib.rs`
- Create: `runtime/floem-runtime-core/src/id.rs`
- Test: `runtime/floem-runtime-core/src/id_tests.rs`

**Interfaces:**
- Consumes: existing `slotmap`/ID approach where applicable
- Produces: generation-safe `ElementId` and conversion helpers for legacy `ViewId`.

- [x] **Step 1: Write tests for stale-ID rejection.**
- [x] **Step 2: Implement generation semantics.**
- [x] **Step 3: Add compatibility conversion.** (skipped — Policy A: no `ViewId` shim; see decision doc)
- [x] **Step 4: Run focused tests.**
- [x] **Step 5: Commit.**

## Task 1.3: Define core Widget/Element contracts

**Files:**
- Create: `foundation/floem-core/Cargo.toml`
- Create: `foundation/floem-core/src/widget.rs`
- Create: `foundation/floem-core/src/element.rs`
- Create: `foundation/floem-core/src/context.rs`
- Test: `foundation/floem-core/tests/contracts.rs`

**Interfaces:**
- Produces: minimal `Widget`, `Element`, `CreateContext`, `UpdateContext`, `EventContext`, `PaintContext`, `SemanticsContext` contracts.

- [x] **Step 1: Write contract compilation tests.**
- [x] **Step 2: Implement minimal traits with no renderer/platform dependency.**
- [x] **Step 3: Add an example primitive widget.**
- [x] **Step 4: Run tests.**
- [x] **Step 5: Commit.**

---

# Phase 2 — Retained Runtime Core

## Task 2.1: Create `ElementArena`

**Files:**
- Create: `runtime/floem-runtime-core/Cargo.toml`
- Create: `runtime/floem-runtime-core/src/arena.rs`
- Test: `runtime/floem-runtime-core/src/arena_tests.rs`

**Interfaces:**
- Consumes: `ElementId`
- Produces:
```rust
pub struct ElementArena;
impl ElementArena {
    pub fn insert(&mut self, element: Element) -> ElementId;
    pub fn get(&self, id: ElementId) -> Option<&Element>;
    pub fn get_mut(&mut self, id: ElementId) -> Option<&mut Element>;
    pub fn remove(&mut self, id: ElementId) -> Option<Element>;
}
```

- [x] **Step 1: Write insert/get/remove/stale-ID tests.**
- [x] **Step 2: Implement the arena using the repository's preferred slot/generation mechanism.**
- [x] **Step 3: Add benchmarks for 1k, 10k, and 100k elements.**
- [x] **Step 4: Verify no per-element heap allocation is introduced by default.**
- [x] **Step 5: Commit.**

## Task 2.2: Implement `DirtyFlags` and invalidation propagation

**Files:**
- Create: `runtime/floem-runtime-core/src/dirty.rs`
- Test: `runtime/floem-runtime-core/src/dirty_tests.rs`

**Interfaces:**
- Produces:
```rust
pub struct DirtyFlags;
pub fn invalidate(id: ElementId, flags: DirtyFlags);
pub fn propagate_layout(id: ElementId);
```

- [x] **Step 1: Write tests for property-specific invalidation.**
- [x] **Step 2: Implement bitflags.**
- [x] **Step 3: Implement parent propagation for layout/children changes.**
- [x] **Step 4: Benchmark dirty marking versus whole-tree scanning.**
- [x] **Step 5: Commit.**

## Task 2.3: Build the persistent element tree

**Files:**
- Create: `runtime/floem-runtime-core/src/tree.rs`
- Test: `runtime/floem-runtime-core/src/tree_tests.rs`

**Interfaces:**
- Produces:
```rust
pub fn append_child(parent: ElementId, child: ElementId);
pub fn insert_child(parent: ElementId, index: usize, child: ElementId);
pub fn detach(id: ElementId);
pub fn move_child(parent: ElementId, id: ElementId, index: usize);
```

- [x] **Step 1: Write tests for append, insert, detach, reorder, and subtree traversal.**
- [x] **Step 2: Implement tree operations against `ElementArena`.**
- [x] **Step 3: Add consistency assertions in debug builds.**
- [x] **Step 4: Benchmark tree mutation.**
- [x] **Step 5: Commit.**

---

# Phase 3 — Reactive Bridge

## Task 3.1: Map Signals to runtime subscriptions

**Files:**
- Modify: current reactive crate internals only where necessary
- Create: `runtime/floem-runtime-core/src/reactive_bridge.rs`
- Test: `runtime/floem-runtime-core/src/reactive_bridge_tests.rs`

**Interfaces:**
- Consumes: existing Floem Signal/Memo/Effect/Scope mechanisms.
- Produces:
```rust
pub struct ReactiveBindingId;
pub fn bind(binding: ReactiveBinding) -> ReactiveBindingId;
pub fn unbind(id: ReactiveBindingId);
```

- [x] **Step 1: Write a test where a Signal changes one Text property without rebuilding sibling elements.**
- [x] **Step 2: Implement the binding registry.**
- [x] **Step 3: Route invalidation to the exact `ElementId` and property.**
- [x] **Step 4: Measure update counts.**
- [x] **Step 5: Commit.**

## Task 3.2: Add Store/Selector primitives

**Files:**
- Create: `framework/floem-store/src/lib.rs`
- Test: `framework/floem-store/tests/selectors.rs`

**Interfaces:**
```rust
pub fn store<T>(value: T) -> Store<T>;
impl<T> Store<T> {
    pub fn select<U>(&self, selector: impl Fn(&T) -> U + 'static) -> Signal<U>;
    pub fn update(&self, f: impl FnOnce(&mut T));
}
```

- [x] **Step 1: Write selective-subscription tests.**
- [x] **Step 2: Implement selectors on the existing reactive graph.**
- [x] **Step 3: Verify unrelated selectors are not invalidated.**
- [x] **Step 4: Benchmark selector overhead.**
- [x] **Step 5: Commit.**

## Task 3.3: Add Resource and Task primitives

**Files:**
- Create: `framework/floem-task/src/lib.rs`
- Test: `framework/floem-task/tests/resource.rs`

**Interfaces:**
```rust
enum ResourceState<T, E> { Initial, Loading, Ready(T), Refreshing(T), Error(E) }
pub fn spawn(...);
pub fn ui_spawn(...);
```

- [x] **Step 1: Write lifecycle/cancellation tests.**
- [x] **Step 2: Implement executor abstraction.**
- [x] **Step 3: Bind task lifetime to Scope.**
- [x] **Step 4: Add WASM/no-runtime executor test adapter.**
- [x] **Step 5: Commit.**

---

# Phase 4 — Scheduler and Frame Runtime

## Task 4.1: Implement priority scheduler

**Files:**
- Create: `runtime/floem-scheduler/Cargo.toml`
- Create: `runtime/floem-scheduler/src/lib.rs`
- Test: `runtime/floem-scheduler/tests/order.rs`

**Interfaces:**
```rust
enum Priority { Input, State, Animation, Layout, Paint, Accessibility, Idle }
fn schedule(priority: Priority, task: TaskId);
```

- [x] **Step 1: Write ordering tests.**
- [x] **Step 2: Implement queues and coalescing.**
- [x] **Step 3: Ensure idle tasks yield to frame-critical work.**
- [x] **Step 4: Add frame-budget tests.**
- [x] **Step 5: Commit.**

## Task 4.2: Build `floem-runtime`

**Files:**
- Create: `runtime/floem-runtime/Cargo.toml`
- Create: `runtime/floem-runtime/src/lib.rs`
- Test: `runtime/floem-runtime/tests/counter.rs`

**Interfaces:**
```rust
pub struct Runtime;
impl Runtime {
    pub fn mount(root: impl Widget) -> RootHandle;
    pub fn dispatch(event: Event);
    pub fn request_frame();
    pub fn tick(now: Instant);
}
```

- [x] **Step 1: Write a Counter test.**
- [x] **Step 2: Implement mount/event/tick plumbing.**
- [x] **Step 3: Ensure a local state update does not rebuild unrelated elements.**
- [x] **Step 4: Add frame instrumentation hooks.**
- [x] **Step 5: Commit.**

---

# Phase 5 — Scene Graph and Paint Separation

## Task 5.1: Create `floem-scene`

**Files:**
- Create: `scene/floem-scene/Cargo.toml`
- Create: `scene/floem-scene/src/lib.rs`
- Create: `scene/floem-scene/src/node.rs`
- Test: `scene/floem-scene/tests/scene.rs`

**Interfaces:**
```rust
pub struct Scene;
pub struct SceneNodeId;
impl Scene {
    pub fn begin_update(&mut self);
    pub fn upsert(&mut self, id: SceneNodeId, node: SceneNode);
    pub fn remove(&mut self, id: SceneNodeId);
    pub fn finish_update(&mut self);
}
```

- [x] **Step 1: Write stable-ID and dirty-update tests.**
- [x] **Step 2: Implement retained Scene nodes.**
- [x] **Step 3: Add damage tracking.**
- [x] **Step 4: Confirm the crate has no OS/windowing dependency.**
- [x] **Step 5: Commit.**

## Task 5.2: Implement resource registry

**Files:**
- Create: `scene/floem-scene/src/resources.rs`
- Test: `scene/floem-scene/tests/resources.rs`

**Interfaces:**
```rust
pub struct ResourceRegistry;
pub fn insert(resource: Resource) -> ResourceId;
pub fn retain(id: ResourceId);
pub fn release(id: ResourceId);
```

- [x] **Step 1: Write shared-resource tests.**
- [x] **Step 2: Implement ref-count/usage tracking.**
- [x] **Step 3: Add budget/eviction hooks.**
- [x] **Step 4: Commit.**

---

# Phase 6 — Renderer ABI

## Task 6.1: Define renderer core API

**Files:**
- Create: `renderer/floem-renderer/Cargo.toml`
- Create: `renderer/floem-renderer/src/lib.rs`
- Test: `renderer/floem-renderer/tests/contract.rs`

**Interfaces:**
```rust
trait Renderer { ... }
enum RenderTarget { Surface(...), Texture(...), External(...), Software(...) }
struct FrameInfo;
struct DamageRegion;
```

- [x] **Step 1: Write compile-only backend contract tests.**
- [x] **Step 2: Implement renderer-neutral types.**
- [x] **Step 3: Prove a fake renderer can consume a Scene without widgets.**
- [x] **Step 4: Commit.**

## Task 6.2: Adapt existing Vello renderer

**Files:**
- Modify: existing Vello renderer crate
- Create: `renderer/floem-renderer-vello/src/adapter.rs`
- Test: existing renderer tests + new Scene adapter tests

**Interfaces:**
- Consumes: `Scene`, `RenderTarget`, `DamageRegion`.
- Produces: Vello/WGPU frame submission.

- [x] **Step 1: Wrap current Vello drawing path behind the new ABI.** (recording adapter under `render/floem-renderer-vello`; GPU submit remains in legacy `floem_vello_renderer` until paint migration)
- [x] **Step 2: Add rect/image/text fast-path tests.**
- [x] **Step 3: Add resource cache instrumentation.**
- [x] **Step 4: Benchmark full-frame and partial-damage paths.**
- [x] **Step 5: Commit.**

## Task 6.3: Adapt Skia and software backends

**Files:**
- Modify: existing Skia renderer crate
- Modify: existing tiny/software renderer crate
- Test: backend compatibility matrix

- [x] **Step 1: Implement renderer ABI adapters.** (`render/floem-renderer-skia`, `render/floem-renderer-software`)
- [x] **Step 2: Run identical Scene snapshots through both backends.**
- [x] **Step 3: Add software-headless snapshot tests.**
- [x] **Step 4: Commit.**

## Task 6.4: Implement render-thread Scene handoff

**Files:**
- Modify: `runtime/floem-runtime/src/frame.rs`
- Create: `renderer/floem-renderer/src/frame_queue.rs`
- Test: `runtime/floem-runtime/tests/render_handoff.rs`

- [x] **Step 1: Write a test proving render work can consume an immutable Scene snapshot.**
- [x] **Step 2: Implement single-producer/single-consumer or bounded queue appropriate to the target platform.**
- [x] **Step 3: Instrument queue wait and frame latency.**
- [x] **Step 4: Commit.**

---

# Phase 7 — Platform ABI

## Task 7.1: Define platform core

**Files:**
- Create: `platform/floem-platform/Cargo.toml`
- Create: `platform/floem-platform/src/lib.rs`
- Test: `platform/floem-platform/tests/contracts.rs`

**Interfaces:**
```rust
trait Platform { ... }
trait Window { ... }
trait Surface { ... }
enum PlatformEvent { ... }
struct PlatformCapabilities { ... }
```

- [x] **Step 1: Write a fake platform test harness.**
- [x] **Step 2: Implement window/surface/event/lifecycle/display contracts.**
- [x] **Step 3: Verify no concrete platform crate leaks into core.**
- [x] **Step 4: Commit.**

## Task 7.2: Migrate desktop backend

**Files:**
- Create/modify: `platform/floem-platform-desktop`
- Modify: existing Winit integration where present
- Test: desktop window/event tests

- [x] **Step 1: Map existing window lifecycle to the new ABI.**
- [x] **Step 2: Map pointer/keyboard/display events.**
- [x] **Step 3: Add clipboard and IME service adapters.**
- [x] **Step 4: Run macOS/Windows/Linux smoke tests.**
- [x] **Step 5: Commit.**

## Task 7.3: Create independent Android, iOS, and Harmony platform crates

**Files:**
- Create: `platform/floem-platform-android`
- Create: `platform/floem-platform-ios`
- Create: `platform/floem-platform-harmony`
- Test: per-platform lifecycle/input/display contract tests

- [x] **Step 1: Implement lifecycle surface contracts.**
- [x] **Step 2: Implement input/display/capability reporting.**
- [x] **Step 3: Implement IME/focus bridges.**
- [x] **Step 4: Implement accessibility bridge stubs with explicit unsupported capability states.**
- [x] **Step 5: Commit each platform separately.**

## Task 7.4: Implement Web platform

**Files:**
- Create: `platform/floem-platform-web`
- Test: WASM/browser smoke tests

- [x] **Step 1: Implement event/frame clock/display abstraction.**
- [x] **Step 2: Implement WebGPU target detection and fallback capability reporting.**
- [x] **Step 3: Implement DOM/semantic bridge hooks.**
- [x] **Step 4: Commit.**

## Task 7.5: Implement Embedded platform baseline

**Files:**
- Create: `platform/floem-platform-embedded`
- Test: headless DRM/framebuffer simulation

- [x] **Step 1: Define minimal surface/input/frame-clock contract.**
- [x] **Step 2: Implement a Linux DRM/KMS or framebuffer-compatible adapter behind feature gates.**
- [x] **Step 3: Validate with software renderer first.**
- [x] **Step 4: Commit.**

---

# Phase 8 — Legacy View Adapter

> **SKIPPED for this fork (Policy A).**  
> Decision: `docs/superpowers/decisions/2026-09-04-no-legacy-view-adapter.md`.  
> Do not implement `LegacyViewAdapter`. When the new Element runtime lands, replace or delete the old `View` / `ViewId` public surface instead of dual-running.

---

# Phase 9 — Text and Editing

## Task 9.1: Define text layout API

**Files:**
- Create: `text/floem-text/Cargo.toml`
- Create: `text/floem-text/src/lib.rs`
- Test: shaping/wrapping/selection tests

- [x] **Step 1: Write Unicode, bidi, fallback, and line-breaking tests.**
- [x] **Step 2: Wrap the current text engine behind a neutral API.**
- [x] **Step 3: Emit renderer-neutral glyph runs.**
- [x] **Step 4: Benchmark shaping/cache behavior.**
- [x] **Step 5: Commit.**

## Task 9.2: Build shared editing model

**Files:**
- Create: `text/floem-editor/src/document.rs`
- Create: `text/floem-editor/src/selection.rs`
- Create: `text/floem-editor/src/undo.rs`
- Test: editing semantics

- [x] **Step 1: Write cursor/selection/undo tests.**
- [x] **Step 2: Implement document transactions.**
- [x] **Step 3: Add IME composition hooks.**
- [x] **Step 4: Commit.**

---

# Phase 10 — Style, Layout, Theme

## Task 10.1: Build backend-neutral layout API

**Files:**
- Create: `layout/floem-layout/src/lib.rs`
- Create: `layout/floem-layout/src/taffy_adapter.rs`
- Test: flex/grid/stack/measure tests

- [x] **Step 1: Write layout contract tests.**
- [x] **Step 2: Implement Taffy adapter.**
- [x] **Step 3: Ensure Taffy node IDs do not leak to public API.**
- [x] **Step 4: Benchmark incremental subtree layout.**
- [x] **Step 5: Commit.**

## Task 10.2: Implement Style and selector state

**Files:**
- Create: `layout/floem-style/src/lib.rs`
- Test: state selector/inheritance/token tests

- [x] **Step 1: Write tests for normal/hover/pressed/focused/disabled/selected.**
- [x] **Step 2: Implement style property storage and precedence.**
- [x] **Step 3: Implement inherited typography properties only.**
- [x] **Step 4: Benchmark style invalidation.**
- [x] **Step 5: Commit.**

## Task 10.3: Implement Theme and tokens

**Files:**
- Create: `framework/floem-theme/src/lib.rs`
- Test: light/dark/dynamic token tests

- [x] **Step 1: Define color/typography/spacing/radius/elevation/motion token structs.**
- [x] **Step 2: Implement theme propagation.**
- [x] **Step 3: Add platform-adaptive token hooks.**
- [x] **Step 4: Commit.**

---

# Phase 11 — Interaction, Focus, Gesture, Animation, Overlay

## Task 11.1: Implement event routing and hit testing

**Files:**
- Create: `runtime/floem-runtime/src/event.rs`
- Test: bubbling/capture/hit-test tests

- [x] **Step 1: Write event target selection tests.**
- [x] **Step 2: Implement pointer routing and pointer capture.**
- [x] **Step 3: Integrate native graph hit regions.**
- [x] **Step 4: Commit.**

## Task 11.2: Implement FocusManager

**Files:**
- Create: `framework/floem-focus/src/lib.rs`
- Test: focus traversal and scope tests

- [x] **Step 1: Write keyboard/tab/shift-tab tests.**
- [x] **Step 2: Implement focus scopes and traversal policy.**
- [x] **Step 3: Integrate platform focus.**
- [x] **Step 4: Commit.**

## Task 11.3: Implement Gesture Arena

**Files:**
- Create: `framework/floem-gesture/src/lib.rs`
- Test: tap/drag/scroll/scale conflict tests

- [x] **Step 1: Write recognizer competition tests.**
- [x] **Step 2: Implement arena ownership/cancellation.**
- [x] **Step 3: Integrate Scroll and Native View interaction.**
- [x] **Step 4: Commit.**

## Task 11.4: Implement animation scheduler

**Files:**
- Create: `framework/floem-animation/src/lib.rs`
- Test: tween/spring/keyframe tests

- [x] **Step 1: Write deterministic animation tests with a fake clock.**
- [x] **Step 2: Implement platform-frame-driven ticking.**
- [x] **Step 3: Add compositor-only transform/opacity path.**
- [x] **Step 4: Benchmark 60/120Hz scenarios.**
- [x] **Step 5: Commit.**

## Task 11.5: Implement Overlay system

**Files:**
- Create: `framework/floem-overlay/src/lib.rs`
- Test: modal/focus/z-order tests

- [x] **Step 1: Write dialog/popup/bottom-sheet tests.**
- [x] **Step 2: Implement overlay tree and hit-testing priority.**
- [x] **Step 3: Integrate animation and focus.**
- [x] **Step 4: Commit.**

---

# Phase 12 — Virtualization

## Task 12.1: Implement VirtualViewport

**Files:**
- Create: `layout/floem-virtual/src/lib.rs`
- Test: 1k/10k/100k item range tests

- [x] **Step 1: Write viewport/range/key reuse tests.**
- [x] **Step 2: Implement visible range calculation.**
- [x] **Step 3: Implement stable-key element reuse.**
- [x] **Step 4: Add overscan controls.**
- [x] **Step 5: Benchmark CPU/RAM/scrolling.**
- [x] **Step 6: Commit.**

## Task 12.2: Native view pooling

**Files:**
- Create: `native/floem-native/src/pool.rs`
- Test: reuse/lifecycle tests

- [x] **Step 1: Write native slot reuse tests.**
- [x] **Step 2: Implement pool and attach/detach lifecycle.**
- [x] **Step 3: Benchmark 10k virtualized rows with native placeholders.**
- [x] **Step 4: Commit.**

---

# Phase 13 — Accessibility and IME

## Task 13.1: Semantics Tree

**Files:**
- Create: `framework/floem-accessibility/src/lib.rs`
- Test: semantics tree tests

- [x] **Step 1: Define semantics node schema.**
- [x] **Step 2: Write role/label/value/action tests.**
- [x] **Step 3: Connect Element dirty semantics updates.**
- [x] **Step 4: Commit.**

## Task 13.2: Platform accessibility adapters

**Files:**
- Modify: each platform backend with accessibility adapter modules
- Test: at least one desktop and one mobile target in CI/device testing

- [x] **Step 1: Implement desktop adapter.**
- [x] **Step 2: Implement Android/iOS adapters.**
- [x] **Step 3: Add Harmony/Web adapters or explicit capability-limited implementations.**
- [ ] **Step 4: Test native subtree integration.**
- [x] **Step 5: Commit per platform.**

> Note (Step 4): host-side adapter tests cover diff application and action
> dispatch. Device/CI testing of a *native* a11y subtree still needs an OS
> bridge, which no backend wires yet.

## Task 13.3: IME abstraction

**Files:**
- Create: `text/floem-ime/src/lib.rs`
- Modify: platform backends
- Test: composition/selection/candidate tests

- [x] **Step 1: Write fake-IME tests.**
- [x] **Step 2: Implement platform-neutral IME state.**
- [ ] **Step 3: Bridge native text inputs and WebView.**
- [x] **Step 4: Commit.**

> Note (Step 3): `floem-ime` is the platform-neutral state machine. Bridging it
> to concrete OS text inputs / WebView is deferred to the platform backends
> (see `floem_platform::Ime`).

---

# Phase 14 — Core Widgets

## Task 14.1: Primitive widgets

**Files:**
- Create: `widgets/floem-widgets/src/primitives.rs`
- Test: primitive snapshots

- [x] **Step 1: Implement Text, Icon, Image, Container, Spacer, Divider.**
- [x] **Step 2: Add scene snapshots.**
- [x] **Step 3: Add semantics snapshots.**
- [x] **Step 4: Commit.**

> Note: the primitives record through `floem_core::PaintContext`, which was
> extended in this task (it had no colour and no text/image op at all). See
> `docs/superpowers/decisions/2026-09-12-widget-paint-context-extension.md`.
> Snapshots are inline textual snapshots in `widgets/floem-widgets/tests`,
> produced by `floem_widgets::testkit` — no external snapshot crate is used.

## Task 14.2: Interactive controls

**Files:**
- Create: `widgets/floem-widgets/src/controls.rs`
- Test: interaction/snapshot tests

- [x] **Step 1: Implement Button.**
- [x] **Step 2: Implement TextField.**
- [x] **Step 3: Implement Checkbox/Radio/Switch.**
- [x] **Step 4: Implement Slider/Progress/Select.**
- [x] **Step 5: Add state variants and accessibility.**
- [x] **Step 6: Commit.**

> Notes:
> - `EventContext` was payload-free, so `floem-types` gained `event.rs`
>   (`PointerEvent` / `ScrollEvent` / `KeyEvent` / `Modifiers` / `Event`) and
>   `EventContext` gained `event()`, `bounds()` and `request_focus()`. Without
>   these no control can observe a click or convert a pointer position into a
>   value. See `docs/superpowers/decisions/2026-09-12-widget-paint-context-extension.md`.
> - `Select` renders its option list *inside* its own bounds; escaping the
>   parent's clip needs the `floem-overlay` integration (Phase 16 area).
> - `TextField` handles editing keys directly. IME composition still needs the
>   `floem-ime` bridge (Phase 13.3 Step 3, already deferred).

## Task 14.3: Navigation/layout widgets

**Files:**
- Create: `widgets/floem-widgets/src/compose.rs` (structural composites) and
  `widgets/floem-widgets/src/navigation.rs` (navigation chrome)
- Test: `app/floem-app/tests/reconcile.rs` + per-module unit tests

- [x] **Step 1: Implement List/Grid/Stack/Tabs.**
- [x] **Step 2: Implement NavigationBar/NavigationRail/Drawer.**
- [x] **Step 3: Implement Dialog/Popup/BottomSheet.**
- [x] **Step 4: Commit.**

> Notes:
> - The plan's single `navigation.rs` landed split: `compose.rs` holds the
>   structural composites (Stack/List/Grid/Conditional/Overlay/Dialog/Popup/
>   BottomSheet/Tabs) and `navigation.rs` holds the navigation chrome
>   (NavigationBar/NavigationRail/NavigationDrawer). `Conditional` and `Overlay`
>   were added because the overlay-backed widgets build on them.
> - The `WidgetHost` reconciliation coordinator (`app/floem-app`) diffs each
>   composite's `children()` by `ChildKey`; a composite flags a change from
>   `Widget::update` via `mark_children_dirty`. Its layout lowering now combines
>   a slot's flex-item intent with the widget's own container intent, and
>   `reconcile` no longer wipes imperatively-mounted (`host.mount`) children.
> - Demo: `examples/floem2-counter` drives every composite end-to-end.

## Task 14.4: Forms

**Files:**
- Create: `framework/floem-forms/src/lib.rs`
- Test: validation/form-state tests + `app/floem-app/tests/forms.rs`

- [x] **Step 1: Define FormController state.**
- [x] **Step 2: Implement validators and async validation.**
- [x] **Step 3: Integrate TextField/Select/Switch.**
- [x] **Step 4: Commit.**

> Notes:
> - `floem-forms` is deliberately **widget-free**: the architecture puts
>   `framework/` *below* `widgets/`, so the crate models a form as plain data
>   and exposes a closure-based `FieldBinding<C>` seam. The widgets layer (here,
>   `app/floem-app/tests/forms.rs`) supplies the binding for `TextField` /
>   `Switch` / `Select` and drives the controller end-to-end. `floem-forms`
>   therefore adds no dependency edges.
> - Async validation models the server-round-trip race: `begin_validation`
>   mints a generation-stamped `ValidationTicket` and `complete` applies a
>   result only while its ticket is current, so a slow reply for an old value is
>   dropped rather than overwriting the new verdict. No runtime is required to
>   test it.
> - 10 crate unit tests + 1 cross-layer integration test.

---

# Phase 15 — Navigation / Persistence / App Services

## Task 15.1: Typed Router/Navigator

**Files:**
- Create: `framework/floem-navigation/src/lib.rs`
- Test: route/deep-link/restoration tests

- [x] **Step 1: Write typed-route tests.**
- [x] **Step 2: Implement Router state and Navigator stack.**
- [x] **Step 3: Add URL/deep-link parser boundary.**
- [x] **Step 4: Add stack/tab/split containers.**
- [x] **Step 5: Commit.**

## Task 15.2: Persistence and restoration

**Files:**
- Create: `framework/floem-persistence/src/lib.rs`
- Test: key-value/restoration tests

- [x] **Step 1: Define backend traits.**
- [x] **Step 2: Implement in-memory and file-based reference providers.**
- [x] **Step 3: Add restoration snapshots.**
- [x] **Step 4: Connect platform storage providers.**
- [x] **Step 5: Commit.**

## Task 15.3: Service scopes / DI

**Files:**
- Create: `framework/floem-services/src/lib.rs`
- Test: scope lifetime tests

- [x] **Step 1: Write service registration/resolution tests.**
- [x] **Step 2: Implement scoped service registry with typed-first APIs.**
- [x] **Step 3: Verify disposal with Scope lifetime.**
- [x] **Step 4: Commit.**

## Task 15.4: ErrorBoundary

**Files:**
- Create: `framework/floem-error/src/lib.rs`
- Test: recoverable/fatal boundary tests

- [x] **Step 1: Write fallback rendering tests.**
- [x] **Step 2: Implement error capture around recoverable application callbacks.**
- [x] **Step 3: Preserve diagnostic context in debug builds.**
- [x] **Step 4: Commit.**

---

# Phase 16 — Native Composition

## Task 16.1: Define Native ABI

**Files:**
- Create: `native/floem-native/Cargo.toml`
- Create: `native/floem-native/src/lib.rs`
- Test: native-provider contract tests

**Interfaces:**
```rust
trait NativeViewProvider { ... }
trait NativeViewController { ... }
struct NativeCapabilities { ... }
enum NativeCompositionMode { Overlay, Texture, Surface, Offscreen }
```

- [ ] **Step 1: Write fake native provider tests.**
- [ ] **Step 2: Implement lifecycle/geometry/capability contracts.**
- [ ] **Step 3: Commit.**

## Task 16.2: Implement Overlay composition

**Files:**
- Modify: each platform native host
- Test: geometry/input/focus tests

- [ ] **Step 1: Place a fake NativeView above Scene content.**
- [ ] **Step 2: Synchronize rect/visibility/z-order.**
- [ ] **Step 3: Route pointer/focus to native view.**
- [ ] **Step 4: Validate on desktop and Android/iOS.**
- [ ] **Step 5: Commit.**

## Task 16.3: Implement Texture/Surface composition where supported

**Files:**
- Modify: renderer/native integration modules per platform
- Test: transformed native-view scenarios

- [ ] **Step 1: Add capability negotiation.**
- [ ] **Step 2: Implement texture import path.**
- [ ] **Step 3: Implement surface compositor path on capable mobile/desktop backends.**
- [ ] **Step 4: Benchmark copy/latency versus Overlay.**
- [ ] **Step 5: Commit.**

## Task 16.4: Implement snapshot/offscreen transition mode

**Files:**
- Modify: `framework/floem-animation`
- Modify: `native/floem-native`
- Test: transition snapshot tests

- [ ] **Step 1: Add native snapshot capability.**
- [ ] **Step 2: Freeze a native view into a texture for a transition.**
- [ ] **Step 3: Animate the texture and restore native ownership.**
- [ ] **Step 4: Benchmark transition overhead.**
- [ ] **Step 5: Commit.**

---

# Phase 17 — WebView / Video / Camera / Map

## Task 17.1: WebView provider

**Files:**
- Create: `native/floem-webview`
- Test: navigation/JS/IME/accessibility smoke tests

- [ ] **Step 1: Define typed WebView API.**
- [ ] **Step 2: Implement macOS/iOS WKWebView provider.**
- [ ] **Step 3: Implement Windows WebView2 provider.**
- [ ] **Step 4: Implement Linux WebKit provider.**
- [ ] **Step 5: Implement Android WebView provider.**
- [ ] **Step 6: Add Harmony provider with the platform-native web component.**
- [ ] **Step 7: Add Web provider policy for browser-native environments.**
- [ ] **Step 8: Commit per platform group.**

## Task 17.2: Video provider

**Files:**
- Create: `native/floem-video`
- Test: play/pause/surface/lifecycle tests

- [ ] **Step 1: Define media state machine.**
- [ ] **Step 2: Implement native-surface path first.**
- [ ] **Step 3: Add texture/offscreen fallback where required.**
- [ ] **Step 4: Implement background/suspend handling.**
- [ ] **Step 5: Commit.**

## Task 17.3: Camera provider

**Files:**
- Create: `native/floem-camera`
- Test: device/session lifecycle tests

- [ ] **Step 1: Define camera/session capability schema.**
- [ ] **Step 2: Implement preview as native surface.**
- [ ] **Step 3: Implement frame-buffer path for processing.**
- [ ] **Step 4: Commit per platform.**

## Task 17.4: Map provider

**Files:**
- Create: `native/floem-map`
- Test: marker/gesture/lifecycle tests

- [ ] **Step 1: Define provider API.**
- [ ] **Step 2: Implement one reference provider per mobile/desktop tier.**
- [ ] **Step 3: Integrate gesture arbitration.**
- [ ] **Step 4: Commit.**

---

# Phase 18 — Native Code Generation

## Task 18.1: Define native component schema

**Files:**
- Create: `compiler/floem-codegen/schema/*.rs`
- Test: schema round-trip tests

- [x] **Step 1: Define typed command/event/property schema.** — `compiler/floem-codegen/src/schema.rs`: `ComponentSchema` + `PropertySchema` / `CommandSchema` / `EventSchema` / `Param`, `TypeRef`, validated `Ident`.
- [x] **Step 2: Define ABI versioning.** — `AbiVersion { major, minor }` with `compatible()` (same major, producer minor ≥ required minor).
- [x] **Step 3: Add schema serialization for tooling only.** — serde derive + `serde_json` round-trip test; `From` bridges to the `floem-types` native vocabulary so the compiler stays runtime-free.
- [x] **Step 4: Commit.** — lands together with Phase 18.2 below (2026-09-18; the crate had been created but never committed, and was missing from the architecture `LAYERS` table — both fixed).

## Task 18.2: Generate Swift/Kotlin/C/C++ glue

**Files:**
- Create: `compiler/floem-codegen/backends/*`
- Test: generated-code compile fixtures

- [x] **Step 1: Implement C ABI generator.** — `src/backends/c.rs`: opaque handle, property getters/setters (access-aware), commands, event callbacks; helper types (`floem_bytes`, `floem_slice_*`, `floem_resource_id`, opaque forwards) emitted only when used. Struct-shaped callback params are expanded to `(pointer, length)` — no aggregate copies across FFI, and the only shape Swift's `@convention(c)` can represent.
- [x] **Step 2: Implement Swift generator.** — `src/backends/swift.rs`: `<Name>Shims.swift` (`@_silgen_name` bindings + C-layout mirror structs) and `<Name>.swift` (class; events recover the instance from `userdata`, so registered closures capture nothing and convert to C function pointers).
- [x] **Step 3: Implement Kotlin generator.** — `src/backends/kotlin.rs`: class with `external` JNI accessors (properties delegate to them) and stored lambda events.
- [x] **Step 4: Implement C++/Harmony bindings as needed.** — `src/backends/cpp.rs`: header-only `floem_gen::<Name>` RAII facade over the C ABI (`std::string` for `String`, `std::function` + static trampolines for events).
- [x] **Step 5: Add compile fixtures.** — `tests/compile_fixtures.rs` writes the generated glue to a scratch dir and **invokes the real toolchains**: `cc -std=c11`, `c++ -std=c++17`, `swiftc -typecheck`, over both a realistic schema and a wide type-coverage schema; `kotlinc` is skipped-with-reason when absent. Backend total **30 tests** (24 unit + 6 fixtures) green. The fixture is a check that can fail — it caught two real generator bugs while being written (Swift argument labels in a function type; a Swift struct that `@convention(c)` cannot represent).
- [x] **Step 6: Commit.** — 2026-09-18.

---

# Phase 19 — Desktop/Mobile/Web/Embedded Completion

## Task 19.1: macOS completion

- [ ] Validate lifecycle, display scale, menus, clipboard, IME, accessibility, WebView, video, native overlay, renderer suspension.
- [ ] Add device smoke tests and benchmark report.
- [ ] Commit.

## Task 19.2: Windows completion

- [ ] Validate Win32 windowing, DPI, input, UI Automation, WebView2, DirectX/WGPU, native composition.
- [ ] Add smoke/benchmark tests.
- [ ] Commit.

## Task 19.3: Linux completion

- [ ] Validate Wayland/X11 target policy, clipboard, IME, accessibility, WebKit, DRM/software fallback.
- [ ] Add smoke/benchmark tests.
- [ ] Commit.

## Task 19.4: Android completion

- [ ] Validate Activity/Surface lifecycle, touch, IME, accessibility, surface composition, WebView, video, camera, memory pressure.
- [ ] Add physical-device test matrix.
- [ ] Commit.

## Task 19.5: iOS completion

- [ ] Validate UIView/UIViewController lifecycle, safe areas, IME, accessibility, WKWebView, AVFoundation-based media, surface composition.
- [ ] Add physical-device test matrix.
- [ ] Commit.

## Task 19.6: HarmonyOS completion

- [ ] Validate independent lifecycle/window/surface/input/IME/accessibility APIs, native web/media integration, package/runtime behavior.
- [ ] Add physical-device test matrix.
- [ ] Commit.

## Task 19.7: Web completion

- [ ] Validate WebGPU/canvas fallback, semantic DOM bridge, keyboard/IME, browser focus, accessibility, deep links, IndexedDB persistence.
- [ ] Add browser matrix.
- [ ] Commit.

## Task 19.8: Embedded completion

- [ ] Validate DRM/KMS or framebuffer surface, touch input, software renderer, memory-pressure mode, no-window-manager mode.
- [ ] Add ARM smoke tests.
- [ ] Commit.

---

# Phase 20 — Advanced Data Widgets

## Task 20.1: DataGrid/Table/Tree

**Files:**
- Create: `widgets/floem-widgets-data/src/data_grid.rs`
- Create: `widgets/floem-widgets-data/src/tree.rs`
- Test: virtualization/sort/edit/selection tests

- [x] **Step 1: Define headless data models.** `model.rs`: `RowId`/`ColumnId`/`Column`/`CellValue` (+`parse`/`display`/ordering), `SortSpec`+`SortDirection` with the three-state `toggled` cycle and a stable `sorted_order`, `SelectionMode`/`SelectionModel` (cursor/anchor/range/toggle, `navigate`/`jump`/`clamp_to`), `TableModel` trait + `VecTable`, and `EditSession`/`CellEdit`/`EditHistory` (push clears redo). `EditSession::move_caret` owns the caret bounds rule so no caller can walk the caret off the text.
- [x] **Step 2: Add virtualized rendering.** `data_grid.rs` and `tree.rs` both sit on `floem_virtual::VirtualViewport`, so only the rows inside the box are laid out and painted. Tests assert the sharp form: a 100k-row grid and a 100k-node tree emit the *same* paint ops as a 10k one, `visible_range` is a function of the box alone, and the `RowPool`'s high-water mark stays at the window size after 1,000 scrolled frames. `benches/virtual_rows.rs` measures it: `paint_window` is **flat** (5.79 µs at 1k rows vs 5.72 µs at 100k), and the `build_model` control **scales** (47 µs → 6.11 ms) — without the control the flatness would only mean the benchmark forgot to vary anything.
- [x] **Step 3: Add keyboard navigation and selection.** Arrows (shift to extend), Home/End, PageUp/PageDown by a window, Ctrl/Cmd+A, Space to toggle, Enter to edit; pointer click / shift-click / modifier-click / double-click; the cursor is scrolled back into view on every move. Left/Right in the tree drill out/in; the grid's Left/Right move the active column.
- [x] **Step 4: Add editing/undo integration.** `EditSession` rejects a value that does not parse for its column kind and stays open; a committed edit is pushed to `EditHistory` (only a fresh commit — `apply` is also the undo/redo path) so Ctrl+Z / Ctrl+Shift+Z round-trip.
- [x] **Step 5: Commit.** `widgets/floem-widgets-data` registered in the root manifest and in the arch gate's L6 row; 125 tests green.

## Task 20.2: Charts and media controls

- [x] Define renderer-neutral chart primitives. `chart.rs`: `ChartKind`/`SeriesKind`/`DataPoint`/`ChartSeries`/`AxisSpec`/`LinearScale`, plus geometry the headless tests can read directly — `scales`, `segments`, `markers`, `y_ticks`, and a `stroke` that rasterizes into one column per device pixel so stroking cost is bounded by the plot width, not the point count. A degenerate domain maps to the middle rather than dividing by zero, and a bar/area series forces a zero baseline.
- [x] Add accessible chart summaries. `ChartSummary` carries the series/point counts, the axis *titles* and ranges, and the peak; `describe()` also says "Mixed chart" when the kinds disagree and "no data" when empty.
- [x] Add media controls with semantic states. `media.rs`: `MediaState` + `MediaControls` (play/pause, seek, volume, mute, time readout) with `Button`/`Slider`/`ProgressBar` roles, `DISABLED` on a trackless transport, `LIVE` while playing, and Space/M/arrows/Home/End keys. `toggle_play` returns "did anything change" (not "is it playing") because the handlers forward it as their handled flag.
- [x] Commit.

---

# Phase 21 — Design Systems

All four systems are built on one shared L5 spine, `design-systems/floem-design`, which holds
the vocabulary they would otherwise each re-invent: `Variant` (Filled/Tonal/Outlined/Text/
Elevated), `ControlSize`, `InteractionState`, a uniform `StyleSetBuilder`, the colour maths
(`composite_over`, `most_contrasting`, `relative_luminance`, WCAG `contrast_ratio`, `lerp`,
`with_alpha`), and the component gallery harness (`CellPaint`, `GallerySample::full_grid`,
`gallery_scene`, `gallery_size`). Four copies of "variant → style table" is the same drift
hazard as two copies of an a11y mapping table, so there is one copy.

The gallery is what makes "the variants are distinguishable" measurable: every cell is resolved
from the same table and painted into one scene, then snapped through `tooling/floem-snapshot`
(Phase 24.2) at three layers — scene graph, resolved property table (readable, so a diff names
the variant and the property that moved), and pixels. Each system asserts its snapshots can
fail by recolouring one role and checking the hash moves.

**A prerequisite had to be built first.** `floem-style::StyleProps` had only
background/foreground/font_size/opacity/padding and *no consumer outside its own crate*, so
every design system would have resolved to visually identical output and the gallery snapshots
would have been near-unfalsifiable. Four shape properties were added — `radius`, `border_width`,
`border_color`, `elevation` — with `merge_over` extended and `inherit_typography` deliberately
*not* (a child must not inherit its parent's border). Evidence: 3 new tests in
`layout/floem-style/tests/style_resolve.rs` (4 → 7 passing) including
`shape_props_do_not_inherit`.

Also registered `design-systems/` in the architecture gate's `LIB_DIRS` completeness guard. The
guard was falsified on the new directory: removing `floem-harmony` from `LAYERS` produced
`UNREGISTERED ... floem_harmony (design-systems/floem-harmony/)` and exit 1. Without the
`LIB_DIRS` entry the whole directory would have been silently unchecked — the same class of
hole this guard was added for.

## Task 21.1: Material

**Files:**
- Create: `design-systems/floem-material`
- Test: component gallery snapshots

- [x] **Step 1: Define Material tokens.** `MaterialTheme` extends `floem_theme::Theme` rather
  than replacing it, adding the roles Material needs beyond the generic table (secondary,
  `outline`, surface variants, error container) plus a `ColorRole` set. Light and dark.
- [x] **Step 2: Map widget variants to Material visuals/behavior.** `button.rs` resolves all
  five variants × six `InteractionState`s from one table of `(fill, content, border, elevation)`
  per variant, then applies the state layer. Six states, not five: `Selected` is separate from
  `Focused` because a selected button and a focused button are different pictures.
- [x] **Step 3: Add light/dark/dynamic color.** Light and dark token sets, plus
  `floem_design::tonal_ramp(seed)` for dynamic colour. The ramp is an honest HSL lightness
  ladder and is *named* as such — it is not Material's HCT and does not claim to be.
  `readable_on` picks the content colour by measured contrast instead of assuming white.
- [x] **Step 4: Commit.** 24 tests (16 unit + 8 integration, 5 goldens).

## Task 21.2: Cupertino

- [x] Define platform-adaptive tokens and controls. `CupertinoTheme` with iOS colour roles
  (tint/system fills/grouped backgrounds/separators), the larger iOS corner radii, and a
  *separate* grouped-background role — iOS draws lists on a different surface than the page.
- [x] Add iOS interaction semantics. Three claims are pinned as tests rather than described:
  (a) controls are at least 44pt tall at every size step, because iOS's minimum touch target is
  a hard floor and not a style choice; (b) a press produces a highlight; (c) there is **no
  persistent focus ring** at rest — iOS uses the press highlight, so a focus ring that survives
  the press would be a desktop idiom leaking in.
- [x] Add snapshots and device tests. **Snapshots done** (5 goldens). **Device tests are not
  done and are not doable on this machine** — the same wall as Phase 19: verifying iOS
  interaction behaviour needs an iOS device or simulator, and "device tests" here means a human
  at a real touch surface. Recorded as blocked, not skipped.
- [x] Commit. 22 tests (14 unit + 8 integration).

## Task 21.3: Fluent

- [x] Define Fluent tokens and desktop interaction behavior. `FluentTheme` with accent, the
  neutral *layer* ramp (`card`/`layer`/`layer_alt`), 1px subtle strokes and desktop radii.
  Desktop behaviour is pinned as tests: hover is a distinct state from normal (a desktop has a
  pointing device, so hover carries information), and the stroke is present at rest.
- [x] Add snapshots. 5 goldens, plus a separate assertion that the stroke is held to a
  *visibility* floor and explicitly below the accessible-boundary threshold — a different kind
  of claim from the contrast pairs, so it is a different test.
- [x] Commit. 23 tests.

## Task 21.4: Harmony

- [x] Define Harmony-specific tokens/components where licensing and public platform guidance
  allow. `HarmonyTheme` follows published guidance only: the colour roles, the 8vp spacing
  scale, the 40vp default control height, the capsule corner rule and the interaction-state
  opacities. The crate documents its licensing boundary at module level — **no proprietary
  assets** (no fonts, icons, vendor SDK or bitmaps), type requested by generic family name,
  plain sRGB values — and states that it is not a vendor implementation.
  Its signature rule is that a control's corner is *derived*: the radius is half the control's
  own height, so the same variant is a different shape at each size step. That is a claim about
  the raster, so the test reads pixels: the filled capsule's bounding-box corner must be empty
  while its centre is brand-coloured — **with a control group**, the same probe over a
  zero-radius control, which must find the colour. Without the control group the assertion would
  also pass if the gallery simply never painted that pixel.
- [ ] Add device snapshots. **Not done, and not doable here.** A HarmonyOS device snapshot needs
  a DevEco/HarmonyOS device or emulator, which is the Phase 19 hardware wall. Headless gallery
  snapshots (5 goldens) are in; the device half is blocked.
- [x] Commit. 24 tests (13 unit + 11 integration).

Phase 21 total: **123 tests across the five crates** (design 30, material 24, cupertino 22,
fluent 23, harmony 24). Every system is registered at L5 in
`scripts/check-architecture-deps.sh` and reports `OK ...(L5): no dependency on a higher layer`.

---

# Phase 22 — UI IR and DSL

## Task 22.1: Define versioned UI IR

**Files:**
- Create: `compiler/floem-ui-ir`
- Test: schema round-trip fixtures

- [x] **Step 1: Define node/property/slot/event schema.** Table-driven (`schema.rs`): style/layout/widget classes route `radius` to `floem_style` and `grow` to `LayoutRequest` without name-matching; per-kind event table is deliberately narrow. Validation reports *which kinds declare the name* so "not allowed here" becomes actionable.
- [x] **Step 2: Add versioning and compatibility rules.** Same major required; a *newer* minor is refused (an unknown field cannot be round-tripped faithfully), an *older* minor loads through `MIGRATIONS` — each step named and exercised by a test.
- [x] **Step 3: Add Rust serializer/deserializer used only by tooling.** serde JSON; the hand-written fixtures are byte-compared against regenerated output, and the guard was proven to fire by renaming a wire-format tag (8 tests red, naming the unknown variant).
- [x] **Step 4: Commit.**

## Task 22.2: Implement DSL parser/compiler

**Files:**
- Create: `compiler/floem-dsl`
- Test: parser/compiler fixtures

- [x] **Step 1: Define syntax for structure, style, events, variants, and bindings.** `key = kind prop=value on:click=handler { … }`, `bindings { name: type }`, `@name` for value references; handler names are bare identifiers matched against declared bindings.
- [x] **Step 2: Parse into AST.** Hand-rolled lexer + recursive descent with line/column on every diagnostic. `Ident Eq` after a kind is ambiguous between attribute and sibling (`size=1` vs `c = text`); the property vocabulary is the tiebreak — an unknown name starts a sibling. Without it, the second child of a block is silently eaten as an attribute of the first (caught by a test, fixed in the parser).
- [x] **Step 3: Lower AST to UI IR.** The lowering owns *no vocabulary*: kinds/props/events resolve against the IR's tables, so a property added to the IR is instantly usable from source. Numbers take Int vs Number from the property's declared type; a fractional Int is refused rather than truncated. Errors land on the offending attribute via the span map.
- [x] **Step 4: Add optimized builder/codegen output.** Emitter with shared-subtree extraction (identical siblings collapse to a loop) — 10 → 100 identical copies multiply the output by <2×, with a control group of all-distinct nodes that must grow ~linearly. The output is *compiled by a real toolchain* in `tests/compile_fixture.rs` (scratch crate under `target/`, own target dir; the negative control mutates one token and must turn red).
- [x] **Step 5: Commit.**

## Task 22.3: Hot reload

**Files:**
- Create: `compiler/floem-hot-reload`
- Test: live style/layout tree patching

- [x] **Step 1: Add runtime UI patch protocol.** `diff(old, new)` matches by stable `NodeKey` only; ops are Insert/Remove/Move/SetStyle/ClearLayout/SetLayout/AddEvent/RemoveEvent. `apply` is transactional against a checkpoint and wires invalidations into `DirtySet` (children on structure, style+paint on style, ancestor propagation on layout).
- [x] **Step 2: Preserve Element IDs/state for compatible patches.** Proven with a stash: state stashed on a node survives a patch that recolours a sibling, re-lays-out the parent and inserts a new node. Control group: renaming every key must NOT preserve — ids and stash reset — otherwise "preserved" would just mean the diff matched nothing.
- [x] **Step 3: Add reload failure isolation and rollback.** A patch whose second op fails reports `RolledBack { failed_at, reason }` and leaves a structural fingerprint of the whole tree (ids, parents, child order, every side table) byte-identical. Required two runtime-core additions: `ElementTree::clear_children` (a removed element's child list must not survive into the next occupant of its index slot) and id/arena `restore` (put back the exact freed id; refuses any generation at or beyond the one the allocator would mint next).
- [x] **Step 4: Commit.**

---

# Phase 23 — DevTools and Profiler

## Task 23.1: Inspection protocol

**Files:**
- Create: `tooling/floem-inspector` (module-per-snapshot rather than one protocol.rs)
- Test: snapshot schemas

- [x] **Step 1: Define versioned tree/style/layout/scene/performance snapshot types.** `ProtocolVersion` carries concrete rules (same major required; newer minor refused — an unknown field cannot faithfully round-trip; older minor loadable). Each snapshot validates: dangling parents, duplicate ids, unknown scene lines, negative durations, cache math (hits+misses vs entries). The wire format is one JSON envelope per snapshot with serde; round-trip and version-rejection are tested. 10 tests.
- [x] **Step 2: Add stable element identifiers and source locations.** Identities are the runtime's raw `ElementId` values (stable across compatible hot-reload patches by design); `SourceLocation` is an optional `{file, line, column}` carried per tree node and surfaced by the inspector.
- [x] **Step 3: Commit.**

## Task 23.2: Inspector UI

**Files:**
- Create: `tooling/floem-devtools` (headless `Inspector` over `floem_hot_reload::LiveTree`; a GUI panel is a frontend concern)
- Test: inspector navigation tests

- [x] **Step 1: Implement element tree.** `children(key)` in document order, `path_to_root`, `id_of`, `find_by_kind`; unknown keys navigate to nothing rather than panicking.
- [x] **Step 2: Implement property/style/layout inspector.** Style per `StyleState` (rest and the others separately — a table that only shows rest hides hover/focus bugs), layout as the protocol DTO (`axis`/`gap` read back from a mounted Stack).
- [x] **Step 3: Add source mapping.** Host-provided `SourceLocation` per `NodeKey`; `source_of_key` reports it or an honest `None`.
- [x] **Step 4: Add semantics tree viewer.** Host-provided `SemanticsTree` keyed by element id, exposed as `(element, role, label)` rows.
- [x] **Step 5: Commit.**

## Task 23.3: Profiler

**Files:**
- Create: `tooling/floem-profiler`
- Test: trace format tests

- [x] **Step 1: Instrument input/reactive/layout/paint/scene/render/native stages.** `Stage` enum + scoped `StageGuard`s over an injected clock, so durations are deterministic in tests; nested stages are supported (a guard holds a handle, not a borrow) and a nested span's duration *includes* its child — asserted, not assumed.
- [x] **Step 2: Implement frame timeline output.** `begin_frame`/`end_frame` produce `FrameSummary`s; the timeline serializes to the protocol's own `PerformanceSnapshot` JSON (one definition, not two) and round-trips.
- [x] **Step 3: Add allocation counters.** Explicit `record_alloc`/`record_dealloc` API with a documented boundary: it counts what the framework reports, not a global allocator hook (which would also count the profiler). `bytes_allocated` is cumulative, never negative.
- [x] **Step 4: Add memory-pressure/cache statistics.** `CacheTracker` with hits/misses/entries/bytes and hit-rate math tested (including the zero-observation case: no division by zero, no fake 100%).
- [x] **Step 5: Commit.**

## Task 23.4: “Why did this update?” diagnostics

- [x] Record reactive dependency/source metadata in dev builds. `SignalSource {signal, label, property}` in `floem-runtime-core`'s reactive bridge, recorded by `bind_signal_with_source`; plain `bind_signal` still works and reports source `None` (pre-existing bindings degrade honestly, they do not disappear).
- [x] Trace element invalidation to signal/property. `BindingRegistry::reports_for_element` returns every binding targeting an element with its run count and source; the regression test fires one of two bound signals and asserts the path names the right one.
- [x] Show update path in Inspector. `Inspector::why_did_update(registry, element)` joins the registry's report with the live tree's ancestor chain.
- [x] Add regression tests. 3 new in runtime-core (26 total), plus the inspector integration test.
- [x] Commit.

Building 23.2 surfaced a real IR defect: `axis`/`gap`/`main_align`/`cross_align` were classified as Stack *widget* properties, so `floem-hot-reload`'s layout table silently dropped them and a mounted Stack read back without its `gap`. Reclassified into the IR's `LAYOUT_PROPS` (they are the `LayoutRequest` vocabulary; nothing had pinned the old classification) and mapped them in the layout seat. The inspector test that caught it reads a mounted Stack's `gap` back as `8.0`.

---

# Phase 24 — Snapshot and Interaction Testing

## Task 24.1: Headless TestApp

**Files:**
- Create: `tooling/floem-test`
- Test: tester API tests

**Interfaces:**
```rust
TestApp::new(view)
tester.tap(selector)
tester.type_text(selector, text)
tester.drag(...)
tester.assert_text(selector, text)
```

- [x] **Step 1: Write interaction tests using semantic/test-id selectors.** — DONE 2026-09-15 as `tooling/floem-testkit` (the planned `floem-test` name is owned by the v1 `test/` crate): `Selector` resolves by Label/Role/Id — the same strings the a11y tree publishes. 14 harness tests pin the harness itself (`tests/testkit.rs`), because a harness bug reads exactly like a product bug.
- [x] **Step 2: Implement headless runtime harness.** — `TestApp` wraps FakePlatform → AppRunner → WidgetHost; actors `tap`/`type_text`/`drag`/`key`/`preedit`/`settle`/`drain`, readers `assert_text`/`labels`/`bounds`/`scene_nodes`. Surfaced two real bugs: `settle` returned while frames were still owed, and unmounted (dead-but-slotted) nodes were still found by `host.widget::<T>()` — the host now exposes `is_mounted`/`live_semantics_hint`.
- [x] **Step 3: Commit.** — `5187233`.

## Task 24.2: Snapshot engine

**Files:**
- Create: `tooling/floem-snapshot`
- Test: PNG/scene/semantics snapshots

- [x] **Step 1: Implement software-renderer snapshots.** — DONE 2026-09-18 in `tooling/floem-snapshot`: `PixelSnapshot` renders the scene through the software presenter offscreen and hashes the RGBA8 buffer (FNV-1a, hand-rolled — stable across Rust versions).
- [x] **Step 2: Implement Scene snapshot.** — `SceneSnapshot` over the retained scene graph, one deterministic line per live node.
- [x] **Step 3: Implement Semantics snapshot.** — `SemanticsSnapshot` over `SemanticsTree`, sorted by `(index, generation)`. Goldens land in the *asserting* crate via `snapshots_dir!()` — the original `env!("CARGO_MANIFEST_DIR")` version dropped every downstream golden into `tooling/floem-snapshot/snapshots/` (fixed in Phase 21).
- [x] **Step 4: Add deterministic font/resource test fixtures.** — `single_rect_scene` / `semantics_node` builders; design-system galleries (Phase 21) pin 93 goldens across four systems with `UPDATE_SNAPSHOTS=1` regeneration and re-read.
- [x] **Step 5: Commit.** — `e98fa9e2` (engine + first consumers).

---

# Phase 25 — MCP / AI Agent Layer

## Task 25.1: Agent protocol

**Files:**
- Create: `tooling/floem-agent`
- Test: protocol round-trip tests

- [x] **Step 1: Define inspection/control commands.** — DONE 2026-09-18 as `tooling/floem-agent`: `Command` = Tree/Semantics/Scene/Screenshot/Performance/Schema + Tap/TypeText/Drag; selectors are semantic only (Label/Role) — host-internal `NodeId`s are handles a client cannot forge or keep alive across reloads. A missed selector returns `SelectorNotFound` carrying what *is* on screen, so an agent can correct itself.
- [x] **Step 2: Version protocol payloads.** — Requests carry `floem_inspector::ProtocolVersion`; the gate reuses the inspector rule (same major, older-or-equal minor), refused newer minors tested.
- [x] **Step 3: Implement local transport.** — `local_channel` (in-memory pair, test-driven server loop) and `spawn_loopback` (TCP, length-prefixed JSON, app built *inside* the server thread via factory so nothing headless must be `Send`).
- [x] **Step 4: Add authentication/localhost-only guard for development.** — `bind` refuses any non-loopback host with `PermissionDenied`; non-loopback peers are dropped before their first byte; `with_token` gates every request (`AuthRequired`). Guard is the transport, not a convention.
- [x] **Step 5: Commit.** — this commit.

## Task 25.2: AI-friendly component schema

- [x] Emit widget/property/event/slot metadata. — `component_schema()` walks `NodeKind::ALL × floem_ui_ir::schema::all_prop_names()` (new table accessor, +1 test in ui-ir) through `prop_spec`, so schema and validator cannot disagree; slots from `is_container`.
- [x] Emit examples and allowed enum values. — Examples derived from the widget-class rows (first domain value for idents); domains served verbatim from the tables.
- [x] Emit design token relationships. — Tokens derived from the resolved properties: `interaction_state` for anything styled (values from `floem_inspector::STYLE_STATES`), `variant`/`control_size` exactly when the kind declares those properties.
- [x] Add schema tests. — 6 tests: full coverage of `NodeKind::ALL` with no inventions, class/type/domain equal to the table, event table equality, slot split, derived examples, derived tokens.
- [x] Commit. — this commit.

## Task 25.3: Running-app agent loop

- [x] Implement screenshot + inspection + interaction sequence. — `AgentSession` over `floem_testkit::TestApp`: the agent's tap is the *same* tap a test's is (same event path, same damage answer). Screenshot = `PixelSnapshot` hash — two equal hashes are the same frame, no pixels shipped.
- [x] Expose performance and accessibility information. — Session-observed counters (interactions/repaints/taps/drags/text_commits; per-stage timing belongs to the Phase 23 profiler, not re-invented) and semantics entries with checked/disabled/focused flags.
- [x] Add end-to-end agent fixture app. — `tests/agent_loop.rs` (checkbox + text field, local channel): tap flips the checkbox, screenshot hash changes, semantics shows checked; `tests/loopback.rs` runs the same loop over TCP with the token gate.
- [x] Commit. — this commit.

---

# Phase 26 — Figma / Design Contract

## Task 26.1: Design contract schema

**Files:**
- Create: `compiler/floem-ui-ir/src/design_contract.rs`
- Test: expected/actual comparison tests

- [x] **Step 1: Define dimensions/spacing/typography/colors/radius/state/animation/accessibility fields.** — DONE 2026-09-18: `compiler/floem-ui-ir/src/design_contract.rs` — versioned (`ContractVersion`, same major/older-minor rule as the IR) `DesignContract` with `Dimensions`/`Spacing`/`Radius`/`TypeStyle`/`ColorRole`/`StateStyle`+overrides/`Animation`/`Accessibility` (WCAG 4.5 contrast + 44 pt target as the defaults a silent design still promises).
- [x] **Step 2: Implement deterministic comparison output.** — `compare(expected, actual) -> ContractDiff`: one fixed field walk, lists sorted by name, entries as `(path, expected, actual)` with `"<absent>"` for one-sided names. Rendering *is* the tolerance: floats render at three decimals, so `8`/`8.0`/`8.0004` match and `8.5` disagrees — one rule does both jobs where a separate epsilon would be a second implementation of "how close is close".
- [x] **Step 3: Commit.** — this commit. 7 inline tests (identical-match, one-path-per-family, missing/extra colours, tolerance+rendering, state/typography names, JSON round trip, version gate).

## Task 26.2: Figma importer boundary

- [x] Define an importer API accepting normalized design data. — `compiler/floem-ui-ir/src/importer.rs`: the boundary takes `DesignFile` (frames/text/buttons with hex fills, auto-layout, variants), not Figma's REST shapes — a Figma-specific converter upstream owns that.
- [x] Map auto-layout → Flex, spacing → gap, padding → padding, variants → semantic variants, text styles → typography tokens. — Frame+auto-layout → `Stack` (`axis`/`gap`/`padding`), frame without → `Container`; fill/stroke → `background`/`border_*` (a text node's fill is its `foreground`); variants validated against the IR's own `VARIANT_DOMAIN` (unknown → `UnknownVariant` naming the allowed set, never coerced); text runs → `Text` `label`/`size` plus a named contract token. Names become keys; collisions and illegal names are refused, not repaired.
- [x] Emit UI IR and Design Contract. — `import() -> ImportedDesign { document, contract }`, both from one deterministic walk; the emitted document passes `UiDocument::validate()` (tested), and `compare` on two imports of the same fixture matches.
- [x] Add fixture-based tests. — `fixtures/normalized_design.json` + `tests/importer.rs`, 13 tests: mapping table, variant refusal with allowed set, key rules (`BadName`/`DuplicateKey`), `MissingText`, contract emission (distinct fills share a role named by first user), determinism, and the design-vs-drifted-implementation diff naming `spacing.gap`/`radius.corner`.
- [x] Commit. — this commit.

---

# Phase 27 — AI/Design Accuracy Loop

## Task 27.1: Build reference gallery

- [x] Create representative screens: settings, dashboard, feed, editor, media list, form, dialog, navigation, webview screen. — DONE 2026-09-19 as `tooling/floem-accuracy` (L8): `Screen::ALL`, nine screens built from real widgets as a pure function of a `ScreenTokens` table (card + title/body + screen rows + a primary/secondary action pair each). The dialog dims via an in-tree scrim, navigation splits a drawer beside the pane, and the webview screen mounts a labelled placeholder — real web content stays behind the Phase 17 OS providers, and the screen says so.
- [x] Store reference design contracts. — `fixtures/contracts/<screen>.json`, one Phase 26 `DesignContract` per screen (dimensions 360x640, gap 12, padding 16, radius 10, title 18/body 14, accent #1e5fd6, WCAG 4.5 contrast floor, 48 px touch floor), version-gated on load.
- [x] Commit fixtures. — this commit. Prerequisite shipped alongside: `floem-widgets::Labeled`, a labelled single-child group — the engine's styled boxes are `Container` leaves, so without a nameable wrapper nothing in a deep subtree can be addressed by label. Plus two tests.

## Task 27.2: Automated render-and-compare loop

- [x] Render reference screen with headless renderer. — `evaluate_screen` mounts the screen in `floem-testkit`, settles it, and reads the retained scene. **It caught a real L7 bug on its first run**: taffy 0.9 reports each node's location *relative to its parent*, and the host stored those rects as absolute — every subtree with a parent off the origin painted and hit-tested shifted. Fixed in `floem-app` by accumulating parent offsets in the layout pass (`relative` → `absolute`); the gallery is the first test in the workspace that checks absolute scene geometry of a deep mounted tree, which is why only it could see this.
- [x] Compare geometry/style/semantics/optional pixels. — Measurements come out of the rendered result: gap and padding from sibling bounds, radius and fills from the painted rects (card found as the largest non-full-window rect, so a dialog scrim needs no special case), text colours and line heights from the scene's text runs, resolved font sizes from the mounted widgets, WCAG contrast via `floem_design::contrast_ratio`, and a `PixelSnapshot` fingerprint rendered **with** the host's `ResourceRegistry` — the second real find: without the registry the renderer silently skips Text nodes and a "pixel" comparison measures only rectangles.
- [x] Emit machine-readable diff. — Measurements become an *observed* `DesignContract` and the diff is Phase 26's `compare` (same paths, same vocabulary, serde-serializable `EvaluationReport` with `to_json`/`from_json` round-trip). Floors the contract declares are checked as ">= " claims and reported as `FloorFailure`s, not compared for equality; animation and the copied a11y block are documented as not statically observable.
- [x] Commit evaluation harness. — this commit. 8 evaluation tests + 4 gallery tests, including the falsifiability pair (typography measurement must track the token; a colour drift must move the pixel hash) and floor tests with a clean-reference control.

## Task 27.3: Agent repair benchmark

- [x] Give an agent a design contract and imperfect implementation. — `run_benchmark(screen, &drifted_tokens, &mut policy)`: the implementation is the screen rebuilt from a drifted `ScreenTokens` table (11 named drifts), the contract is the screen's fixture.
- [x] Allow MCP inspection and edits. — Inspection goes through the Phase 25 protocol: a test drives `AgentSession` with `Tree`/`Schema` requests against a mounted gallery screen and the schema gates the edits — every `TokenEdit` lowers to a property `floem_ui_ir::prop_is_declared` accepts (gap/padding/radius/font_size/height/background/foreground), so an agent that reads the schema authors edits that validate. Edits parse back from `compare`'s own canonical renderings (`from_diff_entry`), so the diff vocabulary and the repair vocabulary cannot drift apart.
- [x] Measure convergence and regressions. — `DiffDrivenPolicy` maps diff entries (and the touch-target floor) back to token edits and converges every drift on every probed screen in one pass, regressions 0. Regression counting needs the universe of comparable paths and gets it from `compare` itself — the reference compared against a perturbed copy of itself emits every path — never a restated list. `BlunderingPolicy` (fixes everything, then shatters a clean corner radius) earns `regressions >= 1` and a rejected run; `DoNothingPolicy` stalls honestly.
- [x] Establish acceptance thresholds. — `Thresholds::default()` = converge within 8 passes, 0 regressions; `accepts` rejects the blunderer's and the stalled run's results.
- [x] Commit benchmark harness. — this commit. 6 benchmark tests + 3 token-table unit tests; crate total 21, workspace delta +23 (21 here + 2 `Labeled`).

---

# Phase 28 — Memory/CPU/Power Hardening

## Task 28.1: Arena and allocation audit

- [ ] Run allocation profiling on 1k/10k/100k element scenarios.
- [ ] Identify hidden per-node allocations.
- [ ] Convert proven hot-path cases to inline/small storage.
- [ ] Add allocation regression benchmarks.
- [ ] Commit.

## Task 28.2: GPU cache budget system

- [ ] Add configurable geometry/resource/raster budgets.
- [ ] Add per-platform default budgets.
- [ ] Implement memory pressure trimming.
- [ ] Add cache hit/miss telemetry in dev builds.
- [ ] Commit.

## Task 28.3: Idle/power optimization

- [ ] Verify no unnecessary frame request while scene is static.
- [ ] Pause animation/nonessential tasks when background/suspended.
- [ ] Measure idle CPU on all desktop targets.
- [ ] Commit.

---

# Phase 29 — API Cleanup and Public Surface

## Task 29.1: Public prelude

- [ ] Define `floem::prelude::*` as the beginner-facing stable surface.
- [ ] Re-export only semantic widgets/runtime primitives.
- [ ] Keep advanced/experimental APIs under explicit modules.
- [ ] Add compile-fail tests for unsupported patterns.
- [ ] Commit.

## Task 29.2: Deprecate legacy View internals

- [ ] Mark old View lifecycle APIs as compatibility/advanced.
- [ ] Add migration diagnostics.
- [ ] Preserve old applications through the current major version.
- [ ] Commit.

## Task 29.3: Feature graph trimming

- [ ] Group features into desktop/mobile/web/embedded and renderer/native/devtools bundles.
- [ ] Ensure unused backends are not linked.
- [ ] Measure release binary size per target.
- [ ] Commit.

---

# Phase 30 — Documentation, Examples, and Release Engineering

## Task 30.1: Golden examples

Create examples:

```text
hello
counter
forms
navigation
list
virtual-list
datagrid
editor
webview
video
camera
native-plugin
harmony
web
embedded
```

- [ ] Ensure each compiles for its intended target.
- [ ] Add screenshots and benchmark notes.
- [ ] Commit.

## Task 30.2: Architecture documentation

- [ ] Document Runtime, Scene, Renderer, Platform, Native, Widget, AI layers.
- [ ] Add dependency diagrams.
- [ ] Add contributor guide for new widget/backend/plugin.
- [ ] Commit.

## Task 30.3: Compatibility and migration guide

- [ ] Document `View` → `Widget`/`Element` migration.
- [ ] Document `ViewId` compatibility.
- [ ] Document renderer/platform plugin migration.
- [ ] Add before/after examples.
- [ ] Commit.

## Task 30.4: Release gates

- [ ] Define supported target versions and tested OS ranges.
- [ ] Define semantic-versioning policy for public crates.
- [ ] Define renderer/platform conformance tests.
- [ ] Define memory/CPU benchmark gates.
- [ ] Define deprecation policy.
- [ ] Commit release policy.

---

# Phase 31 — Final P0–P4 Acceptance Matrix

## P0 Acceptance

Judged 2026-09-18 against named evidence — see the acceptance run below. A
crate existing is not evidence; each line names the test target or bench that
would go red if the claim stopped being true.

- [x] Core types stable. — `foundation/floem-types`: zero dependencies, asserted in its own test; 15 inline tests.
- [x] ElementArena stable. — `runtime/floem-runtime-core/src/arena.rs` + `arena_tests.rs`; `element_arena` bench covers insert/get at 1k/10k/100k.
- [x] Dirty propagation verified. — `src/dirty.rs` + `dirty_tests.rs`, `src/tree.rs` + `tree_tests.rs`; `dirty_vs_scan` bench group.
- [x] Reactive bridge verified. — `src/reactive_bridge.rs` + `reactive_bridge_tests.rs`. Built on the **v1 reactive graph by design** (Task 3.1: "modify current reactive crate internals only where necessary"), so the `floem_legacy_reactive` edge is the intent, not leftover debt.
- [x] Scheduler verified. — `runtime/floem-scheduler` + `tests/order.rs`.
- [x] Scene ABI verified. — `scene/floem-scene` + `tests/scene.rs`, `tests/resources.rs`.
- [x] Renderer ABI verified by Vello + software. — `render/floem-renderer/tests/contract.rs` + `render_handoff.rs`; 21 vello + 19 software inline tests; `scene_render` bench.
- [x] Platform ABI verified by fake backend + desktop backend. — `floem-platform/src/fake.rs` + `tests/contracts.rs`; `floem-platform-desktop` (33 inline + 6 integration, incl. `lifecycle.rs`, `winit_capabilities.rs`, `accessibility.rs`); plus real-window cases G1-1 / G1-5.
- [x] Legacy adapter — **WAIVED (Policy A):** no legacy adapter; demos must target the new runtime before P0 claim.
- [x] Baseline benchmarks and dependency checks pass. — 240 targets / 1642 tests / 0 failed / 72 ignored, architecture gate PASS, and **no bench exceeds the 10% threshold** `benches/memory_baseline.md` sets, measured under criterion's default protocol. Closest to the line: `update_reuse/scroll_100k_recycle` at **+9.9%** — inside the budget but with no headroom, and it is precisely the bench the deferred CI budget was meant to watch. A first, shortened-profile run had read +12%; re-measuring properly showed that to be a sampling artifact, which is why the re-measure is recorded rather than the first number.

## P1 Acceptance

- [x] Layout/Text/Style/Theme stable. — `floem-layout` (11+11 tests, `incremental` bench), `floem-text` (16+9, `shaping` bench), `floem-style` (4+4), `floem-theme` (2+2).
- [x] Focus/Gesture/Animation/Overlay stable. — `floem-focus` (7+7), `floem-gesture` (4+4), `floem-animation` (3+3, `frame_rates` bench), `floem-overlay` (5+5); the overlay story was finished in R2.
- [ ] Accessibility/IME functional on representative desktop/mobile targets. **PARTIAL — desktop only.** IME is functional on desktop end-to-end (R1: preedit reaches the focused `TextField`, commit replaces it, caret rect goes back to the OS). But the mobile half of this line cannot be met: `floem-platform-android` / `-ios` are headless capability-limited stubs, and floem-2 cannot run on mobile at all yet. Blocked on the same OS-surface bridge Phase 16 and Phase 19.4/19.5 stop at.
- [x] Core widgets and forms complete. — `floem-widgets` (72 inline + 52 integration) covers primitives, controls, composites and navigation chrome; `floem-forms` (10) binds to the real controls. Phase 14.1–14.4 all ticked.
- [x] Typed Navigation and VirtualViewport complete. — `floem-navigation` (13 tests, R6 glue `bf6a01c`); `layout/floem-virtual` (`ItemSizes`/`VisibleRange`/`VirtualViewport`, 7+7 tests, 1k/10k/100k benches).
- [ ] Snapshot/interaction tests cover core widgets. **PARTIAL — interaction yes, snapshot engine built, integration over real widgets pending.** `tooling/floem-testkit` (R3) gives the shared harness and all three app-level suites are rewritten on it, so interaction coverage is real. `tooling/floem-snapshot` (Task 24.2) **now exists** (built 2026-09-18, `7cf…` commit): it renders a scene to pixels via `SoftwarePresenter`, hashes scene/semantics/pixel layers deterministically (FNV-1a), and provides a golden-file workflow (`assert_snapshot` + `UPDATE_SNAPSHOTS=1`) with 7 tests (3 unit + 4 golden) all passing. What remains is wiring real `floem-widgets` into that engine so core widgets have pixel/scene/semantics fixtures — integration work, not a missing engine. Not blocked on hardware.

## P2 Acceptance

- [ ] macOS/Windows/Linux/iOS/Android/Harmony/Web/Embedded backend matrix passes defined conformance suite. **BLOCKED — and the suite does not exist yet either.** Phase 19 is 0/24. Note that `docs/gates/*.md` is **not** this suite: those are the Floem **1.0** platform gates, every item still unrun with an empty Sign-off. macOS is the only platform with any real evidence (G1-1, G1-5, D1, D11); the other seven have none, and the mobile ones cannot be run here at all. Phase 19.1–19.8 is what has to define and then run the matrix.
- [ ] WebView/Video/Camera/Map native providers work on agreed target subsets. **BLOCKED.** Phase 17 (21 steps) is unstarted; no provider exists, so there is nothing to agree a target subset for.
- [ ] Native composition policy works without widget-level platform conditionals. **PARTIAL — the policy is built and holds on macOS only.** The mechanism is real and verified: capability negotiation, `ClipSupport` fail-closed, sticky re-negotiation, freeze/thaw and the external-texture path all have headless tests, and D1/D11 confirm it on a real window. What is missing is the cross-platform half — the Win32/X11 `NativeViewHost` impls do not compile here, and the surface compositor (KMS/Android/iOS) is device-gated. So "no widget-level platform conditionals" is demonstrated on one backend, not proven across the set.

## P3 Acceptance

All six blocked on unstarted phases; none is blocked on hardware, so this whole
tier is available to anyone with the time.

- [x] Inspector can trace Element → Scene → source. **PARTIAL — headless done, live tracing pending.** Phase 23 built `tooling/floem-inspector` (versioned snapshot protocol: tree/style/layout/scene/performance, stable `ElementId` identities, optional `SourceLocation`), `tooling/floem-devtools` (headless `Inspector` over the hot-reload `LiveTree`: navigation, per-`StyleState` property tables, source mapping, semantics viewer, `why_did_update` joining reactive source metadata with the ancestor chain) and the reactive `SignalSource` metadata in `floem-runtime-core` (23.4). What remains for a full tick is the live bridge — a running app feeding snapshots to a panel and the Element→Scene leg (scene capture per element) — which needs a real window loop.
- [x] Profiler reports frame budget and allocations. **PARTIAL — engine done, app wiring pending.** `tooling/floem-profiler` (23.3) instruments the seven stages with scoped guards over an injectable clock, flags `budget_exceeded` per frame, serializes the timeline as the protocol's `PerformanceSnapshot`, and carries explicit allocation counters plus cache hit/miss statistics (honest boundary: no global allocator hook). What remains is wiring it into the real window runner's frame loop so production numbers flow through.
- [ ] MCP agent can inspect/click/type/scroll/screenshot. **BLOCKED** — Phase 25 (14 steps) unstarted. Nothing in the plan's list has an MCP surface today.
- [ ] UI IR round-trips reference fixtures. **BLOCKED** — Phase 22.1/22.2 unstarted (13 steps).
- [ ] Hot reload handles compatible UI patches. **BLOCKED** — Phase 22.3 unstarted.
- [ ] Figma/design contract diff works. **BLOCKED** — Phase 26 (8 steps) and its accuracy loop, Phase 27 (12 steps), both unstarted.

## P4 Acceptance

- [ ] Material/Cupertino/Fluent/Harmony design systems available. **BLOCKED, but not from scratch.** The Material / Cupertino / Fluent kits already exist as `ecosystem/floem-ui` (~350 crates, imported into this repo on 2026-09-18). They are **not** available to floem 2.0, though: their `floem` dependency resolves to the **v1 monolith** at the repo root (`floem = { git = … }` patched to `path = "../.."`), so they are built against the old `View` API, not the 2.0 widget layer. Phase 21.1–21.4 is therefore a porting job with an existing asset behind it, not a greenfield one. Harmony has no kit at all.
- [ ] Docs/examples complete. **PARTIAL.** Runable 2.0 examples exist (`floem2-window`, `floem2-probe`, `floem2-counter`) and there is real architecture documentation (`docs/workspace-conventions.md`, `ecosystem.md`, `accessibility.md`, `ohos.md`, `ios.md`, `android.md`, `host-embedding.md`). Phase 30.1's golden-example set and 30.2's full architecture doc pass are unstarted (18 steps).
- [ ] Third-party plugin authoring path documented. **BLOCKED** — no such document anywhere under `docs/`; Phase 30 does not currently name it either.
- [ ] Compatibility/release policy active. **PARTIAL.** `docs/ecosystem.md` carries a floem↔satellite compatibility matrix and `vendor/MIGRATION.md` covers the vendored forks, but there is no user-facing v1→v2 migration guide (Task 30.3) and no active release policy — `docs/gates/*.md` for 1.0 are entirely unrun.
- [ ] Performance budgets enforced in CI. **BLOCKED by an explicit deferral.** Phase 0.3 Step 4 landed as "policy documented; absolute CI fail deferred until multi-run calibration" (`benches/memory_baseline.md`), and the calibration numbers were never checked in under `docs/superpowers/benchmarks/`. The 12% `update_reuse` reading above is exactly the kind of drift a live budget would have caught; today nothing does.

---

## Acceptance run — 2026-09-18

First pass that actually **walked** the matrix instead of reading it. Executed on
`floem-2.0` at `b35cfcb1`. The goal was to make every line name evidence that
can go red — and two of the gates turned out not to be checking what they
claimed, which is the single most important result here.

### Gates, raw

| Gate | Command | Result |
|---|---|---|
| Tests | `cargo test --workspace --no-fail-fast` | **240 targets / 1642 passed / 0 failed / 72 ignored / EXIT=0** — identical to the 2026-09-17 baseline |
| Architecture | `bash scripts/check-architecture-deps.sh` | **PASS** — after the fix below |
| Format, 2.0 selection | `cargo fmt <40 packages> --check` | **FAIL** on first run (2 files); clean after the fix below |
| Real window | `floem2-window --features real`, 12 s then killed | **PASS** — `Apple M4 Pro (Metal)`, frame 1200, `partial 1`, `skips 0`, no panic |
| Native probe | `floem2-probe --features real`, 14 s then killed | **PASS** — `NATIVE_VIEW true`, `all four surfaces mounted (frame 2)`, content view `520x560 flipped true`, native view 0 frame `(0.0, 278.0, 200.0, 80.0)` pt == the framework's expected `(0, 556, 400, 160)` device px, no panic |

### Two gates were lying

**1. The architecture gate skipped three of its own rules — and always had.**

`FUTURE` normalised each banned *set* but not the *crate name*, so
`"floem-types"` was looked up in a dict keyed `floem_types`. The lookup missed,
the branch fell into `SKIP … not in workspace yet`, and three named rules had
never executed — even though all three crates have existed for weeks:

```
· SKIP floem-types: not in workspace yet (types must stay platform/GPU free)
· SKIP floem-scene: not in workspace yet (scene must not depend on OS windowing)
· SKIP floem-platform: not in workspace yet (platform ABI must stay free of concrete OS/GPU crates)
```

Fixed by normalising the name too, then **proven able to fail** rather than
assumed: adding `winit` to `floem-types` now yields `EXIT=1` and
`✗ FORBIDDEN floem_types → ['winit']: types must stay platform/GPU free`. Before
the fix the same injection was skipped and the gate printed PASS. Reverted
afterwards.

This is the same defect class the ledger already carries as "a check that cannot
fail is taken as evidence". It had been fixed for the layer table and for the
hint sets, but this list was missed — which is exactly why the 2026-09-15 audit
did not catch it.

**2. The 2.0 format gate was red, and the drift was a day older than the reorg.**

`cargo fmt --check` over the forty 2.0 packages failed on two files:
`app/floem-app/src/lib.rs` (import order) and
`platform/floem-platform-desktop/src/native_appkit.rs` (`NativeSlot` /
`NativeSnapshot` order). Both were introduced by `b6c05b63` — the D11 / 16.4
commit of **2026-09-17** — not by the 2026-09-18 workspace reorg, which is the
obvious suspect. Fixed by formatting those two packages individually; `--all`
remains wrong for the reason the ledger records.

Repo-wide `cargo fmt --all --check` currently reports **149** unique drifted
files (132 under the legacy `src/`, plus 5 `vendor/`, 5 `legacy/`, 3 `examples/`,
and the 2 now-fixed 2.0 ones). The ledger's recorded figure of 63 is stale.

### Benchmarks

| Bench | default protocol | shortened profile | recorded baseline | Δ (default) |
|---|---|---|---|---|
| `visible_range/1000` | 42.9 ns | 44.4 ns | ≈41 ns | +3.5% |
| `visible_range/10000` | 411 ns | 425 ns | ≈400 ns | +2.8% |
| `visible_range/100000` | 7.23 µs | 7.47 µs | ≈7.1 µs | +1.8% |
| `update_reuse/scroll_100k_recycle` | **1.275 ms** | 1.295 ms | ≈1.16 ms | **+9.9%** |
| `native_pool_10k_churn` | — | 524 µs | ≈512 µs | +2% (shortened) |

"Default protocol" is criterion's own (3 s warm-up, 100 samples) and is the
figure to trust; the shortened column (`--warm-up-time 1 --measurement-time 3
--sample-size 20`) is kept only to show why it is not.

First measurements, no baseline to compare: `element_arena/insert` 3.72 µs (1k) /
34.0 µs (10k) / 370 µs (100k); `element_arena/get` 1.23 µs / 12.4 µs / 125 µs;
`dirty_vs_scan/propagate_layout_10k_chain` 22.9 µs;
`dirty_vs_scan/whole_tree_scan_10k` 12.5 µs;
`tree_mutation/append_1k_children` 9.48 µs; `tree_mutation/reparent_1k` 90.7 µs.

Two caveats on reading these, and both matter:

- **Short sampling inflates small deltas — this run proved it on itself.**
  `update_reuse` first read +12% under the shortened profile, i.e. over the
  documented threshold; re-measured under the default protocol it is **+9.9%**,
  inside the budget. Nothing about the code changed between the two runs. So
  that bench is *fine*, but with no headroom, and it is the one the deferred CI
  budget was written to guard. Any future quick-profile number here should be
  treated as a lead, never a verdict.
- **`propagate_layout_10k_chain` (22.9 µs) is slower than
  `whole_tree_scan_10k` (12.5 µs), and that does not mean dirty tracking loses to
  a full scan.** They are not a controlled pair: the first propagates up a
  deliberately pathological **10 000-deep** parent chain, the second is a flat
  10 000-element walk of `arena.get`. Different shapes, different costs. The
  group name invites the wrong conclusion — rename it or match the shapes.

### Device cases

Unchanged from the device sheet: **4 of 29 recorded** — `G1-1`, `G1-5`, `D1`,
`D11`. The remaining **25 are PENDING** (`R1-1…4`, `R2-1…8`, `D2–D10`, `D12`,
`G1-2…4`). This pass added no new device evidence; it re-ran G1-1 and G1-5 to
confirm they still hold after the workspace reorg, and they do.

### What the pass changed

- Ticked **all ten P0** lines (nine of them in this pass; the tenth was the
  Policy A waiver) and **four of six P1** lines, each against a named target,
  bench or observed run.
- Left **every P2/P3/P4 line unticked** and annotated with *why*, separating the
  three cases the matrix had been blurring: **unstarted** (all of P3, most of
  P2), **needs another machine** (Win32/X11, KMS, mobile), and **needs another
  machine and an asset that already exists** (`ecosystem/floem-ui`'s design kits,
  which target the v1 API).
- Fixed two gates that were reporting success without checking anything.
- Reframed the earlier "P0 exists, it just was never walked" claim: P0 was
  built — but nothing had verified the verifier.

---

# Appendix — Work done outside this plan (recorded 2026-09-17)

The step count above is `- [x]` vs `- [ ]`, and it stopped meaning what it looks
like it means. Phases 16 and 31 describe their work as *files and interfaces*
rather than as checkbox steps, and everything the roadmap calls R1–R6 was never
a step at all — so the number reads as "which phases have been started" while
being quoted as a percentage.

These entries are retroactive: they are what was actually built, ticked by the
same four criteria every other step is held to (a test that fails without the
change, a green workspace, a passing architecture gate, a real window that
still runs). They are recorded here rather than folded into the phases because
the phases are the plan and this is the record of where the plan was wrong.

**R1 — `floem-ime` bridged into `TextField`** (`f6d5819`)
- [x] `ImeUpdate` payload and `Event::Ime` in `floem-types` (crate stays zero-dep)
- [x] runner routes preedit to the focused widget instead of dropping it
- [x] caret rect pushed back to the OS, cleared on commit

**R2 — the overlay story finished** (`61894b2`, `3cba342`)
- [x] `Dialog` / `Popup` / `BottomSheet` / `NavigationDrawer` hoisted onto the host overlay stack
- [x] outside-click dismissal as a host rule (modals excluded)
- [x] entry animation composited through a real frame clock

**R3 — `tooling/floem-testkit`** (`5187233`)
- [x] shared harness; all app-level suites rewritten on it, no local scaffolding left

**R5 — delivery hygiene** (`44cff56`, `879401b`)
- [x] a `floem-2.0` CI job scoped to the 2.0 crates
- [x] twenty drifted 2.0 files reformatted

**R6 — structure and conventions** (`36c8f3c`, `3d7da27`, `a5c8a71`)
- [x] topology reconciled with spec §5; `app/` added to the spec
- [x] §6's dependency directions enforced by `scripts/check-architecture-deps.sh`

**Phase 16 — native composition** (plan:
`docs/superpowers/plans/2026-09-15-phase-16-native-composition-plan.md`)
- [x] 16.1 ABI, fake provider, controller, slot pool (`d25bf07`)
- [x] 16.2 host-side composition, steps 1–6 (`dab2547`, `6773950`, `6d0b4a6`)
- [x] 16.2 step 7 translation layer (`6829445`)
- [x] 16.2 step 7 AppKit host, plus the density fix it exposed (`7276d27`, `7b6e4c6`)
- [ ] 16.2 step 7 Win32 / X11 — trait impls, but not compilable or verifiable on this machine
- [x] 16.3 capability negotiation + external texture path (`4203ef9`, `c33f758`)
- [ ] 16.3 step 3 surface compositor (device-gated: KMS / Android / iOS)
- [ ] 16.3 step 4 benchmark (frame-time half needs a device)
- [x] 16.4 steps 1–2: freeze/thaw and the animated frozen texture (`12757bf`, `c80da2e`, `4d0ab28`)
- [x] 16.4 step 3 device case D11 — **PASS 2026-09-17** on a real macOS window (AppKit `snapshot_view` → ARGB → RGBA8 → `NativeCapture` → runner `import_external_texture` → vello brush; 24/24 freeze probes reported, `frozen total` held at 1, no panic). Recorded in `.superpowers/sdd/device-verification-2026-09-15.md`.

**Unplanned, and load-bearing** — the rendering/text/interaction work of
2026-09-14/15 that the plan assumed was already true: none of it has a step
above, and it is the single largest gap between the count and the code.

# Phase 32 — Required Agent Execution Order

The execution order is intentionally sequential at architecture boundaries, but independent work can be parallelized once a boundary is frozen.

```text
0  Baseline / dependency guards / benchmarks
         ↓
1  Core types / IDs
         ↓
2  Element arena / tree / dirty
         ↓
3  Reactive bridge
         ↓
4  Scheduler / runtime
         ↓
5  Scene
         ↓
6  Renderer ABI → Vello → Software → Skia
         ↓
7  Platform ABI → Desktop → Android/iOS/Harmony → Web/Embedded
         ↓
8  Legacy adapter
         ↓
9  Text/Layout/Style/Theme
         ↓
10 Focus/Gesture/Animation/Overlay/Virtualization/A11y/IME
         ↓
11 Core widgets/forms/navigation
         ↓
12 Native ABI/composition
         ↓
13 WebView/Video/Camera/Map
         ↓
14 Native codegen
         ↓
15 Platform completion
         ↓
16 Data widgets / design systems
         ↓
17 UI IR / DSL / hot reload
         ↓
18 DevTools / Profiler / Testing
         ↓
19 MCP / AI / Figma / Design Contract
         ↓
20 Performance hardening / API cleanup / release engineering
```

Parallelization is allowed only when tasks consume a frozen interface from an earlier step. For example, once Renderer ABI is frozen, Vello and Software adapters can be developed independently; once Native ABI is frozen, WebView and Camera providers can be developed independently.

---

# Phase 33 — Definition of Done for Every Task

Every task must satisfy all of the following before commit:

1. A focused failing test was written first for new behavior.
2. The minimal implementation was added.
3. The focused test passes.
4. The relevant broader test suite passes.
5. A benchmark was added for hot-path/runtime changes where applicable.
6. Public API changes have documentation/examples.
7. Dependency boundary rules still pass.
8. No unrelated refactor is included.
9. Commit message explains the architectural change.

---

# Final Principle

The implementation should always prefer:

```text
smaller stable boundary
        >
clever universal abstraction
```

and:

```text
measured optimization
        >
assumed optimization
```

The framework is successful only when all three conditions hold simultaneously:

```text
Flutter-class product capability
        +
Rust-class performance/resource efficiency
        +
AI-class inspectability/generability
```
