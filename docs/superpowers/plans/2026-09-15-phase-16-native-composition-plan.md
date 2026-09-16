# Plan: Phase 16 — native composition (R4)

- **Date:** 2026-09-15
- **Plan:** `docs/superpowers/plans/2026-09-02-floem-2-complete-implementation-plan.md` (§Phase 16)
- **Ledger:** `.superpowers/sdd/progress-floem-2.md` (Roadmap R4)
- **Status:** not started. Deliberately landed as a plan: the phase's *last* mile
  needs real devices, but its first two thirds do not, and R1–R3 have just made
  the headless half cheap to test.

## Why this is a plan rather than a branch

Phase 16 is the one roadmap item that cannot be finished on this machine. Real
native views mean real `NSView` / `HWND` / `SurfaceView` / KMS planes, and the
questions that decide the design — does the compositor honour z-order, does the
surface punch through the UI layer, who owns IME while a native text field is
focused — are answered by an OS, not by a test.

Writing the phase off as "device work" would be wrong in the other direction:
of the four tasks, two (16.1, 16.4) and the majority of a third (16.2) are
pure bookkeeping against a provider trait, and are as testable today as the
overlay stack was. So this document splits every task into

- a **headless half** — ABI, contracts, host-side composition, fake-provider
  tests, shippable now; and
- a **device half** — enumerated as numbered cases with a platform, a setup, an
  expected observation and a falsifier, to be run in one pass once the
  headless halves are all in (the same batching the R1/R2/R4 device list uses).

The device cases are the deliverable this plan exists to protect: they are the
part that gets lost when a phase is abandoned half-done, so they are written
down as *cases*, not as prose about what "should" work.

## What already exists

| Piece | Where | State |
| --- | --- | --- |
| `NativeViewId` | `foundation/floem-types/src/ids.rs` | done |
| `NativeSlot` (id + generation), `NativeViewPool<K>` | `native/floem-native/src/pool.rs` | done (Phase 12.2 precursor) |
| `Capability::NATIVE_VIEW` | `platform/floem-platform/src/capabilities.rs` | flag exists (`1 << 10`); set by `web_default` only, **not** by `desktop_default` |
| `SurfaceTarget` (raw-window-handle pair, size, scale) | `platform/floem-platform/src/surface.rs` | done |
| Scene nodes | `scene/floem-scene/src/node.rs` | `Group`/`Rect`/`Image`/`Text` — **no hole/participant variant** |
| `Renderer` trait | `render/floem-renderer/src/lib.rs` | `begin_frame`/`render_scene`/`render_scene_with_resources`/`end_frame`/`trim_caches` — **no native/participant hook** |
| Host paint, overlay stack, frame clock, damage | `app/floem-app/src/lib.rs` | done (R2) |
| Headless harness | `tooling/floem-testkit` | done (R3) — `TestApp`, `Selector`, actors, `settle` |

