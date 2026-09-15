# Floem 2.0 Complete Architecture Specification

## Status

Proposed architecture for a long-lived, industrial-grade, Rust-first, cross-platform GUI framework.

## Goal

Evolve Floem from a capable Rust GUI toolkit into a lightweight, high-performance, multi-platform application UI framework with first-class support for macOS, Windows, Linux, iOS, Android, HarmonyOS/OpenHarmony, Web/WASM, and Embedded Linux/ARM, while preserving Floem's strongest existing property: fine-grained reactivity and low-cost local updates.

## Product Positioning

Floem 2.0 is a **Rust-native, reactive-retained, GPU-accelerated, multi-platform, AI-friendly application UI framework**.

The intended product combination is:

- Flutter-level product completeness and widget coverage.
- Slint-level DSL, compiler, tooling, preview, and AOT mindset.
- Xilem/Masonry-level separation between high-level UI description and low-level runtime infrastructure.
- GPUI-level low-level control and large-editor suitability.
- egui-level low-friction onboarding and developer ergonomics.
- Makepad-level GPU-first/custom-rendering capability.
- Dioxus-level template/IR/tooling friendliness.
- SOUI-style industrial component/theme discipline.

Floem must not become a Rust clone of any one of these systems.

---

# 1. Non-Goals

Floem 2.0 is not required to:

1. Reproduce another framework's public API exactly.
2. Make every subsystem mandatory for every target.
3. Implement every OS service in Rust.
4. Force one async runtime, database, networking stack, or state-management pattern on application developers.
5. Guarantee identical rendering pixels across all platforms where the operating system or native compositor necessarily differs.
6. Treat UI IR or JSON as the production runtime representation.

---

# 2. Supported First-Class Platforms

The target matrix is:

| Platform | Tier | Notes |
|---|---|---|
| macOS | P0/P1 | Desktop, high-DPI, Metal/WGPU |
| Windows | P0/P1 | Desktop, DirectX/WGPU, WebView2 |
| Linux | P0/P1 | X11/Wayland; desktop and kiosk |
| iOS | P1/P2 | UIKit lifecycle, IME, accessibility |
| Android | P1/P2 | Surface, IME, accessibility, native views |
| HarmonyOS/OpenHarmony | P1/P2 | Independent platform backend; no Android emulation layer |
| Web/WASM | P2 | WebGPU first, DOM/Canvas hybrid capability |
| Embedded Linux/ARM | P2 | DRM/KMS/framebuffer, touch, low-memory mode |

Platform-specific APIs may differ internally. The shared contract is capability-oriented, not OS-name-oriented.

---

# 3. Architectural Principles

## 3.1 State change is not tree rebuild

A state mutation must invalidate the smallest possible reactive/runtime scope.

## 3.2 Retained runtime, fine-grained reactivity

Use one persistent Element Tree and a separate reactive graph. Do not rebuild an entire UI tree for local updates.

## 3.3 Scene is renderer-independent

Widgets never depend on Vello, Skia, WGPU, Metal, DirectX, or software rendering details.

## 3.4 Platform is renderer-independent

Window/surface/input/lifecycle/IME/accessibility are platform services; renderers consume render targets and scenes.

## 3.5 Native views are external compositor participants

WebView, video, camera, map, UIKit/Android/Harmony native controls are represented by a Native Graph rather than ordinary scene nodes.

## 3.6 Zero-cost where practical

Avoid unnecessary allocations, locks, dynamic dispatch, serialization, resource duplication, or FFI calls in hot paths.

## 3.7 High-level and low-level APIs coexist

New developers use semantic widgets; framework authors can reach Element/Layout/Paint/Native primitives.

## 3.8 Capability-based platform APIs

Applications query capabilities instead of spreading `target_os` conditionals throughout UI code.

## 3.9 Tooling is part of the framework

Inspector, profiler, snapshot testing, hot reload, UI IR, MCP, and agent integration are first-class development features but excluded from production hot paths unless explicitly enabled.

## 3.10 Stable boundaries over maximal abstraction

