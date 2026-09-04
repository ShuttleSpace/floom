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
- Create: `crates/floem-types/Cargo.toml`
- Create: `crates/floem-types/src/lib.rs`
- Modify: workspace `Cargo.toml`
- Test: `crates/floem-types/src/lib.rs`

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
- Modify: `crates/floem-types/src/lib.rs`
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
- Create: `crates/floem-core/Cargo.toml`
- Create: `crates/floem-core/src/widget.rs`
- Create: `crates/floem-core/src/element.rs`
- Create: `crates/floem-core/src/context.rs`
- Test: `crates/floem-core/tests/contracts.rs`

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

- [ ] **Step 1: Write selective-subscription tests.**
- [ ] **Step 2: Implement selectors on the existing reactive graph.**
- [ ] **Step 3: Verify unrelated selectors are not invalidated.**
- [ ] **Step 4: Benchmark selector overhead.**
- [ ] **Step 5: Commit.**

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

- [ ] **Step 1: Write lifecycle/cancellation tests.**
- [ ] **Step 2: Implement executor abstraction.**
- [ ] **Step 3: Bind task lifetime to Scope.**
- [ ] **Step 4: Add WASM/no-runtime executor test adapter.**
- [ ] **Step 5: Commit.**

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

- [ ] **Step 1: Write ordering tests.**
- [ ] **Step 2: Implement queues and coalescing.**
- [ ] **Step 3: Ensure idle tasks yield to frame-critical work.**
- [ ] **Step 4: Add frame-budget tests.**
- [ ] **Step 5: Commit.**

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

- [ ] **Step 1: Write a Counter test.**
- [ ] **Step 2: Implement mount/event/tick plumbing.**
- [ ] **Step 3: Ensure a local state update does not rebuild unrelated elements.**
- [ ] **Step 4: Add frame instrumentation hooks.**
- [ ] **Step 5: Commit.**

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

- [ ] **Step 1: Write stable-ID and dirty-update tests.**
- [ ] **Step 2: Implement retained Scene nodes.**
- [ ] **Step 3: Add damage tracking.**
- [ ] **Step 4: Confirm the crate has no OS/windowing dependency.**
- [ ] **Step 5: Commit.**

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

- [ ] **Step 1: Write shared-resource tests.**
- [ ] **Step 2: Implement ref-count/usage tracking.**
- [ ] **Step 3: Add budget/eviction hooks.**
- [ ] **Step 4: Commit.**

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

- [ ] **Step 1: Write compile-only backend contract tests.**
- [ ] **Step 2: Implement renderer-neutral types.**
- [ ] **Step 3: Prove a fake renderer can consume a Scene without widgets.**
- [ ] **Step 4: Commit.**

## Task 6.2: Adapt existing Vello renderer

**Files:**
- Modify: existing Vello renderer crate
- Create: `renderer/floem-renderer-vello/src/adapter.rs`
- Test: existing renderer tests + new Scene adapter tests

**Interfaces:**
- Consumes: `Scene`, `RenderTarget`, `DamageRegion`.
- Produces: Vello/WGPU frame submission.

- [ ] **Step 1: Wrap current Vello drawing path behind the new ABI.**
- [ ] **Step 2: Add rect/image/text fast-path tests.**
- [ ] **Step 3: Add resource cache instrumentation.**
- [ ] **Step 4: Benchmark full-frame and partial-damage paths.**
- [ ] **Step 5: Commit.**

## Task 6.3: Adapt Skia and software backends

**Files:**
- Modify: existing Skia renderer crate
- Modify: existing tiny/software renderer crate
- Test: backend compatibility matrix

- [ ] **Step 1: Implement renderer ABI adapters.**
- [ ] **Step 2: Run identical Scene snapshots through both backends.**
- [ ] **Step 3: Add software-headless snapshot tests.**
- [ ] **Step 4: Commit.**

