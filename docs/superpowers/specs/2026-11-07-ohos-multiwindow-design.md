<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# OHOS multi-window — design

> Status: approved design, pending implementation plan.
> Scope decision: full (A + B + C), demo = C2 (rebuild the deleted bridges too),
> pairing model = **app-driven**.

## Problem

`floom-winit-ohos` is a **hard-coded single-window** backend. Every OHOS app gets
exactly one native surface and cannot create a second window. A desktop app needs
multi-window: two documents side by side, an inspector, a palette. This design
makes the OHOS backend multi-window.

Evidence of the single-window assumption (all `crates/vendor/floom-winit-ohos/`):

- `src/event_loop.rs:43` — `const GLOBAL_WINDOW: WindowId = WindowId::from_raw(0);`
- `src/event_loop.rs:837-852` — `create_window` returns
  `NotSupportedError("only a single OHOS window is supported")` on the second call.
- `src/event_loop.rs:138-141` — `SharedWindowState { inner: Mutex<WindowInner> }`
  is one global `WindowInner`, not a per-window set.
- `src/event_loop.rs:972-974` — `Window::id()` always returns `GLOBAL_WINDOW`.
- Input / lifecycle events are dispatched to `GLOBAL_WINDOW` throughout
  (`event_loop.rs:291, 329, 337, 449, 464, 478, …`).

Two upstream facts constrain the design:

- **A "window" on OHOS is an ArkTS `XComponent`** (a native surface). Surfaces are
  created by the ArkTS page and handed to Rust as `SurfaceEvent { xcomponent,
  native_window, width, height, density_scale, font_scale }` (`src/app.rs:51-58`).
  This event carries **no window id** — it assumes one surface.
- **The ArkTS shell was deleted.** `examples/ohos` (the HAP project: `FloomBridge.ets`,
  `FloomA11y.ets`, `EntryAbility.ets`, the `ohos-app/` hvigor project) was removed in
  commit `801bc4a1` ("delete v1 from the root workspace"), and the P2 refactor `a370d6c8`
  reorganized `crates/`. The N-API bridges those `.ets` files drove (IME, clipboard,
  file, a11y, external, font) are **not in the current tree** — grep for
  `ohos_ime` / `ohos_bridge` / `register_platform_bridge` / `ArkUI_AccessibilityProvider`
  hits 0 files. `crates/platform/floom-platform/src/harmony/` holds only
  `accessibility.rs` (a stub: `NATIVE_BRIDGE` unset, keeps only the semantics tree)
  and `mod.rs`.

`docs/ohos.md` still describes the pre-refactor, pre-deletion state (IME / a11y /
clipboard bridges "done", gate 1 "pass", on-device verified 2026-09-27). Those claims
are stale relative to the current tree. Reconciling `docs/ohos.md` is a follow-up,
out of scope here.

## Goals

1. `create_window` can be called multiple times; each returns an independent
   `Window` with a **distinct** `WindowId`.
2. Per-window state: surface, lifecycle, focus, scale, IME caps — one slot per window.
3. Surface and input events route **by window id** to the right window.
4. Surfaces are created on demand: the app asks for a window, and an ArkTS
   `XComponent` is spawned to back it, paired by id.
5. A runnable HAP demo that shows **2+ windows rendering on the emulator**, each
   receiving its own input.
6. Rebuild the N-API bridges the deleted shell drove (clipboard / file / external /
   IME / a11y / font), with IME and a11y made **per-window**.

## Non-goals

- No desktop-only features beyond multi-window: `window_title` (OHOS apps are
  full-screen, no titlebar), `drag_drop`, custom cursors, window dragging /
  positioning / fullscreen management remain unsupported. The backend's README
  already declares these out of scope.
- Not resurrecting the full deleted HAP (with all its a11y / IME scaffolding) verbatim;
  the bridges are rebuilt to the shape `docs/ohos.md` documents, minimal and
  multi-window-aware.
- No `docs/ohos.md` rewrite here (separate follow-up; it currently overstates state).

## Decisions (from the design review)

| Question | Decision |
| --- | --- |
| How deep is the "skeleton"? | **A + B + C** — Rust per-window architecture, surface-protocol extension, *and* the ArkTS multi-`XComponent` shell (2+ windows really render on the emulator). |
| How complete is the demo? | **C2** — rebuild the deleted IME / clipboard / file / a11y bridges too, not just a bare shell. |
| How do a Rust `create_window` call and an OHOS surface pair up? | **App-driven** — `create_window(id)` → Rust creates the handle immediately and notifies ArkTS via a bridge to spawn an `XComponent` for that id → the surface callback comes back tagged with the id and binds. |
| Sequencing | **Vertical slice first** — M1 → M2 → M3 get "2 windows render" working end-to-end (highest-risk pairing validated earliest); M4 bridge rebuild last, phasable. |

