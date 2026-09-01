# Floem 2.0 Architecture Specification

## Status

Draft for review. This document consolidates the architecture decisions discussed for Floem 2.0.

## Goal

Evolve Floem from a maturing native Rust GUI library into a lightweight, high-performance, multi-platform application UI framework supporting macOS, Windows, Linux, iOS, Android, HarmonyOS/OpenHarmony, Web/WASM, and Embedded Linux/ARM, while preserving Floem's fine-grained reactive strengths.

## Non-goals

- Do not copy Flutter's implementation or object model literally.
- Do not force one renderer or one async runtime on every platform.
- Do not make native platform views part of the ordinary scene graph.
- Do not require the UI DSL for Rust applications.
- Do not make the UI IR the production runtime representation.
- Do not rewrite all existing Floem widgets before the new runtime has compatibility coverage and benchmarks.

## Architecture Principles

1. Rust-first, not Rust-only.
2. Fine-grained reactivity plus a retained Element tree.
3. Avoid Flutter-style three full runtime trees; use one persistent Element tree plus derived/cached layout, semantics, and scene data.
4. State change does not imply full-tree rebuild.
5. Dirty tracking determines the smallest necessary work: state, style, layout, paint, hit-test, semantics, children, transform.
6. Renderer is driven by a platform-independent retained Scene and a thin Renderer ABI.
7. Platform and Renderer are orthogonal axes.
8. Native Views are external compositor participants with Overlay, Texture, Surface, and Offscreen modes.
9. Platform capabilities are feature-oriented; platform-specific implementations are not hidden behind fake universal OS abstractions.
10. Release runtime must be minimal; DevTools, profiling, MCP, and hot reload are opt-in development capabilities.
11. Public APIs must remain composable and readable by humans and AI agents.
12. Performance budgets and dependency-boundary checks are architectural tests, not after-the-fact optimizations.

## Platform Model

### Platform Core

The core Platform ABI is intentionally small. It covers initialization, event dispatch, window creation, surface creation, frame requests, clock access, and capability reporting.

Conceptual API:

```rust
trait Platform {
    type Window;
    type Surface;
    type Event;

    fn init(&mut self, config: PlatformConfig) -> Result<()>;
    fn run(&mut self, app: &mut dyn RuntimeApp);
    fn create_window(&mut self, config: WindowConfig) -> Result<Self::Window>;
    fn create_surface(&mut self, window: &Self::Window) -> Result<Self::Surface>;
    fn poll_events(&mut self) -> Vec<Self::Event>;
    fn request_frame(&mut self);
    fn now(&self) -> Instant;
    fn capabilities(&self) -> PlatformCapabilities;
}
```

Platform implementations:

- macOS
- Windows
- Linux
- iOS
- Android
- HarmonyOS/OpenHarmony
- Web/WASM
- Embedded

Desktop may share implementation infrastructure, but HarmonyOS is an independent platform backend rather than an Android compatibility layer.

### Platform Services

Window/surface lifecycle is separate from optional services. Services include IME, clipboard, file picker, notifications, camera, media, WebView, sensors, and other platform capabilities.

Services are optional and capability-driven.

## Runtime Model

### One Persistent Element Tree

The production runtime uses:

```text
View/Widget description
        -> Element
             |- reactive/state
             |- layout state
             |- interaction state
             |- paint state
             `- semantics state
        -> Scene
        -> Renderer
```

The runtime does not maintain a second full RenderObject tree equivalent to Flutter's. Layout, semantics, and scene data are stored as compact state/caches indexed by stable element identifiers.

### Element Identity

Introduce a stable `ElementId`. Existing `ViewId` is adapted during migration so existing code can continue to run while new APIs use Element terminology.

The runtime should use compact indexed storage with generational identity to avoid pointer-heavy object graphs in hot paths.

### Reactive Graph

The existing Floem reactive model remains the foundation. Signal, Memo, Effect, and Scope are retained and extended with Resource, Task, Store, Selector, Transaction, and batching capabilities.

Reactive graph and UI tree remain separate:

```text
Reactive graph -> invalidation -> Element / property
```

### Dirty Flags

The runtime tracks:

```text
STATE
STYLE
LAYOUT
PAINT
HIT_TEST
SEMANTICS
CHILDREN
TRANSFORM
```

A change should propagate only as far as required. Examples:

- text content: PAINT
- font size: LAYOUT + PAINT
- width: affected LAYOUT + PAINT
- accessibility label: SEMANTICS
- transform animation: TRANSFORM/compositor path where possible

### Scheduler

Frame scheduling is priority-oriented:

```text
Input -> State -> Animation -> Style/Layout -> Paint/Scene -> Semantics -> Idle
```

Long-running I/O, decoding, and blocking computation must not execute on the UI thread.

## Scene / Renderer Model

### Scene

`floem-scene` defines a platform-independent retained scene graph. It contains geometry, text runs, images, transforms, clip, opacity, shadows/effects, and native composition nodes without depending on OS or a concrete graphics library.

Scene nodes have stable IDs and dirty state.

### Renderer ABI

The Renderer consumes Scene + RenderTarget + DamageRegion. It does not know about widgets, reactive state, or platform widgets.

Conceptual API:

```rust
trait Renderer {
    type Frame;
    type Error;