## Task 6.4: Implement render-thread Scene handoff

**Files:**
- Modify: `runtime/floem-runtime/src/frame.rs`
- Create: `renderer/floem-renderer/src/frame_queue.rs`
- Test: `runtime/floem-runtime/tests/render_handoff.rs`

- [ ] **Step 1: Write a test proving render work can consume an immutable Scene snapshot.**
- [ ] **Step 2: Implement single-producer/single-consumer or bounded queue appropriate to the target platform.**
- [ ] **Step 3: Instrument queue wait and frame latency.**
- [ ] **Step 4: Commit.**

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

- [ ] **Step 1: Write a fake platform test harness.**
- [ ] **Step 2: Implement window/surface/event/lifecycle/display contracts.**
- [ ] **Step 3: Verify no concrete platform crate leaks into core.**
- [ ] **Step 4: Commit.**

## Task 7.2: Migrate desktop backend

**Files:**
- Create/modify: `platform/floem-platform-desktop`
- Modify: existing Winit integration where present
- Test: desktop window/event tests

- [ ] **Step 1: Map existing window lifecycle to the new ABI.**
- [ ] **Step 2: Map pointer/keyboard/display events.**
- [ ] **Step 3: Add clipboard and IME service adapters.**
- [ ] **Step 4: Run macOS/Windows/Linux smoke tests.**
- [ ] **Step 5: Commit.**

## Task 7.3: Create independent Android, iOS, and Harmony platform crates

**Files:**
- Create: `platform/floem-platform-android`
- Create: `platform/floem-platform-ios`
- Create: `platform/floem-platform-harmony`
- Test: per-platform lifecycle/input/display contract tests

- [ ] **Step 1: Implement lifecycle surface contracts.**
- [ ] **Step 2: Implement input/display/capability reporting.**
- [ ] **Step 3: Implement IME/focus bridges.**
- [ ] **Step 4: Implement accessibility bridge stubs with explicit unsupported capability states.**
- [ ] **Step 5: Commit each platform separately.**

## Task 7.4: Implement Web platform

**Files:**
- Create: `platform/floem-platform-web`
- Test: WASM/browser smoke tests

- [ ] **Step 1: Implement event/frame clock/display abstraction.**
- [ ] **Step 2: Implement WebGPU target detection and fallback capability reporting.**
- [ ] **Step 3: Implement DOM/semantic bridge hooks.**
- [ ] **Step 4: Commit.**

## Task 7.5: Implement Embedded platform baseline

**Files:**
- Create: `platform/floem-platform-embedded`
- Test: headless DRM/framebuffer simulation

- [ ] **Step 1: Define minimal surface/input/frame-clock contract.**
- [ ] **Step 2: Implement a Linux DRM/KMS or framebuffer-compatible adapter behind feature gates.**
- [ ] **Step 3: Validate with software renderer first.**
- [ ] **Step 4: Commit.**

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

- [ ] **Step 1: Write Unicode, bidi, fallback, and line-breaking tests.**
- [ ] **Step 2: Wrap the current text engine behind a neutral API.**
- [ ] **Step 3: Emit renderer-neutral glyph runs.**
- [ ] **Step 4: Benchmark shaping/cache behavior.**
- [ ] **Step 5: Commit.**

## Task 9.2: Build shared editing model

**Files:**
- Create: `text/floem-editor/src/document.rs`
- Create: `text/floem-editor/src/selection.rs`
- Create: `text/floem-editor/src/undo.rs`
- Test: editing semantics

- [ ] **Step 1: Write cursor/selection/undo tests.**
- [ ] **Step 2: Implement document transactions.**
- [ ] **Step 3: Add IME composition hooks.**
- [ ] **Step 4: Commit.**

---

# Phase 10 — Style, Layout, Theme

## Task 10.1: Build backend-neutral layout API