Prefer small stable interfaces over generic mega-traits.

---

# 4. Reference Architecture

```text
Application / Feature Modules
        │
        ├── Rust API
        ├── Optional Floem DSL
        └── UI IR / AI
        │
        ▼
Application Framework
├── Navigation
├── State / Store / Resource / Task
├── Persistence
├── Services / DI
└── Error Boundaries
        │
        ▼
Widget & Design System
├── Primitives
├── Controls
├── Layout
├── Navigation
├── Data
├── Media
└── Material / Cupertino / Fluent / Harmony
        │
        ▼
Runtime
├── Element Arena
├── Reactive Graph Bridge
├── Scheduler
├── Dirty Tracking
├── Event Routing
├── Focus
├── Gesture Arena
├── Animation
├── Overlay
└── Resource Management
        │
        ├──────────────┬───────────────────┐
        ▼              ▼                   ▼
Layout/Text      Scene Graph          Native Graph
        │              │                   │
        └──────────────┴───────────────────┘
                       │
          ┌────────────┴────────────┐
          ▼                         ▼
    Renderer ABI              Native ABI
          │                         │
   ┌──────┼──────┐          ┌───────┼─────────┐
   ▼      ▼      ▼          ▼       ▼         ▼
 Vello   Skia   SW       WebView  Video   Camera/Map
          │                         │
          └────────────┬────────────┘
                       ▼
                 Platform ABI
                       │
   macOS / Windows / Linux / iOS / Android / Harmony
                / Web / Embedded
```

---

# 5. Workspace and Crate Topology

Recommended final topology:

```text
floem/
├── crates/
├── foundation/
│   ├── floem-types
│   ├── floem-core
│   └── floem-macros
├── reactive/
│   ├── floem-reactive
│   └── floem-reactive-runtime
├── runtime/
│   ├── floem-runtime-core
│   ├── floem-runtime
│   ├── floem-scheduler
│   └── floem-runtime-dev
├── layout/
│   ├── floem-layout
│   ├── floem-style
│   └── floem-virtual
├── text/
│   ├── floem-text
│   ├── floem-editor
│   └── floem-ime
├── scene/
│   ├── floem-scene
│   └── floem-paint
├── renderer/
│   ├── floem-renderer
│   ├── floem-renderer-wgpu
│   ├── floem-renderer-vello
│   ├── floem-renderer-skia
│   └── floem-renderer-software
├── platform/
│   ├── floem-platform
│   ├── floem-platform-desktop
│   ├── floem-platform-macos
│   ├── floem-platform-windows
│   ├── floem-platform-linux
│   ├── floem-platform-ios
│   ├── floem-platform-android
│   ├── floem-platform-harmony
│   ├── floem-platform-web
│   └── floem-platform-embedded
├── native/
│   ├── floem-native
│   ├── floem-webview
│   ├── floem-video
│   ├── floem-camera
│   └── floem-map
├── framework/
│   ├── floem-widget
│   ├── floem-navigation
│   ├── floem-overlay
│   ├── floem-forms
│   ├── floem-accessibility
│   ├── floem-animation
│   ├── floem-gesture
│   ├── floem-theme
│   ├── floem-i18n
│   └── floem-persistence
├── widgets/
│   ├── floem-widgets
│   ├── floem-widgets-data
│   └── floem-widgets-media
├── app/
│   └── floem-app
├── design-systems/
│   ├── floem-material
│   ├── floem-cupertino
│   ├── floem-fluent
│   └── floem-harmony
├── tooling/
│   ├── floem-cli
│   ├── floem-devtools
│   ├── floem-inspector
│   ├── floem-profiler
│   ├── floem-test
│   ├── floem-snapshot
│   └── floem-agent
└── compiler/
    ├── floem-ui-ir
    ├── floem-dsl
    ├── floem-codegen
    └── floem-hot-reload
```

`app/` was missing from this topology and is added here: nothing else in the
tree can host a widget tree, and the shell (added 2026-09-12, see
`docs/superpowers/decisions/2026-09-12-headless-app-shell.md`) is the top of the
layering.

