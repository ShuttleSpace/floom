# Plan: Phase 16 — native composition (R4)

- **Date:** 2026-09-15
- **Plan:** `docs/superpowers/plans/2026-09-02-floom-2-complete-implementation-plan.md` (§Phase 16)
- **Ledger:** `.superpowers/sdd/progress-floom-2.md` (Roadmap R4)
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
| `NativeViewId` | `crates/floom-types/src/ids.rs` | done |
| `NativeSlot` (id + generation), `NativeViewPool<K>` | `crates/floom-native/src/pool.rs` | done (Phase 12.2 precursor) |
| `Capability::NATIVE_VIEW` | `crates/floom-platform/src/capabilities.rs` | flag exists (`1 << 10`); set by `web_default` only, **not** by `desktop_default` |
| `SurfaceTarget` (raw-window-handle pair, size, scale) | `crates/floom-platform/src/surface.rs` | done |
| Scene nodes | `crates/floom-scene/src/node.rs` | `Group`/`Rect`/`Image`/`Text` — **no hole/participant variant** |
| `Renderer` trait | `crates/floom-renderer/src/lib.rs` | `begin_frame`/`render_scene`/`render_scene_with_resources`/`end_frame`/`trim_caches` — **no native/participant hook** |
| Host paint, overlay stack, frame clock, damage | `crates/floom-app/src/lib.rs` | done (R2) |
| Headless harness | `crates/floom-testkit` | done (R3) — `TestApp`, `Selector`, actors, `settle` |

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

**D5 — `floom-native` stays a leaf.** ABI traits and the participant registry go
in `crates/floom-native` (extending `lib.rs` as the Phase 12.2 decision
promised), gaining `floom-overlay`-style nothing: the crate keeps depending only
on `floom-types` + `floom-virtual` + `floom-scene`-free basics. Host-side
composition lives in `crates/floom-app`; OS bridges live in the platform crates.
This keeps `check-architecture-deps.sh` green and keeps the ABI usable by a
platform backend without dragging in the framework.

---

## Task 16.1 — Define the Native ABI *(headless, shippable now)*

**Status: done, `d25bf07`** — 30 tests. One deviation: the three shared enums
ended up in `crates/floom-types/src/native.rs` rather than in `abi.rs`,
because a widget at L6 asks for a participant and a provider at L4 implements
one, and neither crate may depend on the other's.

**Files**

- Modify: `crates/floom-native/src/lib.rs`, `crates/floom-native/Cargo.toml`
- Create: `crates/floom-native/src/abi.rs`, `crates/floom-native/src/fake.rs`
- Test: `crates/floom-native/tests/contract.rs`

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
they depend on). `crates/floom-app/tests/native.rs`, 16 tests. Deviations and
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
- **Settled (`6d0b4a6`): the clip.** `NativeState` grew a `clip: Option<Rect>`
  and `rect` stopped being intersected with anything. Placement and clipping are
  two statements: a provider handed the intersection would *rescale* its content
  to fit, which looks plausible on a map and obviously wrong on a video. Cropping
  became a declared capability (`ClipSupport::{Platform,Provider,Unsupported}`),
  and a provider that cannot crop is moved by the host into a mode whose pixels
  the framework owns — where the crop is free — or hidden with a counted
  diagnostic. The controller refuses to *show* a cropped rect to a provider that
  cannot crop, so no backend can opt out by forgetting.
- That same work found a second bug: **an off-screen participant kept its hit
  region**, so a surface clipped away by a container went on swallowing clicks
  meant for whatever was painted where it used to be. Withdrawing the *native*
  claim is not enough — the node is still a widget, and a deep node's region
  outranks a shallow one's. Off-screen participants now have no region at all.
- **Step 7's macOS half is done (`6829445` for the translation layer, then the
  AppKit host).** `floom-platform-desktop/src/native.rs` holds the
  `NativeState` → `ViewPlacement` translation and the `NativeViewProvider` impl
  over a `NativeViewHost` trait; `native_appkit.rs` holds the AppKit
  `NativeViewHost`, and `WinitPlatform::native_view_provider` builds one from
  winit's `RawWindowHandle::AppKit`'s `ns_view`. What is left of step 7 is the
  Win32 and
  X11 trait impls.