**Files:**
- Create: `layout/floem-layout/src/lib.rs`
- Create: `layout/floem-layout/src/taffy_adapter.rs`
- Test: flex/grid/stack/measure tests

- [ ] **Step 1: Write layout contract tests.**
- [ ] **Step 2: Implement Taffy adapter.**
- [ ] **Step 3: Ensure Taffy node IDs do not leak to public API.**
- [ ] **Step 4: Benchmark incremental subtree layout.**
- [ ] **Step 5: Commit.**

## Task 10.2: Implement Style and selector state

**Files:**
- Create: `layout/floem-style/src/lib.rs`
- Test: state selector/inheritance/token tests

- [ ] **Step 1: Write tests for normal/hover/pressed/focused/disabled/selected.**
- [ ] **Step 2: Implement style property storage and precedence.**
- [ ] **Step 3: Implement inherited typography properties only.**
- [ ] **Step 4: Benchmark style invalidation.**
- [ ] **Step 5: Commit.**

## Task 10.3: Implement Theme and tokens

**Files:**
- Create: `framework/floem-theme/src/lib.rs`
- Test: light/dark/dynamic token tests

- [ ] **Step 1: Define color/typography/spacing/radius/elevation/motion token structs.**
- [ ] **Step 2: Implement theme propagation.**
- [ ] **Step 3: Add platform-adaptive token hooks.**
- [ ] **Step 4: Commit.**

---

# Phase 11 — Interaction, Focus, Gesture, Animation, Overlay

## Task 11.1: Implement event routing and hit testing

**Files:**
- Create: `runtime/floem-runtime/src/event.rs`
- Test: bubbling/capture/hit-test tests

- [ ] **Step 1: Write event target selection tests.**
- [ ] **Step 2: Implement pointer routing and pointer capture.**
- [ ] **Step 3: Integrate native graph hit regions.**
- [ ] **Step 4: Commit.**

## Task 11.2: Implement FocusManager

**Files:**
- Create: `framework/floem-focus/src/lib.rs`
- Test: focus traversal and scope tests

- [ ] **Step 1: Write keyboard/tab/shift-tab tests.**
- [ ] **Step 2: Implement focus scopes and traversal policy.**
- [ ] **Step 3: Integrate platform focus.**
- [ ] **Step 4: Commit.**

## Task 11.3: Implement Gesture Arena

**Files:**
- Create: `framework/floem-gesture/src/lib.rs`
- Test: tap/drag/scroll/scale conflict tests

- [ ] **Step 1: Write recognizer competition tests.**
- [ ] **Step 2: Implement arena ownership/cancellation.**
- [ ] **Step 3: Integrate Scroll and Native View interaction.**
- [ ] **Step 4: Commit.**

## Task 11.4: Implement animation scheduler

**Files:**
- Create: `framework/floem-animation/src/lib.rs`
- Test: tween/spring/keyframe tests

- [ ] **Step 1: Write deterministic animation tests with a fake clock.**
- [ ] **Step 2: Implement platform-frame-driven ticking.**
- [ ] **Step 3: Add compositor-only transform/opacity path.**
- [ ] **Step 4: Benchmark 60/120Hz scenarios.**
- [ ] **Step 5: Commit.**

## Task 11.5: Implement Overlay system

**Files:**
- Create: `framework/floem-overlay/src/lib.rs`
- Test: modal/focus/z-order tests

- [ ] **Step 1: Write dialog/popup/bottom-sheet tests.**
- [ ] **Step 2: Implement overlay tree and hit-testing priority.**
- [ ] **Step 3: Integrate animation and focus.**
- [ ] **Step 4: Commit.**

---

# Phase 12 — Virtualization

## Task 12.1: Implement VirtualViewport

**Files:**
- Create: `layout/floem-virtual/src/lib.rs`
- Test: 1k/10k/100k item range tests