    fn begin_frame(&mut self, target: &RenderTarget, info: &FrameInfo) -> Result<Self::Frame, Self::Error>;
    fn render(&mut self, frame: &mut Self::Frame, scene: &Scene, damage: &DamageRegion) -> Result<(), Self::Error>;
    fn present(&mut self, frame: Self::Frame) -> Result<(), Self::Error>;
}
```

Renderers:

- Vello/WGPU: primary high-quality GPU vector path where supported.
- Skia: compatibility and complex-graphics backend.
- Software/tiny-skia: embedded, CI, headless and fallback path.
- WGPU primitives may exist as renderer infrastructure but should not leak into widget APIs.

### Resource Model

Scene stores stable resource references. A ResourceRegistry manages decoded data, font/glyph resources, images, textures, shaders, and other external resources.

Cache classes:

- geometry cache: aggressive
- resource cache: aggressive with memory budgets
- raster cache: conservative and cost-based

### Damage

DamageRegion is an optimization boundary. Full-frame rendering remains a valid renderer strategy when partial damage is unsuitable.

## Native Composition Model

Native components are not ordinary scene nodes. They are managed by a Native Graph and composed with the scene through platform compositor facilities.

Four modes:

1. Overlay: native view placed above/below the Floem surface.
2. Texture: native content exposed as a GPU texture.
3. Surface: native surface participates in OS/GPU compositor composition.
4. Offscreen: native content renders offscreen and is consumed as a texture/snapshot.

Runtime chooses a mode based on platform, capabilities, clipping/transforms, animation, accessibility, and content type.

Native component lifecycle supports Attached, Detached, Suspended, and Destroyed states so virtualization and route transitions can reuse expensive views.

High-frequency native operations must avoid per-frame cross-language calls. Transform and compositor animations should remain on the render/composition path whenever possible.

## Widget / Layout / Style

Widgets are semantic UI descriptions. Elements own runtime state. Widget categories:

- primitive
- composite
- native

The style system supports layout, paint, typography, interaction states, transitions, tokens, and restricted inheritance.

Themes are independent from widgets and include Material, Cupertino, Fluent, Harmony, and custom themes.

Layout is exposed through Floem abstractions with Taffy initially acting as an adapter. Taffy-specific node IDs do not leak into public framework APIs.

Virtualization is a first-class runtime facility for large lists, grids, trees, and data tables.

## Animation / Gesture / Text / Accessibility

Animation is a runtime service with Tween, Curve, Spring, Keyframe, Timeline, Transition, SharedElement, and physics concepts.

Compositor-friendly animation properties such as transform and opacity should not require ordinary reactive graph invalidation each frame unless layout actually depends on them.

Gesture processing uses pointer abstraction plus a gesture arena. Pointer inputs include touch, mouse, pen, trackpad, and gamepad where applicable.

Text is an independent subsystem covering Unicode, shaping, bidi, line breaking, fonts, glyph caches, selection, editing, cursor, emoji, RTL, and IME.

Accessibility is represented by a semantics tree and bridged to each platform. Native views can contribute accessibility subtrees.

## Application Framework

Provide small primitives rather than impose one architecture:

- local/shared/resource/persistent state
- Store and selector
- Task/resource
- Scope-bound dependency injection
- typed navigation and route state
- persistence/cache
- error boundaries
- undo/redo
- form model and validation

Navigation is state-driven and should support stack, tabs, split view, drawer, modal, deep links, and state restoration.

## Tooling / AI

UI IR is an interchange representation for Figma import/export, design tooling, DSL, AI, tests, and DevTools. It is not the production runtime tree.

DevTools should provide:

- element tree
- properties and styles
- layout bounds
- reactive dependencies
- scene inspection
- frame timeline
- CPU/GPU timing
- allocations/memory
- accessibility tree

An optional local agent/MCP interface should expose inspection and controlled interaction only in development builds.

## Compatibility and Migration

Migration must be incremental.

1. Build the new runtime and scene/platform boundaries beside existing Floem.
2. Adapt existing `View`/`ViewId` to `Widget`/`ElementId` through compatibility adapters.
3. Keep existing renderer implementations operational while introducing the new Renderer ABI.
4. Migrate core widgets one family at a time.
5. Remove legacy APIs only after coverage, benchmarks, and examples are equivalent or better.

The first milestone does not require the complete widget library, mobile platforms, HarmonyOS, or DSL.

## Performance Budgets

Initial engineering targets are directional rather than device-independent guarantees:

- no whole-tree rebuild for a local signal update
- zero or near-zero allocations in hot update/render paths for common widgets
- compositor-only transform/opacity animation where possible
- virtualized 100k-item collections
- bounded GPU resource caches
- frame-time profiling at 60/120 Hz
- memory and allocation regression tracking in CI
- minimal release dependencies through Cargo features

Concrete budgets must be measured on declared reference devices before turning them into hard release gates.

## P0 Deliverables

P0 establishes only the new foundations:

1. types/core IDs
2. runtime Element arena
3. reactive-runtime bridge
4. dirty tracking
5. scheduler
6. scene graph
7. renderer ABI
8. platform ABI
9. legacy View compatibility adapter
10. benchmark and dependency-boundary harness

No broad widget rewrite is required in P0.

## Reference Baseline

As of the current upstream `main`, Floem uses a workspace containing renderer, skia, vello, vger, tiny_skia, reactive, editor-core, examples, ui-events-winit, and test crates. The root package currently owns many framework dependencies and exposes renderer/serde/editor/image/runtime features. The current public `View` trait spans update/style/layout/event/paint behavior, while `ViewId` is a stable handle used for runtime operations. This specification intentionally moves toward clearer boundaries without discarding those existing implementations.