**Step 7: what landed, and what is left.** The bridge is the only part of 16.2
that needs an OS, and it split into a translation layer that needs no window and
six OS calls behind a trait. Both halves are now in for macOS, in three commits
that are worth reading in order, because the second and third changed what the
first assumed:

1. **Done.** `floom-platform-desktop` gains `floom-native` (same layer, L4, so
   the architecture gate permits it; verified the gate *sees* the edge by moving
   `floom-native` to L5, which fails `UPWARD floom_platform_desktop(L4) →
   floom_native(L5)`). `DesktopNativeBridge` implements `NativeViewProvider`
   over a `NativeViewHost` trait.
2. **Done, and the origin turned out not to be a platform fact.**
   `to_view_placement` deliberately does *not* mirror: it returns the framework's
   own top-left device rects, and `os_frame(rect, parent, scale, origin)` is the
   one place the convention is applied. Two reasons, and the second is the one
   that would have been missed:
   - `ViewPlacement` is what a backend consumes, and a top-left backend (Win32,
     X11) should place it unread. Mirroring in the shared layer would have handed
     every future backend AppKit's convention and made the module's own docs
     false.
   - `NSView::isFlipped` is **per-view**, and winit's content view already
     overrides it to `true`. So on a floom 2.0 window the correct answer is
     `Origin::TopLeft` and no flip happens at all; on a bare `NSView` it is
     `Origin::BottomLeft`. The AppKit host asks the parent (`parent.isFlipped()`)
     rather than assuming, and a backend that hardcoded "macOS means bottom-left"
     would place every participant mirrored about the parent's centre — a bug
     that only shows up under a non-winit parent.
   The mirror itself still has to go **after** `to_device`, not before, and
   `os_frame` only accepts a `DeviceRect` so that the wrong order is unreachable
   rather than merely discouraged. A test pins the case at both scales; D1's
   assertion is the involution that falls out of it, `mirror_device(frame,
   parent_device_height) == state.device_rect()`, which is checkable without
   eyes once a window exists.