- [ ] **Step 1: Write viewport/range/key reuse tests.**
- [ ] **Step 2: Implement visible range calculation.**
- [ ] **Step 3: Implement stable-key element reuse.**
- [ ] **Step 4: Add overscan controls.**
- [ ] **Step 5: Benchmark CPU/RAM/scrolling.**
- [ ] **Step 6: Commit.**

## Task 12.2: Native view pooling

**Files:**
- Create: `native/floem-native/src/pool.rs`
- Test: reuse/lifecycle tests

- [ ] **Step 1: Write native slot reuse tests.**
- [ ] **Step 2: Implement pool and attach/detach lifecycle.**
- [ ] **Step 3: Benchmark 10k virtualized rows with native placeholders.**
- [ ] **Step 4: Commit.**

---

# Phase 13 — Accessibility and IME

## Task 13.1: Semantics Tree

**Files:**
- Create: `framework/floem-accessibility/src/lib.rs`
- Test: semantics tree tests

- [ ] **Step 1: Define semantics node schema.**
- [ ] **Step 2: Write role/label/value/action tests.**
- [ ] **Step 3: Connect Element dirty semantics updates.**
- [ ] **Step 4: Commit.**

## Task 13.2: Platform accessibility adapters

**Files:**
- Modify: each platform backend with accessibility adapter modules
- Test: at least one desktop and one mobile target in CI/device testing

- [ ] **Step 1: Implement desktop adapter.**
- [ ] **Step 2: Implement Android/iOS adapters.**
- [ ] **Step 3: Add Harmony/Web adapters or explicit capability-limited implementations.**
- [ ] **Step 4: Test native subtree integration.**
- [ ] **Step 5: Commit per platform.**

## Task 13.3: IME abstraction

**Files:**
- Create: `text/floem-ime/src/lib.rs`
- Modify: platform backends
- Test: composition/selection/candidate tests

- [ ] **Step 1: Write fake-IME tests.**
- [ ] **Step 2: Implement platform-neutral IME state.**
- [ ] **Step 3: Bridge native text inputs and WebView.**
- [ ] **Step 4: Commit.**

---

# Phase 14 — Core Widgets

## Task 14.1: Primitive widgets

**Files:**
- Create: `widgets/floem-widgets/src/primitives.rs`
- Test: primitive snapshots

- [ ] **Step 1: Implement Text, Icon, Image, Container, Spacer, Divider.**
- [ ] **Step 2: Add scene snapshots.**
- [ ] **Step 3: Add semantics snapshots.**
- [ ] **Step 4: Commit.**

## Task 14.2: Interactive controls

**Files:**
- Create: `widgets/floem-widgets/src/controls.rs`
- Test: interaction/snapshot tests

- [ ] **Step 1: Implement Button.**
- [ ] **Step 2: Implement TextField.**
- [ ] **Step 3: Implement Checkbox/Radio/Switch.**
- [ ] **Step 4: Implement Slider/Progress/Select.**
- [ ] **Step 5: Add state variants and accessibility.**
- [ ] **Step 6: Commit.**

## Task 14.3: Navigation/layout widgets

**Files:**
- Create: `widgets/floem-widgets/src/navigation.rs`
- Test: layout/navigation tests

- [ ] **Step 1: Implement List/Grid/Stack/Tabs.**
- [ ] **Step 2: Implement NavigationBar/NavigationRail/Drawer.**
- [ ] **Step 3: Implement Dialog/Popup/BottomSheet.**
- [ ] **Step 4: Commit.**

## Task 14.4: Forms

**Files:**
- Create: `framework/floem-forms/src/lib.rs`
- Test: validation/form-state tests

- [ ] **Step 1: Define FormController state.**
- [ ] **Step 2: Implement validators and async validation.**
- [ ] **Step 3: Integrate TextField/Select/Switch.**
- [ ] **Step 4: Commit.**

---

# Phase 15 — Navigation / Persistence / App Services

## Task 15.1: Typed Router/Navigator