The exact directory layout may be adapted to the existing Floem repository; the dependency directions below are normative. Which adaptations this repository has
actually made — and why each one stands — is recorded in
`docs/superpowers/decisions/2026-09-15-workspace-topology.md`. The dependency
directions are enforced by `scripts/check-architecture-deps.sh`, whose layer
table is the authoritative answer to "may crate A depend on crate B?".

---

# 6. Dependency Direction Rules

Allowed dependency flow:

```text
floem-types
    ↓
floem-core
    ↓
floem-reactive
    ↓
floem-runtime-core
    ↓
floem-runtime
    ↓
layout / style / text / scene / framework
    ↓
renderer ABI / platform ABI / native ABI
    ↓
concrete backends
```

Rules:

- `floem-types` has no platform/GPU/UI-framework dependencies.
- `floem-scene` has no OS windowing dependency.
- `floem-renderer-*` may depend on scene and renderer ABI, never widgets.
- `floem-widget` may depend on framework primitives, never concrete renderer backends.
- `floem-platform-*` may depend on platform APIs and low-level graphics handles, never application widgets.
- Design systems depend on semantic widgets/theme, not renderer implementations.
- Tooling reads stable inspection protocols, not private runtime internals.
- Native providers depend on `floem-native` and platform bindings, never on application state.

CI must enforce these constraints by checking Cargo dependency graphs.

---

# 7. Core Type Layer

`floem-types` owns small platform-neutral value types:

```rust
Point
Size
Rect
Insets
Constraints
Transform
Color
ScaleFactor
DisplayInfo
WindowId
SurfaceId
ElementId
SceneNodeId
ResourceId
NativeViewId
PlatformId
```

These are value types, cheap to copy where appropriate, and independent from OS APIs.

---

# 8. UI Model

The conceptual split is:

```text
View / Widget = description
Element       = persistent runtime identity/state
Scene         = render description
Native Node   = native compositor participant
```

## 8.1 Widget

Widget is a high-level declarative description.

## 8.2 Element

Element is persistent runtime state stored in an arena/slot map-like structure with stable generation-aware identity.

## 8.3 Layout state

Layout metadata belongs to the runtime, not to a renderer.

## 8.4 Paint state

Paint metadata is converted to Scene nodes; renderer resources live in resource registries.

---

# 9. Element Runtime

Core shape:

```rust
struct Element {
    id: ElementId,
    kind: ElementKind,
    parent: Option<ElementId>,
    children: SmallVec<[ElementId; 4]>,
    state: StateSlot,
    layout: LayoutState,
    interaction: InteractionState,
    dirty: DirtyFlags,
}
```

Implementation may use an arena-backed SoA or hybrid AoS/SoA layout after profiling.

The architecture must separate identity from storage layout so the internal representation can be optimized without public API changes.

---

# 10. Dirty Tracking

```rust
bitflags! {
    struct DirtyFlags {
        const STATE      = 1 << 0;
        const STYLE      = 1 << 1;
        const LAYOUT     = 1 << 2;
        const PAINT      = 1 << 3;
        const HIT_TEST   = 1 << 4;
        const SEMANTICS  = 1 << 5;
        const CHILDREN   = 1 << 6;
        const TRANSFORM  = 1 << 7;
    }
}
```

Invalidation rules must be explicit and tested.

Examples:

```text
text content        → PAINT
font size            → LAYOUT + PAINT
padding              → LAYOUT + PAINT
transform animation  → TRANSFORM
accessibility label  → SEMANTICS
child insertion      → CHILDREN + LAYOUT + PAINT
```

---

# 11. Reactive Architecture

Keep Floem's fine-grained reactive model as a foundational differentiator.

Core primitives:

```text
Signal
ReadSignal
WriteSignal
Memo
Effect
Scope
Batch
Transaction
Selector
Resource
Task
Store
```

The reactive graph is independent of the UI tree.

Flow:

```text
Signal
 ↓
Reactive Node
 ↓
Invalidation
 ↓
Runtime subscriber
 ↓
Element/Property dirty bit
```