Nothing above `NativeViewPool` exists: no `NativeViewProvider`, no controller,
no host participant registry, no way for a widget to say "this node is a native
view". A repository-wide search for `native_view` under `crates/ widgets/
framework/ app/ layout/ scene/` returns nothing.

## Where it plugs in (the four seams)

1. **Widget → host.** A widget wants a native view in its box (a map, a video, a
   web view). Needs a door symmetric with `EventContext::open_overlay` /
   `UpdateContext::open_overlay` (R2): the widget declares intent, the host owns
   the participant's lifetime, because a native view must outlive a frame and be
   destroyed exactly once.
2. **Host → platform.** The host computes, per frame, for every live
   participant: logical rect, visibility, clip, z-order, and input mode. The
   diff of that against last frame is what the platform adapter applies. This is
   the same shape as the scene's `upsert` (identical payload ⇒ no damage) and the
   overlay stack's command queue, and it should reuse that idiom rather than
   invent a third.
3. **Platform → OS.** A provider creates/binds a native handle to a
   `NativeSlot.id`, applies geometry, raises/lowers it, hides it, and answers
   capability queries. Providers live in the platform crates, one per OS.
4. **OS → framework (input, focus, IME).** A native view is a real view: it
   takes OS input itself and, if focusable, joins the OS focus chain. The
   framework must therefore *subtract* rather than add: stop routing pointer
   events into a region a capturing native view covers, yield focus when the
   native view takes it, and suspend its own IME cursor-area ownership while a
   native text field is composing (R1's bridge assumes the framework owns that).

## Decisions to take before Task 16.1 is written

**D1 — Composition mode is per participant, negotiated, never assumed.**
`NativeCompositionMode { Overlay, Texture, Surface, Offscreen }` (plan §16.1)
stays, but a provider answers *which* it can do for a given view; the host picks
the strongest available and records the choice. Rationale: X11 can put a
subwindow above a surface, Wayland cannot (no subwindows at all), Windows child
HWNDs cannot be composited under D3D without airspace tricks, KMS has real
hardware planes, and the web has DOM. One mode cannot be correct everywhere, and
a capability query at bind time is cheaper than a per-platform fork in the host.

**D2 — Overlay mode composes *above* the whole scene, and that is a documented
limitation, not a bug to paper over.** A native view can be occluded by another
native view but not by framework-painted content, because the framework draws
into one surface and the native view is a sibling above it. Consequences to
write down: any framework widget that must paint over a native view needs
`Texture`/`Offscreen` mode, and the host should refuse to *silently* enable
Overlay for a participant that a later-painted scene node overlaps. Detecting
that overlap is cheap (the participant's rect vs the scene nodes painted after
it) and turning it into either a downgrade or a loud debug assertion is exactly
the kind of thing that is testable headlessly.

**D3 — Clipping is the host's job.** A participant's effective rect is its
node's box intersected with every ancestor clip and the viewport. Providers get
the already-intersected rect plus a `visible: bool`; they never walk a tree. The
classic failure here is a native view floating over a scrolled-away region or
over a modal scrim, and it is entirely a host-side computation — i.e. R3 can
assert it.

**D4 — Focus and IME are exclusive, and the framework yields.** When focus
enters a native participant, the host calls `controller.focus()` and marks its
own focus as suspended; `Ime` cursor-area updates stop until the participant
reports blur. Rationale: two owners pushing candidate-window geometry is a
visible bug (the candidate window jumps between the framework caret and the
native field), and R1 made the framework an owner, so the handoff has to be
explicit.

**D5 — `floem-native` stays a leaf.** ABI traits and the participant registry go
in `native/floem-native` (extending `lib.rs` as the Phase 12.2 decision
promised), gaining `floem-overlay`-style nothing: the crate keeps depending only
on `floem-types` + `floem-virtual` + `floem-scene`-free basics. Host-side
composition lives in `app/floem-app`; OS bridges live in the platform crates.
This keeps `check-architecture-deps.sh` green and keeps the ABI usable by a
platform backend without dragging in the framework.

---

## Task 16.1 — Define the Native ABI *(headless, shippable now)*

**Status: done, `d25bf07`** — 30 tests. One deviation: the three shared enums
ended up in `foundation/floem-types/src/native.rs` rather than in `abi.rs`,
because a widget at L6 asks for a participant and a provider at L4 implements
one, and neither crate may depend on the other's.

**Files**

- Modify: `native/floem-native/src/lib.rs`, `native/floem-native/Cargo.toml`
- Create: `native/floem-native/src/abi.rs`, `native/floem-native/src/fake.rs`
- Test: `native/floem-native/tests/contract.rs`

**Interfaces** (shapes, not final signatures)

```rust
pub enum NativeCompositionMode { Overlay, Texture, Surface, Offscreen }

pub struct NativeCapabilities {
    pub modes: &'static [NativeCompositionMode], // strongest first
    pub snapshot: bool,     // can freeze itself into a texture
    pub input_modes: InputModes, // passthrough / capture
    pub focusable: bool,
}

pub trait NativeViewProvider {
    fn capabilities(&self) -> NativeCapabilities;
    /// Bind a concrete handle to a slot. `generation` says whether this is a
    /// fresh slot or a recycled one that must be re-bound to a new handle.
    fn create(&mut self, slot: NativeSlot, kind: NativeViewKind) -> Result<(), NativeError>;
    fn destroy(&mut self, slot: NativeSlot);
    fn apply(&mut self, slot: NativeSlot, state: NativeState);
    fn focus(&mut self, slot: NativeSlot);
    fn blur(&mut self, slot: NativeSlot);
    /// Freeze to a texture for a transition; `None` when unsupported.
    fn snapshot(&mut self, slot: NativeSlot) -> Option<NativeSnapshot>;
    fn restore(&mut self, slot: NativeSlot);
}