**Files:**
- Create: `framework/floem-navigation/src/lib.rs`
- Test: route/deep-link/restoration tests

- [ ] **Step 1: Write typed-route tests.**
- [ ] **Step 2: Implement Router state and Navigator stack.**
- [ ] **Step 3: Add URL/deep-link parser boundary.**
- [ ] **Step 4: Add stack/tab/split containers.**
- [ ] **Step 5: Commit.**

## Task 15.2: Persistence and restoration

**Files:**
- Create: `framework/floem-persistence/src/lib.rs`
- Test: key-value/restoration tests

- [ ] **Step 1: Define backend traits.**
- [ ] **Step 2: Implement in-memory and file-based reference providers.**
- [ ] **Step 3: Add restoration snapshots.**
- [ ] **Step 4: Connect platform storage providers.**
- [ ] **Step 5: Commit.**

## Task 15.3: Service scopes / DI

**Files:**
- Create: `framework/floem-services/src/lib.rs`
- Test: scope lifetime tests

- [ ] **Step 1: Write service registration/resolution tests.**
- [ ] **Step 2: Implement scoped service registry with typed-first APIs.**
- [ ] **Step 3: Verify disposal with Scope lifetime.**
- [ ] **Step 4: Commit.**

## Task 15.4: ErrorBoundary

**Files:**
- Create: `framework/floem-error/src/lib.rs`
- Test: recoverable/fatal boundary tests

- [ ] **Step 1: Write fallback rendering tests.**
- [ ] **Step 2: Implement error capture around recoverable application callbacks.**
- [ ] **Step 3: Preserve diagnostic context in debug builds.**
- [ ] **Step 4: Commit.**

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

- [ ] **Step 1: Define typed command/event/property schema.**
- [ ] **Step 2: Define ABI versioning.**
- [ ] **Step 3: Add schema serialization for tooling only.**
- [ ] **Step 4: Commit.**

## Task 18.2: Generate Swift/Kotlin/C/C++ glue

**Files:**
- Create: `compiler/floem-codegen/backends/*`
- Test: generated-code compile fixtures

- [ ] **Step 1: Implement C ABI generator.**
- [ ] **Step 2: Implement Swift generator.**
- [ ] **Step 3: Implement Kotlin generator.**
- [ ] **Step 4: Implement C++/Harmony bindings as needed.**
- [ ] **Step 5: Add compile fixtures.**
- [ ] **Step 6: Commit.**

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

- [ ] **Step 1: Define headless data models.**
- [ ] **Step 2: Add virtualized rendering.**
- [ ] **Step 3: Add keyboard navigation and selection.**
- [ ] **Step 4: Add editing/undo integration.**
- [ ] **Step 5: Commit.**

## Task 20.2: Charts and media controls

- [ ] Define renderer-neutral chart primitives.
- [ ] Add accessible chart summaries.
- [ ] Add media controls with semantic states.
- [ ] Commit.

---

# Phase 21 — Design Systems

## Task 21.1: Material

**Files:**
- Create: `design-systems/floem-material`
- Test: component gallery snapshots

- [ ] **Step 1: Define Material tokens.**
- [ ] **Step 2: Map widget variants to Material visuals/behavior.**
- [ ] **Step 3: Add light/dark/dynamic color.**
- [ ] **Step 4: Commit.**

## Task 21.2: Cupertino

- [ ] Define platform-adaptive tokens and controls.
- [ ] Add iOS interaction semantics.
- [ ] Add snapshots and device tests.
- [ ] Commit.

## Task 21.3: Fluent

- [ ] Define Fluent tokens and desktop interaction behavior.
- [ ] Add snapshots.
- [ ] Commit.

## Task 21.4: Harmony

- [ ] Define Harmony-specific tokens/components where licensing and public platform guidance allow.
- [ ] Add device snapshots.
- [ ] Commit.

---

# Phase 22 — UI IR and DSL