## Architecture

```
floom / floom-app  WindowConfig { id }
        │  active_event_loop.create_window(attrs { window_id })
        ▼
┌──────────────────────────────────────────────────────────────┐
│ floom-winit-ohos (Rust)                                        │
│   SharedWindowState { windows: HashMap<WindowId, WindowInner>, │
│                       next_id: AtomicU64 }                     │
│   Window { id, shared, ohos_app }                              │
│   events routed by id                                          │
└──────────────────────────────────────────────────────────────┘
        │  createWindow(id) / destroyWindow(id)   [N-API → JS thread]
        ▼
┌──────────────────────────────────────────────────────────────┐
│ ArkTS page (examples/ohos HAP)                                 │
│   @State windowIds: number[]                                   │
│   ForEach(windowIds, id => XComponent({ id, type:'surface' })) │
└──────────────────────────────────────────────────────────────┘
        │  onSurfaceCreated(id, xcomponent, nativeWindow, w, h, scales)  [FFI]
        ▼
floom-winit-ohos (Rust)  →  binds surface to WindowInner[id]
        │  app.window_event(target, id, WindowEvent::…)
        ▼
ApplicationHandler
```

## 1. Window id and the pairing protocol (core)

- `PlatformSpecificWindowAttributes` (currently the empty unit struct,
  `event_loop.rs:50-57`) gains `window_id: Option<u64>`. The app sets it when it
  wants a stable id; `None` → the backend auto-assigns the next id.
- Pairing timeline:
  1. App calls `create_window(attrs{window_id: Some(id)})`. Rust inserts a fresh
     `WindowInner` keyed by id (state `Uninitialized`, no surface yet) and returns
     `Window { id }` **immediately** (honors the sync `create_window` contract).
  2. Rust invokes the N-API bridge `createWindow(id)`. This crosses from the
     event-loop worker thread to the ArkTS JS thread (same marshalling pattern the
     platform bridges already use).
  3. ArkTS appends `id` to `@State windowIds`, so ArkUI's `ForEach` mounts a new
     `XComponent` with that id.
  4. That `XComponent`'s `onLoad` → `NativeXComponent` → FFI callback
     `onSurfaceCreated(id, xcomponentPtr, nativeWindow, w, h, density, font)`.
  5. Rust binds the surface into `WindowInner[id]` (state `SurfaceReady`) and
     dispatches `WindowEvent::SurfaceReady`-equivalent for that id.
- Destruction is symmetric: `destroyWindow(id)` → remove from `windowIds` → the
  `XComponent` unloads → `onSurfaceDestroyed(id)` → drop `WindowInner[id]`.

Why app-driven: floem's `create_window` returns `Box<dyn CoreWindow>` synchronously;
the app (not the shell) owns window count. ArkTS-driven (shell owns N fixed
`XComponent`s, app reacts to them appearing) would invert that contract and fight
`RootActiveEventLoop::create_window`. The cost is one new Rust→ArkTS
"create-window" bridge plus dynamic `XComponent` rendering in ArkUI.

## 2. Rust backend changes (M1)

```rust
struct SharedWindowState {
    windows: Mutex<HashMap<WindowId, WindowInner>>,  // was: inner: Mutex<WindowInner>
    next_id: AtomicU64,
}

struct Window {
    id: WindowId,                       // NEW: real per-window id
    shared: Arc<SharedWindowState>,
    ohos_app: OhosApp,
}
```

- `create_window` drops the `NotSupportedError`; inserts a `WindowInner` keyed by
  the resolved id and returns `Window { id }`.
- `Window::id()` returns `self.id` (not `GLOBAL_WINDOW`).
- `xcomponent_ptr` / `native_window_ptr` / `density_scale` / `font_scale` /
  `window_handle` / `update_surface_state` read `self.shared.windows[id]`.
- `WindowInner` keeps its current fields (lifecycle, xcomponent, native_window,
  surface_size, scales, focus, IME caps, …) — now one instance per window.
- Single-window behavior is preserved: with one window the map degenerates to the
  old single-slot semantics.
- `GLOBAL_WINDOW` becomes the fallback id when the app passes `window_id: None` for
  the first window (keeps existing single-window call sites working).

## 3. Surface / input routing (M2)

- `SurfaceEvent` gains `window_id: u64`; `HostEvent::SurfaceCreated / Destroyed /
  Changed` carry it.