pub struct NativeState {
    pub rect: Rect,           // already clipped, already device-rounded
    pub visible: bool,
    pub z: u32,
    pub input: InputMode,
    pub mode: NativeCompositionMode,
}
```

**Steps**

1. Write `FakeNativeProvider` first, recording every command into a `Vec` with
   the slot it targeted. This is the test surface for the whole phase, and it is
   what makes 16.2–16.4 testable without a device.
2. Implement the ABI traits and `NativeError`; keep them object-safe so the host
   can hold `Box<dyn NativeViewProvider>`.
3. Implement lifecycle invariants *in the provider contract* and test them
   against the fake: `destroy` for every `create`; a recycled slot is always
   re-`create`d before any `apply`; `apply` never carries a rect with negative
   extent; `snapshot` twice without `restore` is an error, not a silent second
   freeze.
4. Add `NativeViewKind` as an opaque, provider-interpreted tag (map/video/web/
   custom) so the ABI can name intent without the framework knowing concrete
   OS types.
5. Commit.

**Done when** the fake provider can drive the full lifecycle through the contract
tests, and the tests fail if `create`/`destroy` are unbalanced or a recycled
slot is reused without re-binding.

**Device cases:** none. This task is the reason the phase is not "all device".

---

## Task 16.2 — Overlay composition *(headless half + device half)*

**Status: steps 1–6 and 8 done, `dab2547`** (+ `6773950`, the hit-test rule
they depend on). `app/floem-app/tests/native.rs`, 16 tests. Deviations and
findings worth carrying forward:

- The native door is answered **inline**, not queued like the overlay door: a
  participant only touches fields disjoint from the node being updated, so a
  widget can learn within the same frame whether it got a view or must draw a
  fallback. Queueing would have made `NativeSurface` swallow its own refusal.
- Host-minted slot keys had to be a separate namespace from caller keys
  (`SlotKey::{Caller,Minted}`). A list numbering rows from 0 collided with the
  host's counter and the second participant silently never appeared.
- Step 4's "the participant wins" turned out to need a `HitTester` change too,
  not just a host change: native regions now beat framework regions outright,
  because in Overlay mode the participant is stacked above the *whole* scene.
- **Open question (unsettled):** `NativeState` has no clip channel, so a
  participant clipped by an ancestor is reported as the intersection of its box
  with the clips. A provider that cannot clip at the OS level will shrink its
  content instead of cropping it. Either the ABI grows a clip rect or
  Overlay-mode providers clip themselves; D6 is where this shows.
- Step 7 (the desktop bridge) is untouched, and `desktop_default` still does
  **not** set `Capability::NATIVE_VIEW` — correctly, since there is no desktop
  provider yet.

**Files**

- Modify: `app/floem-app/src/lib.rs` (participant registry, frame diff, paint
  order, hit-test subtraction, focus yield)
- Modify: `foundation/floem-core` (`UpdateContext`/`EventContext` native door)
- Modify: `widgets/floem-widgets` (a `NativeSurface` composite widget holding
  `Surface`-style bookkeeping: present / last_present / slot / in_tree)
- Modify: `platform/floem-platform/src/capabilities.rs` (`desktop_default` gains
  `NATIVE_VIEW` only once a desktop provider exists — see case D1)
- Test: `app/floem-app/tests/native.rs` (testkit)

**Steps**

1. **Registry + diff (headless).** The host tracks `participants: Vec<Participant>`
   (`node`, `slot`, `kind`, `mode`, `last_state`). Each `frame()` computes the
   desired `NativeState` per participant and calls `provider.apply` only when it
   differs. Test: no-op frames issue no commands; moving a node issues exactly
   one `apply`; a node that unmounts issues `destroy` exactly once.
2. **Clipping and visibility (headless).** Effective rect = node box ∩ ancestor
   clips ∩ viewport. Test: a participant in a container that is scrolled out of
   view flips to `visible: false` (not to a moved rect), and one under a modal
   scrim flips invisible too.
3. **Z-order (headless).** `z` follows tree paint order, and a native participant
   always outranks the scene in Overlay mode. Test: two participants, reorder
   them in the tree, expect the `z` values to swap — and expect a scrim overlay
   opened above them to be reported for handling rather than silently ignored
   (D2's "refuse to silently mis-compose").
4. **Hit-test subtraction (headless).** A capturing participant removes its rect
   from the router's reachable area; a passthrough participant does not. Test
   with the testkit: a `tap` inside a capturing participant's rect reaches the
   native view (recorded by the fake provider) and *not* the framework widget
   underneath; inside a passthrough one there is no native hit and the widget
   gets it.
5. **Focus yield (headless).** Focusing a participant blurs the framework
   element and stops IME cursor-area pushes; blurring restores both. Test by
   asserting `provider.commands()` contains `focus` and no `Ime::set_cursor_area`
   arrives while focused.
6. **Virtualized rows (headless).** Wire `NativeViewPool` to the list of visible
   keys so a scrolled row's released slot is reused for the newly visible row,
   with exactly one `destroy`+`create` pair per recycled slot. Test at 1k items
   with a 10-row viewport: live slot count stays ≤ 10, and `generation` bumps
   are observed exactly where recycling happened.
7. **Desktop bridge (device half).** Implement `floem-platform-desktop`'s
   provider: a child `NSView` (macOS) / child `HWND` (Windows) / X11 subwindow or
   a Wayland texture fallback, placed above the surface.
8. Commit the headless half separately from the bridge.

**Done when** steps 1–6 pass headlessly and each device case below has been run
once with the outcome recorded in the device checklist.

**Device cases**

| id | platform | setup | expected | falsifier |
| --- | --- | --- | --- | --- |
| D1 | macOS (Apple Silicon) | vello surface + one native `NSView` participant, window resized and dragged between displays | view stays pixel-aligned to its node at 1x and 2x, no seam, no smearing on resize | a 1px gap or a lagging view while dragging means the rect is being applied before layout or in the wrong scale |
| D2 | macOS | a framework `Dialog` (R2 overlay) opened above a native participant | dialog paints over the native view, or the participant is hidden while the modal is up | native view drawn above the modal = Overlay mode chosen where the host should have downgraded (D2 decision) |
| D3 | macOS | native `NSTextField` participant focused, then framework `TextField` focused | candidate window follows the *focused* owner only; no dual caret | candidate window jumping between owners = focus/IME yield is not exclusive (D4) |
| D4 | Windows 11 | child `HWND` participant over the GPU surface | participant visible and correctly clipped at 150% scaling | view invisible or bleeding through = airspace limitation; record which mode had to be used |
| D5 | Linux Wayland (GNOME) | participant requested on a Wayland session | provider reports no Overlay mode; host falls back to Texture/Offscreen and the app still renders | a black hole where the view should be = negotiation (D1) missing or wrong |
| D6 | Linux X11 | participant on X11 | subwindow placed above the surface, correct clipping under a scrolling container | view floats over scrolled-away content = clip (D3) computed host-side only |

---

## Task 16.3 — Texture / Surface composition *(mostly device-gated)*

**Files**

- Modify: `render/floem-renderer-vello/src/gpu.rs` (import an external texture as
  a scene resource), `render/floem-renderer` (an `import_texture`-shaped hook)
- Modify: platform providers that can expose a surface/texture
- Test: capability-negotiation tests (headless), plus the device cases below

**Steps**

1. **Capability negotiation (headless).** A participant asks for the strongest
   mode it needs; the host picks the strongest the provider offers; when none
   suffices, it is a typed error surfaced at mount time rather than a blank
   region at paint time. Test the matrix (4 modes × 3 capability sets) against
   the fake provider.
2. **Texture import path.** A provider hands the renderer a texture handle
   (or a shared-surface descriptor); the renderer draws it as a scene resource
   so the participant composites *inside* the scene and can be occluded,
   transformed and clipped normally. On the GPU path this is the same `Image`
   node machinery the image work already built — the new part is that the
   resource's contents are owned by another process/API and must be
   re-imported (not re-uploaded) per frame.
3. **Surface compositor path** on backends with real planes (KMS) or native
   layering (Android `SurfaceControl`, iOS `CALayer`).
4. **Benchmark** copy/latency versus Overlay: bytes per frame and added frame
   time. This is the number that decides whether Texture mode is a fallback or a
   default.
5. Commit per platform group.

**Device cases**

| id | platform | setup | expected | falsifier |
| --- | --- | --- | --- | --- |
| D7 | Android (API 30+) | video-like participant in a scrolling list, 60 fps content | no tearing, no surface bleeding through the UI layer, stable frame time | participant drawn above the toolbar = z-order from the OS surface, not the framework's |
| D8 | iOS | same, `UIView` participant inside a transformed/scaled ancestor | content follows the transform, no blurring from a stale texture size | content frozen at the pre-transform size = texture not re-imported on transform |
| D9 | embedded (KMS/DRM, if available) | participant assigned a hardware plane | plane composition active, no extra full-frame copy | plane unused (visible in `modetest`-style output) = capability not negotiated |
| D10 | macOS | texture-mode participant | can be occluded by framework content (the thing Overlay mode cannot do) | cannot be occluded = still in Overlay mode |

---

## Task 16.4 — Snapshot / offscreen transitions *(headless half + device half)*

**Files**

- Modify: `native/floem-native` (snapshot contract), `framework/floem-animation`
  (animate a snapshot texture), `app/floem-app` (transition orchestration)
- Test: `app/floem-app/tests/native_transition.rs`

**Steps**

1. **Snapshot contract (headless).** Freeze → the participant reports
   `snapshot()` as a texture resource → the host swaps it for the live view for
   the duration of a transition → `restore()` returns ownership exactly once.
   The fake provider covers the state machine; R3's `settle` covers the timing.
2. **Animate the frozen texture.** Reuse the R2 compositor path (`CompositorProp`
   folds opacity/translate into scene nodes) rather than adding a second
   animation mechanism — a snapshot is just another `Image` node with a
   composite prop.
3. Device case: a transition that moves a native participant while the OS view is
   frozen must not flicker or show a stale size.

**Device cases**

| id | platform | setup | expected | falsifier |
| --- | --- | --- | --- | --- |
| D11 | macOS | push a screen containing a native participant, animated | the pushed screen animates as a frozen texture, then goes live at the end | the participant is missing during the animation, or live-and-jumping |
| D12 | Android | same, on a low-end device | frame time stays under budget during the transition | dropped frames = snapshot copy is per-frame instead of once |

---

## Cross-cutting rules (apply to every task)

- **DPR rounding.** Rectangles cross the logical→device boundary exactly once,
  in the provider, from a `scale` the host passes down (`SurfaceTarget` already
  carries it). Two roundings (host and provider) produce the classic half-pixel
  seam.
- **Damage and the frame clock.** A participant's `apply` may change pixels
  outside the scene's damage. The host therefore marks `needs_present` when any
  participant state changed, so the GPU path's skip logic (`present_skips`) never
  suppresses a frame the compositor owes. This is testable headlessly against the
  existing `GpuFrameStats`/present-skip assertions.
- **No third mechanism.** Participants reuse the overlay stack's idioms
  (deferred command queue drained after the update pass, `Surface`-style private
  bookkeeping, `last_present` sentinel). A second, parallel lifecycle is the
  likeliest way to ship a stuck native view.
- **Every provider call is idempotent.** The host calls `apply` with the desired
  state, not a delta, so a missed frame self-heals.

## Sequencing

1. 16.1 (headless) — no prerequisites.
2. 16.4 steps 1–2 (headless) — depends on 16.1 only; do it before 16.3 because it
   unblocks animated transitions, which is user-visible value.
3. 16.2 steps 1–6 (headless) — depends on 16.1.
4. 16.2 step 7 + 16.3 — device-gated, per platform, in one pass.
5. 16.3 step 4 benchmarks last: they only matter once a real Texture path exists.

## Out of scope

- Phase 17 providers (web/video/camera/map) — they *consume* this ABI, and
  building them first would fix the ABI by accident.
- Phase 13's two unchecked steps (native text-input / WebView bridging) are the
  same seam as D3; they should be re-scoped to 16.2 rather than implemented
  twice.

## Risks

| risk | impact | mitigation |
| --- | --- | --- |
| Airspace (Windows) makes Overlay mode unusable for the flagship case | the mode users expect is never available | 16.3's texture path exists precisely for this; D4 is the case that decides which becomes the default on Windows |
| Wayland has no subwindows | Overlay mode needs a fallback on the most common Linux desktop | D5 is run in the *first* device pass, not the last, so the fallback is not discovered late |
| IME ownership (D4) conflicts with R1's bridge | visible candidate-window bugs on hybrid UIs | the yield is a single host-side switch with a headless test |
| The phase is abandoned after the headless halves | device cases rot, "supported" becomes a claim | this document's case table is the artefact; the ledger links it as the phase's completion criterion |