## Task 22.1: Define versioned UI IR

**Files:**
- Create: `compiler/floem-ui-ir`
- Test: schema round-trip fixtures

- [ ] **Step 1: Define node/property/slot/event schema.**
- [ ] **Step 2: Add versioning and compatibility rules.**
- [ ] **Step 3: Add Rust serializer/deserializer used only by tooling.**
- [ ] **Step 4: Commit.**

## Task 22.2: Implement DSL parser/compiler

**Files:**
- Create: `compiler/floem-dsl`
- Test: parser/compiler fixtures

- [ ] **Step 1: Define syntax for structure, style, events, variants, and bindings.**
- [ ] **Step 2: Parse into AST.**
- [ ] **Step 3: Lower AST to UI IR.**
- [ ] **Step 4: Add optimized builder/codegen output.**
- [ ] **Step 5: Commit.**

## Task 22.3: Hot reload

**Files:**
- Create: `compiler/floem-hot-reload`
- Test: live style/layout tree patching

- [ ] **Step 1: Add runtime UI patch protocol.**
- [ ] **Step 2: Preserve Element IDs/state for compatible patches.**
- [ ] **Step 3: Add reload failure isolation and rollback.**
- [ ] **Step 4: Commit.**

---

# Phase 23 — DevTools and Profiler

## Task 23.1: Inspection protocol

**Files:**
- Create: `tooling/floem-inspector/src/protocol.rs`
- Test: snapshot schemas

- [ ] **Step 1: Define versioned tree/style/layout/scene/performance snapshot types.**
- [ ] **Step 2: Add stable element identifiers and source locations.**
- [ ] **Step 3: Commit.**

## Task 23.2: Inspector UI

**Files:**
- Create: `tooling/floem-devtools`
- Test: inspector navigation tests

- [ ] **Step 1: Implement element tree.**
- [ ] **Step 2: Implement property/style/layout inspector.**
- [ ] **Step 3: Add source mapping.**
- [ ] **Step 4: Add semantics tree viewer.**
- [ ] **Step 5: Commit.**

## Task 23.3: Profiler

**Files:**
- Create: `tooling/floem-profiler`
- Test: trace format tests

- [ ] **Step 1: Instrument input/reactive/layout/paint/scene/render/native stages.**
- [ ] **Step 2: Implement frame timeline output.**
- [ ] **Step 3: Add allocation counters.**
- [ ] **Step 4: Add memory-pressure/cache statistics.**
- [ ] **Step 5: Commit.**

## Task 23.4: “Why did this update?” diagnostics

- [ ] Record reactive dependency/source metadata in dev builds.
- [ ] Trace element invalidation to signal/property.
- [ ] Show update path in Inspector.
- [ ] Add regression tests.
- [ ] Commit.

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

- [ ] **Step 1: Write interaction tests using semantic/test-id selectors.**
- [ ] **Step 2: Implement headless runtime harness.**
- [ ] **Step 3: Commit.**

## Task 24.2: Snapshot engine

**Files:**
- Create: `tooling/floem-snapshot`
- Test: PNG/scene/semantics snapshots

- [ ] **Step 1: Implement software-renderer snapshots.**
- [ ] **Step 2: Implement Scene snapshot.**
- [ ] **Step 3: Implement Semantics snapshot.**
- [ ] **Step 4: Add deterministic font/resource test fixtures.**
- [ ] **Step 5: Commit.**

---

# Phase 25 — MCP / AI Agent Layer

## Task 25.1: Agent protocol

**Files:**
- Create: `tooling/floem-agent`
- Test: protocol round-trip tests

- [ ] **Step 1: Define inspection/control commands.**
- [ ] **Step 2: Version protocol payloads.**
- [ ] **Step 3: Implement local transport.**
- [ ] **Step 4: Add authentication/localhost-only guard for development.**
- [ ] **Step 5: Commit.**

## Task 25.2: AI-friendly component schema