- **Input events must carry a target window id from ArkTS.** Today `TouchEvent` /
  `MouseEvent` / `KeyEvent` are global because there is one window. For multi-window,
  the ArkTS `XComponent`-level input callbacks know which `XComponent` (hence which
  id) they fired on; the FFI passes that id so Rust dispatches
  `app.window_event(target, id, WindowEvent::…)` to the right window. Without this,
  every input lands on the wrong window the moment there are two.
- `focused` / `RedrawRequested` / `FrameAvailable` become per-id.

## 4. ArkTS multi-`XComponent` shell (M3 — the demo)

Resurrect a minimal `examples/ohos` HAP:

- `EntryAbility.ets` starts a page; the page holds `@State windowIds: number[]`
  and renders `ForEach(windowIds, id => XComponent({ id: String(id), type: 'surface',
  library: … }).onLoad(cb))`.
- Each `XComponent.onLoad` captures its `NativeXComponent`, registers the surface
  and input callbacks, and reports `(id, ptr, nativeWindow, …)` to Rust via FFI.
- New N-API bridge `createWindow` / `destroyWindow`: called from the Rust event-loop
  worker, marshalled to the JS thread, mutates `windowIds` (driving `@State`).
- **Acceptance (user, on emulator):** a floem app opens 2 windows; both surfaces
  render; taps/keys on each go to that window only. This is the GUI-demo gate —
  **not done until the user confirms on the emulator.**

## 5. N-API bridge rebuild (M4 = C2)

Rebuild to the shape `docs/ohos.md` documents (pure-Rust N-API, in
`crates/platform/floom-platform/src`, not vendored):

- Single entry `register_platform_bridge` + `call_bridge` helper (per the doc).
- Modules: `ohos_clipboard`, `ohos_file`, `ohos_external` (share / url),
  `ohos_ime`, `ohos_a11y`, `ohos_font`.
- **IME and a11y must be per-window.** `docs/ohos.md` itself flags that the old
  design used a single global IME queue and a single global `ACTION_TX`
  (a11y reverse-action channel) and that multi-window requires per-window routing
  (`docs/ohos.md:382-385`, `:603-605`). M4 must key these by window id.
- Wire capability bits in `crates/platform/floom-platform/src/harmony/mod.rs`
  (`ACCESSIBILITY`, `IME`, `CLIPBOARD`, …) to reflect what the rebuilt bridges provide.
- **Acceptance (user, on emulator):** clipboard read/write, file pick, IME text,
  and a11y act on the correct window. Rebuilt per-window — **not done until the
  user confirms on the emulator.**

M4 is the largest piece and can be split further (clipboard/file/external first —
window-independent — then IME/a11y, which must be per-window).

## Milestones and acceptance

| Milestone | Deliverable | Acceptance gate |
| --- | --- | --- |
| M1 | Rust per-window architecture; multi-callable `create_window`; real ids | `cargo build --target aarch64-unknown-linux-ohos` green; single window still works |
| M2 | `SurfaceEvent.window_id`; multi-surface FFI; input routing by id; `createWindow`/`destroyWindow` bridge | compiles green |
| M3 | Minimal multi-`XComponent` HAP + a floem app that opens 2 windows | **user runs on emulator: 2 windows render, input routed per window** |
| M4 | 6 N-API bridges + capability bits + per-window IME/a11y | **user runs on emulator: each bridge works on the correct window** |

## Risks / open questions

1. **Dynamic `XComponent` insertion in ArkUI** — ArkUI is declarative; spawning an
   `XComponent` on demand means mutating a `@State` array and rendering it via
   `ForEach`. Need to confirm `XComponent` (surface type) re-mounts cleanly when the
   array grows, and that each instance's surface callback reports its own id.
   **De-risk in M2/M3 with a throwaway page before committing the shell.**
2. **Worker→JS-thread marshalling for `createWindow`** — the platform bridges
   already marshal calls onto the JS thread; reuse that pattern. `createWindow` is
   called from the event-loop worker, so it must not block the pump.
3. **Input routing fidelity** — getting a per-`XComponent` target id from the
   ArkTS input callbacks is the piece most likely to be fiddly (touch coordinates
   are already window-local, but the *routing* must be explicit).
4. **`docs/ohos.md` is stale** — it describes bridges and an `examples/ohos` that
   are gone. Reconcile it as a follow-up so it stops overstating the backend; do not
   fold it into the M1–M4 implementation.
5. **Verification requires the user's DevEco + emulator** for M3/M4 acceptance
   (HAP build + on-device run). Host `cargo check` cannot exercise rendering.