A signal must be able to update a Text, Style, Semantics property, or Controller without reconstructing unrelated elements.

---

# 12. Scheduler

Scheduler priorities:

```text
P0 Input
P1 State
P2 Animation
P3 Layout
P4 Paint
P5 Accessibility
P6 Idle
```

Frame pipeline:

```text
Platform Event
 ↓
Input Routing
 ↓
Reactive State
 ↓
Style
 ↓
Layout
 ↓
Paint/Scene Update
 ↓
Composite
 ↓
Present
```

Idle work must never starve frame-critical work.

The scheduler must support platform refresh rates including 60Hz and 120Hz without hard-coded 60Hz timers.

---

# 13. Layout

`floem-layout` exposes a backend-neutral layout API.

Initial backend:

```text
Taffy adapter
```

Potential future backends:

```text
Custom
Embedded-specialized
```

Supported conceptual layout types:

```text
Flex
Grid
Stack
Overlay
Flow
Absolute
Scroll
Virtual
Custom
```

Runtime decides which subtrees require re-layout; the layout backend calculates geometry.

---

# 14. Style

Style is split conceptually into:

```text
Layout
Paint
Typography
Interaction
Motion
```

Style must support:

- inherited typography properties.
- state selectors.
- tokens.
- platform adaptation.
- explicit precedence.
- cheap style updates without unnecessary allocation.

Do not reproduce the entire CSS cascade.

---

# 15. Theme / Design Tokens

Theme structure:

```text
Theme
├── ColorScheme
├── Typography
├── Shape
├── Spacing
├── Elevation
├── Motion
├── Components
└── PlatformAdaptation
```

Design systems:

```text
Material
Cupertino
Fluent
Harmony
Custom
```

The semantic widget stays constant; the design system determines default appearance and interaction language.

---

# 16. Scene Graph

Scene is a renderer-independent retained graph.

```rust
enum SceneNode {
    Group(GroupNode),
    Transform(TransformNode),
    Clip(ClipNode),
    Opacity(OpacityNode),
    Rect(RectNode),
    RoundedRect(RRectNode),
    Path(PathNode),
    Text(TextNode),
    Image(ImageNode),
    Shadow(ShadowNode),
    Filter(FilterNode),
    NativeView(NativeViewNode),
}
```

Native nodes remain separate from normal rendering semantics even if represented by a scene placeholder for z-order tracking.

Scene nodes have stable identities and dirty state.

---

# 17. Renderer ABI

Conceptual API:

```rust
trait Renderer {
    type Frame;
    type Error;

    fn begin_frame(
        &mut self,
        target: &RenderTarget,
        info: &FrameInfo,
    ) -> Result<Self::Frame, Self::Error>;

    fn render(
        &mut self,
        frame: &mut Self::Frame,
        scene: &Scene,
        damage: &DamageRegion,
    ) -> Result<(), Self::Error>;

    fn present(
        &mut self,
        frame: Self::Frame,
    ) -> Result<(), Self::Error>;
}
```

The actual ABI may use associated traits rather than a dynamic trait in hot paths. The public abstraction must not force `Box<dyn RenderCommand>` or other per-node heap allocations.

Renderer backends:

```text
Vello/WGPU
Skia
Software
```

Renderer fast paths should optimize at least:

```text
solid rect
rounded rect
image
text/glyphs
transform
opacity
clip
```

---

# 18. Rendering Caches

Three separate cache classes:

```text
Geometry Cache
Resource Cache
Raster Cache
```

Default policy:

- Geometry cache: aggressive.
- Resource cache: aggressive with size/pressure limits.
- Raster cache: conservative and governed by a cost model.

Memory pressure must allow all caches to release transient data.

---

# 19. Damage Tracking

```rust
struct DamageRegion {
    rects: SmallVec<[Rect; 8]>,
}
```

Damage tracking is an optimization, not a correctness dependency. Full-frame rendering remains a valid fallback.

---

# 20. Text System

`floem-text` owns the application-level text model.

Responsibilities:

```text
Unicode
Bidi
Shaping
Line breaking
Font fallback
Font loading
Glyph cache integration
Selection
Cursor
IME composition
RTL
Emoji
```

Text layout output becomes a renderer-neutral TextRun/Glyph representation.

The renderer should not own shaping logic.

---

# 21. Editor System

`floem-editor` provides reusable document/editing primitives:

```text
TextDocument
Cursor
Selection
UndoManager
RedoManager
Decoration
Transaction
```

This supports:

```text
TextField
TextArea
RichTextEditor
CodeEditor
MarkdownEditor
```

---

# 22. Gesture / Pointer / Focus

Pointer devices unify into a pointer model:

```text
Mouse
Touch
Stylus
Trackpad
```

Additional inputs:

```text
Keyboard
Gamepad
```

Gestures are recognized through a Gesture Arena capable of resolving:

```text
Tap
DoubleTap
LongPress
Pan
Drag
Scale
Rotation
Swipe
Fling
```

Focus model:

```text
FocusManager
FocusScope
FocusNode
FocusRequester
FocusTraversalPolicy
```

---

# 23. Animation

Animation stack:

```text
Tween
Curve
Spring
Keyframe
Timeline
Physics
Transition
SharedElement
```

Composition-only properties such as transform and opacity should bypass normal layout/reactive scheduling whenever safe.

Animation scheduler must be driven by the platform frame clock.

---

# 24. Overlay

Overlay is a first-class runtime concept:

```text
Window
├── Content Tree
├── Floem Overlay Tree
└── System/IME Layer
```

Overlay handles:

```text
Dialog
Popup
Menu
Tooltip
Toast
Snackbar
BottomSheet
ContextMenu
```

Overlay owns modal state, z-order, hit testing, and focus transitions.

---

# 25. Virtualization

Provide a common `VirtualViewport` abstraction for:

```text
VirtualList
VirtualGrid
VirtualTree
VirtualTable
```

Virtualized collections must use stable keys:

```rust
list(items).key(|item| item.id)
```

Only visible/relevant elements should exist in the retained tree.

Native-view virtualization must reuse native view slots.

---

# 26. Native Composition

Native components are a separate graph.

Composition modes:

```text
Overlay
Texture
Surface
Offscreen
```

Runtime selects a mode through capability negotiation and policy.

Example policy:

```text
Static WebView      → Overlay
Transforming View   → Texture when supported
Video/Camera        → Surface when supported
Complex transition  → Offscreen snapshot when useful
```

Native capabilities:

```rust
struct NativeCapabilities {
    can_transform: bool,
    can_clip: bool,
    can_alpha: bool,
    supports_surface: bool,
    supports_texture: bool,
    supports_offscreen: bool,
    supports_ime: bool,
    supports_accessibility: bool,
    supports_snapshot: bool,
}
```

---

# 27. Native Plugin ABI

Use typed native APIs and code generation.

Three communication classes:

```text
Typed Commands
Typed Events
Shared Memory / Buffer Resources
```

Do not use JSON string channels for video frames or high-frequency data.

Support generated bindings for:

```text
C
C++
Swift
Kotlin
Harmony native APIs
```

---

# 28. Platform ABI

Core platform responsibilities:

```text
Window
Surface
Event loop
Frame clock
Lifecycle
Display info
Input
Capabilities
```

Platform services are extensions:

```text
Clipboard
IME
Accessibility
File picker
Notification
Camera
Media
WebView
Sensors
```

Platform implementations:

```text
macOS
Windows
Linux
Android
iOS
HarmonyOS
Web
Embedded
```

HarmonyOS must have an independent implementation crate; Android concepts may be shared only at the capability/model layer.

---

# 29. Lifecycle

Standard lifecycle states:

```text
Launching
Active
Inactive
Background
Suspended
Resumed
Closing
Closed
```

On suspension the runtime should pause animation, frame production, and non-essential work and release transient GPU resources when appropriate.

---

# 30. Accessibility

Accessibility is represented as a Semantics Tree:

```rust
Semantics {
    role,
    label,
    value,
    hint,
    enabled,
    selected,
    checked,
    actions,
}
```