- [ ] Emit widget/property/event/slot metadata.
- [ ] Emit examples and allowed enum values.
- [ ] Emit design token relationships.
- [ ] Add schema tests.
- [ ] Commit.

## Task 25.3: Running-app agent loop

- [ ] Implement screenshot + inspection + interaction sequence.
- [ ] Expose performance and accessibility information.
- [ ] Add end-to-end agent fixture app.
- [ ] Commit.

---

# Phase 26 — Figma / Design Contract

## Task 26.1: Design contract schema

**Files:**
- Create: `compiler/floem-ui-ir/src/design_contract.rs`
- Test: expected/actual comparison tests

- [ ] **Step 1: Define dimensions/spacing/typography/colors/radius/state/animation/accessibility fields.**
- [ ] **Step 2: Implement deterministic comparison output.**
- [ ] **Step 3: Commit.**

## Task 26.2: Figma importer boundary

- [ ] Define an importer API accepting normalized design data.
- [ ] Map auto-layout → Flex, spacing → gap, padding → padding, variants → semantic variants, text styles → typography tokens.
- [ ] Emit UI IR and Design Contract.
- [ ] Add fixture-based tests.
- [ ] Commit.

---

# Phase 27 — AI/Design Accuracy Loop

## Task 27.1: Build reference gallery

- [ ] Create representative screens: settings, dashboard, feed, editor, media list, form, dialog, navigation, webview screen.
- [ ] Store reference design contracts.
- [ ] Commit fixtures.

## Task 27.2: Automated render-and-compare loop

- [ ] Render reference screen with headless renderer.
- [ ] Compare geometry/style/semantics/optional pixels.
- [ ] Emit machine-readable diff.
- [ ] Commit evaluation harness.

## Task 27.3: Agent repair benchmark

- [ ] Give an agent a design contract and imperfect implementation.
- [ ] Allow MCP inspection and edits.
- [ ] Measure convergence and regressions.
- [ ] Establish acceptance thresholds.
- [ ] Commit benchmark harness.

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

- [ ] Core types stable.
- [ ] ElementArena stable.
- [ ] Dirty propagation verified.
- [ ] Reactive bridge verified.
- [ ] Scheduler verified.
- [ ] Scene ABI verified.
- [ ] Renderer ABI verified by Vello + software.
- [ ] Platform ABI verified by fake backend + desktop backend.
- [x] Legacy adapter — **WAIVED (Policy A):** no legacy adapter; demos must target the new runtime before P0 claim.
- [ ] Baseline benchmarks and dependency checks pass.

## P1 Acceptance

- [ ] Layout/Text/Style/Theme stable.
- [ ] Focus/Gesture/Animation/Overlay stable.
- [ ] Accessibility/IME functional on representative desktop/mobile targets.
- [ ] Core widgets and forms complete.
- [ ] Typed Navigation and VirtualViewport complete.
- [ ] Snapshot/interaction tests cover core widgets.

## P2 Acceptance

- [ ] macOS/Windows/Linux/iOS/Android/Harmony/Web/Embedded backend matrix passes defined conformance suite.
- [ ] WebView/Video/Camera/Map native providers work on agreed target subsets.
- [ ] Native composition policy works without widget-level platform conditionals.

## P3 Acceptance

- [ ] Inspector can trace Element → Scene → source.
- [ ] Profiler reports frame budget and allocations.
- [ ] MCP agent can inspect/click/type/scroll/screenshot.
- [ ] UI IR round-trips reference fixtures.
- [ ] Hot reload handles compatible UI patches.
- [ ] Figma/design contract diff works.

## P4 Acceptance

- [ ] Material/Cupertino/Fluent/Harmony design systems available.
- [ ] Docs/examples complete.
- [ ] Third-party plugin authoring path documented.
- [ ] Compatibility/release policy active.
- [ ] Performance budgets enforced in CI.

---

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