3. **Done, and this is where the crop question was finally cashed in.** The
   `clip` channel from the 16.2 rework is not a rect the OS is handed; a crop of
   a view you do not own has to be a *second* view. `split_placement` turns
   `{frame, crop}` into a container (the crop) and a content rect (the participant
   offset inside it), and the AppKit host makes the container `isFlipped` so the
   content's frame is a plain subtraction in the framework's own convention.
   That is what makes `ClipSupport::Platform` true rather than aspirational, and
   it is why a clipped participant on macOS is cropped rather than hidden. Two
   alternatives were rejected on the way: shrinking the participant's frame
   rescales its content (the D6 bug), and moving its `bounds` origin scrolls it
   (which is a crop of the *page*, not of the view, and destroys the
   participant's own scroll position).
4. **Done, and the flag has exactly one source.** `WinitPlatform::capabilities`
   answers with the bridge's own `platform_capabilities()` once
   `native_view_provider` has built one, and with `desktop_capabilities(false)`
   before that. The flag stays a promise in both directions — advertising it with
   no provider fails every `NativeSurface` mount with "no mode this participant
   accepts", and installing a provider without it makes widgets refuse to try at
   all — so it is derived in one place and stored, never set by hand. The
   remaining gap is honest and device-side: a caller who builds a provider and
   throws it away leaves the flag on, which no amount of local bookkeeping fixes
   without threading the app's host into the platform.

The input mode had to travel with the placement rather than in a call of its own.
On AppKit, "does this participant take the pointer" *is* part of how it is
placed: the container answers `hitTest:` with `nil` when it does not, so the
click reaches the framework's surface underneath. Folding it into
`ViewPlacement` also puts it under the bridge's existing memo for free, which a
separate `set_view_input` call would not be — that one would have to fire on the
first placement as well, and the first placement is the one that is easiest to
miss.

Three things the translation layer had to decide, each with a test that fails if
it is undone, and each of which the recipe did not anticipate:

- The memo that makes a repeated state free is keyed on the state **and the
  parent extent**. A resize moves a top-left-pinned participant whose rect did
  not change, so a memo keyed on the state alone swallows the re-placement the
  resize made necessary.
- `destroy` drops the memo and `create` need not. `rebind` destroys and
  re-creates *the same slot at the same generation*, so only `destroy` is on
  that path; a pool-recycled slot is a different key anyway. Keeping the memo
  across a rebind suppresses the replacement's first placement — and because the
  state is unchanged, the suppression is total.
- A hidden participant is hidden where it stands, not moved: the rect in a
  hidden state is not meaningful, and a hidden view with a nonsensical frame is
  a rejected `SetWindowPos` on some platforms and a flash at the old position on
  others.

D1–D3 become checkable without eyes through that first item: ask the provider
for the `NSView`'s frame and compare it to `state.device_rect()`.

**The first real window found the bug that no headless test could.** An `NSView`
frame is in *points*, so `os_frame` divides the device rect by the window's
`backingScaleFactor` — and on a 2x display the probe's view came out exactly half
sized while the framework's own content was drawn correctly. The AppKit host was
not the problem: the density it divided by was right, and the device rect it was
handed had never been scaled, because `AppRunner::handle` read each window's
scale factor at creation and handed the host only the size. `WidgetHost::scale`
stayed at its 1.0 default, and `to_device` — the one place logical becomes
device — multiplied by one. Fixed in the runner on both paths. The second path is
the one worth naming: the ABI has **no scale event**, so a window dragged to a
display of a different density announces itself as a plain `WindowResized`, and
the scale has to be re-read off the window rather than taken from the record
(which only ever saw the display the window was born on). Two tests pin it — the
runner keeps the density the window was born with, and a participant's
`device_rect` doubles while its `rect` does not. A device-side claim this strong
is worth repeating: *the framework lays out in logical units and places
participants in device pixels, so every path that changes the density must reach
the host.*

**Files**

- Modify: `crates/floom-app/src/lib.rs` (participant registry, frame diff, paint
  order, hit-test subtraction, focus yield)
- Modify: `crates/floom-core` (`UpdateContext`/`EventContext` native door)
- Modify: `crates/floom-widgets` (a `NativeSurface` composite widget holding
  `Surface`-style bookkeeping: present / last_present / slot / in_tree)
- Modify: `crates/floom-platform/src/capabilities.rs` (adds
  `desktop_capabilities(native_view)`; `desktop_default` itself must **not** gain
  `NATIVE_VIEW` — see case D1)
- Add: `crates/floom-platform-desktop/src/native.rs` (`6829445` — the
  `NativeState` → `ViewPlacement` translation, `NativeViewHost`,
  `DesktopNativeBridge`)
- Add: `crates/floom-platform-desktop/src/native_appkit.rs` (the AppKit
  `NativeViewHost`: `Container` + `MacNativeViewHost` + `NativeParent`, macOS only)
- Modify: `crates/floom-platform-desktop/src/winit_backend.rs`
  (`native_view_provider`, `native_view_parent`, the stored `native_caps`)
- Modify: `crates/floom-app/src/runner.rs` (push the window's scale factor to the
  host on creation *and* on the resize a display change arrives as)
- Modify: `crates/floom-platform/src/fake.rs` (the fake window carries a scale,
  so a Retina boot is testable without a Retina display)
- Modify: `crates/floom-testkit` (`TestApp::window`)
- Test: `crates/floom-app/tests/native.rs` (testkit)
- Test: `crates/floom-platform-desktop/tests/winit_capabilities.rs` (a session
  that never built a bridge does not advertise `NATIVE_VIEW`)
- Example: `examples/floom2-probe` (installs the provider over a real window and
  dumps both the OS subview tree and the framework's participant states — the
  device half of D1–D3, run by hand)

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
7. **Desktop bridge.** The translation layer is in (`6829445`); what is left is
   the `NativeViewHost` impls: a child `NSView` (macOS) / child `HWND` (Windows)
   / X11 subwindow or a Wayland texture fallback, placed above the surface.
8. Commit the headless half separately from the bridge. Done both times:
   `dab2547` for the host side, `6829445` for step 7's translation layer.

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

**Status: steps 1 and 2 done.** `tests/negotiation.rs` (7 tests) plus
`floom-renderer/src/external.rs` (10 tests) and four GPU tests in
`gpu.rs`. Findings worth carrying forward:

- **vello 0.7 cannot import a GPU texture.** `peniko::ImageData` is `Blob<u8>`
  and nothing else — there is no `External` variant — so a backend cannot wrap
  an object another process owns. What the framework *can* do, and what texture
  mode actually needs, is accept a fresh pixel buffer per produced frame and
  re-upload only when the producer says the contents changed. That is what
  landed; `ExternalFrame::pixels: Option<Arc<[u8]>>` marks the boundary, where
  `None` is the backend-imported case that is currently unreachable rather than
  an incomplete frame.
- **A refreshed texture must invalidate the fragment that drew it.** Vello's
  scene is a command list, so the per-node fragment cache (R5) bakes in the
  brush it was built from. Re-importing pixels under an unchanged *node* is
  precisely the case that cache gets wrong, and the failure looks like the
  "frozen video" of D8 arriving through the fragment cache instead of the
  texture cache. `import_texture` drops the fragments whose `image_keys`
  mention the resource; the test fails without that line.
- **Liveness has two independent sources.** A texture stays resident if its
  producer produced a frame *or* the scene still draws it. Either alone is
  wrong: a participant's node can be created a pass after its first frame, and
  a paused stream is still on screen. `ExternalTextureCache::prune` is therefore
  driven from the fragment table, and runs on *skipped* frames too, because a
  producer going quiet is not scene damage.
- `texture_cache_hits` counts an external texture as a hit: the upload already
  happened at `import_texture`, so the encode is a lookup.

**Still open:** step 3 (surface compositor on KMS/Android/iOS), a device-gated
task this machine cannot do, and the *frame-time* half of step 4 (needs a
device). The *bytes* half of step 4 is DONE: `texture_mode_moves_an_exact_set_of_bytes_per_produced_frame`
(`crates/floom-renderer/src/external.rs`) pins the per-frame cost model — one
full frame per produced frame, nothing for silence. `external_stats().bytes_uploaded`
counts only what actually crossed.

**Files**

- Modify: `crates/floom-renderer-vello/src/gpu.rs` (import an external texture as
  a scene resource), `crates/floom-renderer` (an `import_texture`-shaped hook)
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

**Status: steps 1 and 2 done (`12757bf`, `c80da2e`, `4d0ab28`).** The plan's
own sequencing put 16.4 ahead of 16.3 on the grounds that it unblocks animated
transitions, and that turned out to be the right call for a second reason: it
is the only remaining task whose *device* case (D11) is runnable on the machine
the work is being done on.

- The frozen texture is painted by the widget that owns the box, so
  `NativeSnapshot` had to move to `floom-types` — a fourth shared noun, which
  16.1 did not anticipate because a frozen participant is still the same
  participant.
- Freezing hides the live view rather than destroying it: a transition's end
  hands it back, and re-creating an OS view is the expensive thing.
- The animation is the overlay's own: `AnimationScheduler` sampled into a
  `CompositorProp`, handed to `paint_node`, folded into the image node. A
  finished transition thaws itself.
- `Ok(None)` from a provider that cannot snapshot is not an error, and starts
  nothing: fading a texture that could not be taken is not a thing to attempt.

**Files**

- Modify: `crates/floom-native` (snapshot contract), `crates/floom-animation`
  (animate a snapshot texture), `crates/floom-app` (transition orchestration)
- Test: `crates/floom-app/tests/native_transition.rs`

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