The Semantics Tree must support both Floem elements and native subtrees.

Platform adapters:

```text
macOS Accessibility
Windows UI Automation
Linux AT-SPI
iOS UIAccessibility
Android Accessibility
Harmony accessibility APIs
Web DOM accessibility bridge
```

---

# 31. Input Method Editor

IME is platform-first and focus-driven.

The focused target can be:

```text
Floem TextInput
Rich Editor
WebView
Native Input View
```

No widget should implement its own platform keyboard stack independently.

---

# 32. Application State

Framework primitives:

```text
Local State
Shared Store
Resource State
Persistent State
Task
Service
```

Do not mandate Redux, MVVM, Elm, Actor, or Entity architecture.

The framework supplies primitives; application architecture remains optional.

Store API should support selective subscriptions:

```rust
store.select(|s| s.user.name)
```

and mutation transactions:

```rust
store.update(|s| { ... })
```

---

# 33. Dependency Injection and Services

Use scoped services rather than one mandatory global container.

Conceptual scope hierarchy:

```text
AppScope
├── WindowScope
│   └── PageScope
│       └── FeatureScope
└── Global Services
```

Service lifetime follows scope lifetime where possible.

---

# 34. Async / Tasks / Resources

Provide executor-neutral APIs:

```text
spawn
spawn_local
spawn_blocking
ui.spawn
```

Resource state:

```text
Initial
Loading
Ready
Refreshing
Error
```

Tasks must be cancellable and scope-bound.

---

# 35. Navigation

Navigation is state, not merely imperative UI operations.

Route example:

```rust
enum Route {
    Home,
    Profile { id: UserId },
    Settings,
}
```

Router responsibilities:

```text
URL/deep-link parsing
Route matching
Restoration
Navigation state
```

Navigator responsibilities:

```text
Push
Pop
Replace
Transition
```

Containers:

```text
Stack
Tabs
SplitView
NavigationRail
Drawer
Modal
```

---

# 36. Persistence / Restoration

Capabilities:

```text
KeyValue
File
SecureStorage
Database adapter
Cache
```

State restoration should be explicit and serializable.

Memory pressure levels:

```text
Normal
Warning
Critical
```

Critical pressure should trigger cache trimming and prefetch cancellation.

---

# 37. Error Boundaries

Application-recoverable component errors may be contained by the nearest ErrorBoundary.

Fatal runtime corruption remains fatal.

Error boundaries must preserve diagnostics and provide a developer-visible stack/context in debug builds.

---

# 38. Widget Library

Tier 0 primitives:

```text
Text
Icon
Image
Container
Spacer
Divider
```

Tier 1 controls:

```text
Button
TextField
Checkbox
Radio
Switch
Slider
Select
Progress
```

Tier 2 navigation/layout:

```text
List
Grid
Stack
Tabs
NavigationBar
NavigationRail
Drawer
Dialog
BottomSheet
Popup
Tooltip
```

Tier 3 data:

```text
DataGrid
Table
Tree
VirtualList
VirtualGrid
Carousel
Charts
```

Tier 4 heavy/native:

```text
WebView
Video
Camera
Map
RichTextEditor
CodeEditor
```

Component APIs should prefer semantic properties such as `variant`, `size`, `loading`, `disabled`, and slots over large monolithic style structs.

---

# 39. Style / Variant / Slot Model

A component exposes:

```text
Structure
Behavior
Appearance
```

Slots are explicit:

```text
dialog()
    .title(...)
    .content(...)
    .actions(...)
```

Variants are semantic:

```text
Primary
Secondary
Danger
Text
```

The design system determines actual visual values.

---

# 40. UI IR

`floem-ui-ir` is an interchange and tooling representation.

Example:

```json
{
  "type": "Column",
  "spacing": 16,
  "children": [
    {"type": "Text", "text": "Welcome", "style": "headline"},
    {"type": "Button", "variant": "primary", "text": "Continue"}
  ]
}
```

UI IR supports:

```text
Rust ↔ UI IR
DSL ↔ UI IR
Figma ↔ UI IR
AI ↔ UI IR
DevTools ↔ UI IR
```

UI IR is not used as the release runtime representation.

---

# 41. DSL

Optional declarative syntax should map to UI IR and then compile/build into native Rust/runtime representations.

Production mode:

```text
DSL → AST → UI IR → optimized builder/codegen
```

Development mode may use incremental interpretation for hot reload.

Rust APIs remain first-class.

---

# 42. AI / MCP

Development-only agent endpoint exposes an inspection and control protocol:

```text
get_ui_tree()
get_element(id)
get_style(id)
get_layout(id)
get_scene(id)
get_reactive_dependencies(id)
get_frame_profile()
screenshot()

click(id)
type(id, text)
scroll(id, amount)
focus(id)
```

All inspection is based on stable snapshot schemas rather than private structures.

AI workflow target:

```text
Figma
 ↓
Design extraction
 ↓
UI IR
 ↓
Floem code
 ↓
Run
 ↓
Screenshot / metrics
 ↓
MCP inspection
 ↓
AI correction
```

---

# 43. Figma / Design Contract

Each design/component may emit a contract:

```text
Dimensions
Spacing
Typography
Colors
Radius
State variants
Interaction
Accessibility
Animation
```

Runtime inspector can compare expected vs actual values.

This is intended to support objective design restoration rather than screenshot-only approximation.

---

# 44. DevTools

Development tooling must support:

```text
Widget/Element Tree
Layout Inspector
Style Inspector
Reactive Dependency Inspector
Scene Inspector
Native Composition Inspector
Frame Timeline
CPU/GPU timing
Allocation counts
Memory
Accessibility Tree
```

The key diagnostic question must be answerable:

```text
Why did this element update?
```

The inspector should show the reactive dependency or invalidation source.

---

# 45. Testing

Required test classes:

```text
Unit
Widget
Interaction
Snapshot
Semantics
Integration
Platform
Performance
Memory
```

A software renderer/headless runtime is mandatory for deterministic CI snapshots where practical.

Selectors should support:

```text
Element ID
Semantic label
Test ID
Accessibility role
```

rather than coordinate-only tests.

---

# 46. Hot Reload

Stage 1: UI DSL/style/layout hot reload.

Stage 2: generated UI builder/module hot reload.

Stage 3: optional dynamic code module replacement for supported targets.

Do not require arbitrary Rust code hot replacement in the initial implementation.

---

# 47. Performance Budgets

These are engineering targets, not universal guarantees:

```text
Idle CPU:           target < 0.5–1% desktop baseline
Frame CPU @ 60Hz:   target < 4–6ms for representative UI
Animation:          60/120Hz where platform allows
Local text update:  no whole-tree layout/paint
10k-row list:       virtualized
Memory allocations: hot path should approach zero
Binary size:        feature-dependent and measured per target
Startup:            measured cold and warm per platform
```

Regression policy:

```text
>10% CPU regression       fail/warn by benchmark class
>10% allocation regression fail/warn
>10% frame regression      fail/warn
memory budget exceeded     warn/fail according to target tier
```

Exact hard thresholds must be calibrated from P0 baseline runs on representative hardware.

---

# 48. Memory Strategy

Memory optimization principles:

1. Stable IDs with compact runtime storage.
2. Avoid one heap allocation per widget/node.
3. SmallVec/small-string-like optimizations only where profiling validates them.
4. Shared immutable resources.
5. Explicit GPU cache budgets.
6. Cache eviction under memory pressure.
7. Native view pooling for virtualization.
8. Render thread consumes immutable scene snapshots rather than locking UI state.
9. Production build strips developer inspection payloads.

---

# 49. Threading Model

Baseline model:

```text
Platform/Main thread
        ↓
UI thread/runtime
        ↓
Immutable Scene
        ↓
Render thread
        ↓
GPU
```

Worker pool:

```text
Image decode
Font loading
Network
File I/O
CPU-intensive preparation
```

Scene should be cheaply transferable to the render thread and should avoid mutable platform-specific state.

---

# 50. Compatibility Strategy

Do not rewrite the existing Floem API in one step.

Introduce:

```text
Legacy View
   ↓
LegacyViewAdapter
   ↓
New Element Runtime
```

Allow old applications to compile while new widgets use the 2.0 runtime directly.

Compatibility aliases such as:

```rust
type ViewId = ElementId;
```

may be used during migration where semantically safe.

---

# 51. P0–P4 Roadmap

## P0 — Runtime and boundaries

Deliver:

```text
Core types
Element IDs/Arena
Dirty tracking
Reactive bridge
Scheduler
Scene ABI
Renderer ABI
Platform ABI
Legacy adapter
Benchmark suite
Dependency boundary tests
```

## P1 — Complete native UI framework

Deliver:

```text
Style
Theme
Layout
Text
Focus
Gesture
Animation
Overlay
Virtualization
Accessibility
Core widgets
Forms
Navigation
```

## P2 — Cross-platform/native completion

Deliver:

```text
macOS
Windows
Linux
Android
iOS
HarmonyOS
Web
Embedded
WebView
Video
Camera
Map
Native codegen
```

## P3 — Tooling and AI

Deliver:

```text
DevTools
Inspector
Profiler
Snapshot testing
UI IR
DSL
Hot reload
MCP/Agent API
Figma importer
Design Contract
```

## P4 — Ecosystem maturity

Deliver:

```text
Material
Cupertino
Fluent
Harmony
Package templates
Docs
Examples
Migration guides
Third-party plugin conventions
Stability policy
Release engineering
```

---

# 52. Phase Exit Criteria

P0 exits only when:

- New runtime can host at least a minimal Text/Button/Counter application.
- Existing Floem demo applications continue to run through the compatibility layer.
- Renderer and Platform APIs have no widget-specific dependency.
- Dirty updates can be measured without whole-tree rebuild.
- Software renderer can produce deterministic snapshots.
- Baseline CPU/memory/allocation benchmarks exist.

P1 exits only when:

- A production-style multi-page form/list app can be built without custom framework patches.
- 120Hz capable animation path exists where supported.
- A11y semantics and IME are functional on at least one desktop and one mobile target.
- Virtualized 10k+ item lists are stable.

P2 exits only when:

- All eight target classes have working platform backends or clearly documented supported subsets.
- Native WebView/video/camera composition works on desktop and mobile.
- No platform-specific logic leaks into widget code.

P3 exits only when:

- Inspector can map screen → Element → Scene → source location.
- Profiler identifies frame and reactive costs.
- Agent/MCP can inspect and interact with a running app.
- UI IR can round-trip basic component trees.

P4 exits only when:

- Component library, design systems, docs, migration examples, package conventions, and release criteria are production ready.

---

# 53. Architectural Risks

## Risk: Runtime over-engineering

Mitigation: keep P0 APIs minimal and benchmark every abstraction.

## Risk: Renderer fragmentation

Mitigation: Scene/Renderer ABI remains tiny; backends own optimization.

## Risk: Platform abstraction leaks

Mitigation: capability checks plus per-platform crates.

## Risk: Memory growth

Mitigation: cache budgets, arena profiling, memory pressure hooks, benchmark gates.

## Risk: Native composition complexity

Mitigation: start with Overlay and platform-native Surface paths; Texture/Offscreen only where needed.

## Risk: DSL becomes a second framework

Mitigation: DSL compiles to UI IR and Rust/runtime primitives; Rust remains canonical.

## Risk: AI tools become unstable private APIs

Mitigation: versioned inspection snapshots and stable agent protocol.

---

# 54. Final Architectural Decision

The core Floem 2.0 identity is:

```text
Fine-grained Reactive
        +
Retained Element Runtime
        +
Incremental Layout/Paint
        +
Renderer-independent Scene
        +
Capability-based Platform ABI
        +
Native Composition
        +
Flutter-level Widget completeness
        +
UI IR / DSL
        +
DevTools / MCP / AI
```

This is the architecture to implement incrementally rather than a requirement to create every crate on day one.
