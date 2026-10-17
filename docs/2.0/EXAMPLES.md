# Golden examples (Floom 2.0)

Every example below is a runnable binary. Unless a row says otherwise it runs
**headless** — a `WidgetHost` pumped by hand — so it can be verified in CI and
on this machine. Each prints a short narrative and **asserts** the invariant it
demonstrates, so an example that stops being true fails rather than printing
something stale.

`cargo run -p <name>` runs one; `cargo build --workspace` compiles all of them.

## The list

| example | topic | target | status |
| --- | --- | --- | --- |
| `floom-demo` | the demo gallery: every section (primitives, controls, composites, motion, navigation, data, forms, styling) in one app — the same tree runs headless, in a real window, and on Android | all / desktop / Android | headless (default); `--features macos` opens a window, `--features android` produces the APK |
| `floom-capabilities` | the platform surface, not the widget one: 12 capability readings (device identity, battery, connectivity, sensors, permissions, safe area, soft keyboard, clipboard read/write, haptics) over all 13 packages `floom-platform-mobile` declares, in one keyed table that prints `—` where the OS has no answer, plus 8 side-effecting actions behind buttons. App/screen/orientation/accessibility/URL-launcher readings are **absent on purpose** — no such package exists, and `docs/platform-and-plugins.md` puts them behind not-yet-written plugins | all / desktop / Android | headless (default); `--features macos` opens a window, `--features android` produces the APK |
| `floom-chart` | self-drawn content: a line chart on a `Canvas`, clipped to its own box, redrawn from data | all | headless |
| `floom-hello` | the smallest app: everything from `floom::prelude` | all | headless |
| `floom-run-desktop` | the same tree in a real OS window through the facade's one-call `floom::run_desktop` — no `WinitApp` impl, no `AppRunner` wiring in the example | macOS / desktop | **display-gated** — `--features desktop,renderer-gpu`; see the known text-ink defect in [`vello-gpu-text-ink-defect.md`](./vello-gpu-text-ink-defect.md) |
| `floom-counter` | composites (`Stack`/`List`/`Grid`/`Tabs`/`Dialog`/`NavBar`) + reconciliation | all | headless |
| `floom-list` | a keyed list: append / remove / reorder without rebuilding rows | all | headless |
| `floom-virtual-list` | virtual scrolling: bounded element pool over 100 000 items | all | headless |
| `floom-datagrid` | data grid: virtualized rows, sorting, selection, edit + undo | all | headless |
| `floom-forms-demo` | form validation, dirty/touched tracking, control bindings | all | headless |
| `floom-navigation-demo` | typed routes, history, page transitions, nav containers, deep links | all | headless |
| `floom-progress-ring` | self-drawn motion: an animated ring + a `Spinner` that animates itself | all | headless |
| `floom-transform` | subtree transforms: a card that scales and rotates as one picture | all | headless |
| `floom-style-demo` | style that is made of something: a gradient card, a real drop shadow, an implicit hover fade armed with a design-system motion token, a blurred card whose ink crosses its own edge, and a frosted panel that greys the edge behind it | all | headless |
| `floom-shader` | custom WGSL shaders: a box whose ink is a program — the source and the caller's scalars reach the scene over the element's own box, and a CPU rasteriser counts the skip instead of faking ink | all | headless |
| `floom-editor-demo` | text document: transactions, selection, undo/redo, IME pre-edit | all | headless |
| `floom-motion` | eased tweens, interruption, a control that animates itself, presence enter/exit, a crossfade that keeps both sides alive, an `AnimatedContent` that keeps both sides alive by key, an `Opacity` that fades a subtree instead of snapping, and a `SharedTransitionLayout` that flies one subtree into the other's box | all | headless |
| `floom-native-plugin` | writing a native provider plugin + slot pooling | all | headless |
| `floom-probe` | device probe: one window holding every control the on-screen cases need | desktop | **display-gated** — `--features real`, driven by hand |
| `floom-window` | a real OS window: winit + vello GPU | desktop | **verified on macOS** |
| `floom-macos` | the macOS gate probe: a clickable tree, a text field an input method can type into, and a clipboard whose far end is the system pasteboard | macOS | **display-gated** — `--features real`, driven by hand (see `docs/gates/macos.md`) |
| `floom-android` | a real Android app: `native-activity` + vello GPU in one APK | Android | **device-gated** — `--features real`, packaged by `examples/floom-android/scripts/build-apk.sh` |

The device-, display- and target-gated rows are the same class of gap as phases
17 and 19: they need OS providers, a display or hardware this machine does not
have. Everything else here is exercised on every change:
`bash scripts/run-golden-examples.sh` *runs* each headless example (compiling
them proves nothing — their assertions are the point), and
`crates/floom/tests/docs_parity.rs` asserts that this table, the `examples/`
directory and that script's list all agree, so a new example cannot land
undocumented and a documented status cannot quietly become false.

## One command per platform

`just build <crate> <platform>` (i.e. `scripts/build.sh`) is the single entry
point: Android delegates to `scripts/build-apk.sh`, iOS to
`scripts/ios-bundle.sh`, OHOS to the crate's hvigor Stage project, and each run
ends with one honest result line —

```text
TIER=<artifact|check-only|blocked> PLATFORM=<p> CRATE=<c> ARTIFACT=<path|->
```

A platform whose backend cannot render, or whose toolchain is missing, is
reported as `check-only`/`blocked`, never as `artifact`.

Two tracks feed the matrix. **v2** is the `floom-*` demos; `floom-demo`
carries one feature per platform. **v1** is the older single-crate stack, and it
is the one with *real* iOS / OHOS / Web backends today (`crates/floom/src/embedder/*`,
`crates/floom/src/platform/*`) — which is why those three platforms are covered by the v1
examples. A v2 artifact on iOS/OHOS/Web waits on those v2 backends (Phase 19),
not on this build plumbing.

| crate (`just build` name) | platform | tier | what comes out |
| --- | --- | --- | --- |
| floom-demo | macos | artifact | `target/debug/floom-demo` — a real winit + vello window |
| floom-demo | android | artifact | `target/android/…/*.apk` — `native-activity` + vello |
| floom-demo | ios / ohos / web | check-only | the v2 backends for these targets are still in-memory skeletons |
| ios (`floom-ios-example`) | ios | artifact | `target/ios/floom-ios-example.app`, ad-hoc signed for the simulator |
| ohos | ohos | artifact | `examples/ohos/ohos-app/…/entry-default-unsigned.hap` (unsigned) |
| webgpu | web | artifact | `target/web/webgpu/` — `webgpu_bg.wasm` + `webgpu.js`, the `wasm-bindgen --target web` bundle (tier drops to `check-only` with a raw `.wasm` when `wasm-bindgen-cli` is not installed — its version must match the `wasm-bindgen` crate) |
| any | windows / linux | check-only | a `cargo check` once `rustup target add` has run |
| any | tui | rendering | no terminal **platform** yet: `floom-renderer-tui` paints any scene into a character grid and encodes it as ANSI (see "Terminal output" below), but nothing yet sizes that grid from a real terminal or feeds stdin events back |

Three toolchain facts are encoded in the scripts because each one costs a build
to rediscover: the OHOS path needs both `DEVECO_SDK_HOME` exported and
`OHOS_BASE_SDK_HOME` pointed at *that same directory* (hvigor rejects the
`command-line-tools/sdk/default` layout with "The SDK management mode has
changed"), the OHOS NDK clang must be the linker — the host `cc` rejects the
GNU-style flags rustc emits for `aarch64-unknown-linux-ohos` — and the wasm32
link needs `DYLD_FALLBACK_LIBRARY_PATH` pointed at the toolchain `lib/`, because
rustup's `rust-lld` resolves `libLLVM.dylib` under `lib/rustlib/<host>/lib` where
it does not exist (a bare `cargo build --target wasm32-unknown-unknown` or
`trunk` build fails with "Library not loaded: @rpath/libLLVM.dylib" until the
path is set or a symlink is added).

## Notes per example

### `floom-demo`

The gallery the examples are converging on: one crate holds the whole thing and
the platform is a feature, so a target build is one command away.

```sh
cargo run -p floom-demo              # headless: every demo mounted and painted
just build floom-demo macos          # a real winit window (vello over wgpu)
just build floom-demo android        # an installable APK
```

The tree is built once (`demos::gallery_root`) and mounted by three backends
that differ only in how frames are driven — a hand-pumped `WidgetHost`, the
desktop winit loop, and the Android `native-activity` loop — so a demo that
renders in CI is the same demo that renders on screen. The headless run asserts
each demo painted at least one widget, which is what makes it a golden example
rather than a screenshot.

**Platform matrix** (`just build floom-demo <platform>`):

| platform | tier | why |
| --- | --- | --- |
| macos | artifact | the desktop winit + vello backend is real |
| android | artifact | the Android winit + vello backend is real |
| ios / ohos / web | check-only | the v2 backends for these targets are still in-memory skeletons; the features wire the backend crate in so the target can be `cargo check`ed, but no artifact is claimed |

### `floom-hello`

The whole app comes from one glob import — the Phase 29.1 surface contract.

```text
hello floom 2.0: mounted and painted 2 widgets
```

Release binary (macOS, Apple M4 Pro, unstripped): **2,694,256 B ≈ 2.6 MiB** —
the floor of the size table, because it pulls the headless core only.

### `floom-chart`

Self-drawn content (MOTION-GAP A1/A2/A7/A8 + the repaint bridge): a `Canvas`
whose painter draws a line chart — axes, a translucent area fill, a stroked
data line — plus the three things a drawn surface needs besides pixels:

* **The clip is a declaration, and a widget's clip is the widget's own box.**
  The canvas declares `clip(ClipShape::Box)`; the host brackets exactly its
  subtree with `ClipPush`/`ClipPop` in the retained scene. The canvas sits
  24px down the surface and the data peaks above the box's top edge, so a
  software render proves the bracket *cuts*: the band of surface above the
  canvas stays untouched while the plot below it paints.
* **The surface says what it is.** The canvas publishes `Role::Image` with a
  label, so assistive tech sees a chart, not silence.
* **Data is made geometry, and the ping carries it.** The painter reads a
  reactive value; a value change alone wakes nothing, the repaint ping wakes
  the loop, and the drawn vertex moves to exactly where the new value says —
  still cut off by the clip, which is a property of the surface, not of one
  dataset.

```text
== 1. the closure reaches the scene ==
  4 path nodes: 1 with a fill, 3 with a stroke
== 2. the clip is a bracket the host writes ==
  1 ClipPush / 1 ClipPop in the retained scene
  the band above the canvas is untouched; the plot itself is not
== 3. the canvas says what it is ==
  role: Image, label: Some("Quarterly revenue")
== 4. data made geometry ==
  third vertex: [133,109] -> [133,2]
  the ping drew the new data; the clip cut it at the same edge
```

### `floom-progress-ring`

Self-drawn motion (MOTION-GAP A7 + the B3 loop): the host animates a `Canvas`
ring by running a tween whose value the painter turns into arc geometry — the
arc's scene node measurably changes shape between frames — and a `Spinner`
animates itself forever by declaring an endless cycle. Also exercised: the
repaint channel, where bumping a `RwSignal` ping wakes the host for a
paint-only repaint with no relayout.

```text
== 1. the host drives a drawn shape ==
  animation value: 0.167 -> 1.000
  arc box: [48,6 .. 84,27] -> [6,6 .. 90,90]
  a finished tween parks the frame loop
== 3. a signal repaints the canvas ==
  painted 2 widgets on the wake frame
== 2. a control that animates itself ==
  running animations nobody asked for: 1
  sweep: 0.017 -> 0.183 after 10 frames
```

### `floom-transform`

Subtree transforms (MOTION-GAP A6/B7): a transform-class composite — scale,
rotate — becomes a `GroupPush` scope bracketing the animated element's picture,
and every backend applies one transform. The card's own payloads never move;
the scope carries the sampled value about the box centre, pivoting where
"scale the whole thing" actually means.

```text
== 1. the scope brackets the picture ==
  sampled scale 0.667 about the box centre
== 2. the pixels follow the scale ==
== 3. a rotation turns the whole picture ==
== 4. a finished transform parks clean ==
```

### `floom-style`

Style realised in primitives (MOTION-GAP A3/A4/B4/B6): a gradient background,
a blurred drop shadow instead of the pseudo-shadow rect v1 faked elevation
with, an implicit transition the author never has to start, Material's own
motion tokens as its timing, and — A4's back half — a *filter*: a card drawn
through a blur, and a frosted panel whose input is the picture behind it. Every
act is asserted — including the control that makes each positive claim mean
something: a flat background really does put the same pixel at both ends, an
unraised card really does leave the pixels below it empty, an untransitioned
button really does snap in one frame, an unfiltered card really does end where
it was drawn, and the frost's own edge really does stay sharp outside the
panel.

```text
== 1. a gradient card is not one colour ==
  left [37, 60, 213, 255]  right [214, 60, 36, 255]
  the ramp reached the renderer intact; the flat card stayed flat
== 2. elevation casts a shadow ==
  below the card: [0, 0, 0, 167]
  below: inked; above and the unraised control: empty
== 3. the hover fade is implicit ==
  one frame after hover: faded #2B6CB0FF, snapped #5186BEFF
  both arrive at #5186BEFF; only one of them travelled
== 4. the design token is the transition ==
  MotionDuration::SHORT2 = 100 ms -> 0.100 s
  armed with MotionTransition::STATE_CHANGE, and it takes that long
== 5. a filter is not a shadow ==
  3px outside the card: [255, 255, 255, 41]   6px outside: [255, 255, 255, 2]
  the ink crossed the edge and thinned; the unfiltered card did not
== 6. frost reads what is behind it ==
  inside the panel, 2px from the edge: [184, 184, 184, 255]
  the edge went grey inside the panel and stayed sharp outside it
```

Act 5's pair is what separates a filter from the shadow in act 2: the ink is
*outside* the card's box and thinning with distance, and the unfiltered card
puts nothing there. Act 6's pair is what separates a backdrop filter from a
content one — both blur, so the only discriminating probe is the same hard
edge read inside the panel (grey) and outside it (still a step).

### `floom-shader`

A5: the one capability in this table whose ink is *code*. Three acts, all
asserted:

1. a `ShaderBox` mounted like any other widget puts one shader node over its
   own laid-out box, carrying the WGSL and the scalars untouched — the box is
   the program's whole output, so nothing is filled behind it;
2. presented to the software renderer the same scene leaves the box exactly as
   the backdrop painted it **and counts one skip** — a CPU rasteriser cannot
   run WGSL, and an empty box is indistinguishable from a shader that drew
   nothing, so the degradation has to be reported rather than merely be;
3. re-pointing the scalars on the mounted widget moves the numbers and leaves
   the box where it was — `u.time` already animates a shader for free, so a
   caller pushes only its own inputs.

```text
act 1  a 160x72 box carries 507 bytes of WGSL and 2 scalars, over its own box
act 2  the software renderer drew no ink and counted 1 skip — the box is empty because the program was dropped, not because it ran
act 3  params went [0.0, 1.0] -> [0.5, 0.25] over the same 160x72 box
```

Pixels are the one thing this example deliberately does not show: a shader's
output needs the GPU path (`floom-window` / `floom-probe`, `--features
real`). What vello does with the same node — compile the WGSL, render it
offscreen, composite it back, count `shaders_drawn` — is asserted by
`crates/floom-renderer-vello/src/gpu.rs` under `--features gpu`, and both
halves are covered by `scripts/mutate-a5-shader.sh` (8/8).

### `floom-motion`

Motion, end to end (MOTION-GAP B1/B2/B3/B4/B5/B6): a curve is not a duration,
interruption carries what it interrupted, a control animates itself, a presence
transition delays unmount, a crossfade keeps *both* sides alive, an
`AnimatedContent` keeps both sides alive *by key*, an `Opacity` fades a
subtree instead of snapping it, and a `SharedTransitionLayout` flies one subtree
into the other's box.

```text
== 4. a presence transition delays unmount ==
  enter fades in; exit keeps the child mounted until it finishes, then it unmounts
== 5. a crossfade keeps both sides live ==
  A and B both stay on screen during the swap; A unmounts only when its exit finishes
== 6. an animated content keeps both sides live ==
  v1 -> v2 keeps both mounted; v2 -> v1 (a return) replays the enter from invisible
== 7. an opacity transition fades instead of snapping ==
  1.0 -> 0.5 fades; 0.5 -> 1.0 fades back up from 0.5, not from 1.0
== 8. a shared element flies to its destination ==
  source rests at y=0; arming the flight…
  flew src (y=0) -> dst box (40); revealed 'dst' content
```

Act 4 is the enter/exit half of B5: a subtree leaving the screen stays mounted
until its exit animation has played, because a departure nobody sees is not a
transition. Act 5 is the `AnimatedContent` / `Crossfade` half: both contents are
stacked in one box, so mid-swap the scene holds **two** mounted children — the
leaving one at full opacity, the arriving one at none — and only the arriving one
is left once the swap completes. Act 6 generalises act 5 to a content key:
`AnimatedContent::new(key).content_with(|v| child).presence(transition)` switches
the tracked value and keeps the leaving subtree mounted (exit) while the arriving
one enters; a value that returns mounts a *fresh* node that replays its enter from
invisible rather than resurrecting the one that already finished. Act 7 is the
B4 opacity half: `Opacity::new(child).opacity(1.0).transition(transition)` wraps
a subtree, and changing the target opacity fades the whole subtree through the
compositor opacity channel (S3's fold-class) instead of snapping — a value that
returns fades back up from where it rested, not from where it was born. Act 8 is
the `SharedTransitionLayout` / `Hero` half of B5 plus the single `TranslateY`
channel of B6: `SharedTransitionLayout::new(key, delta).source(src).dest(dst)`
keeps both subtrees mounted — each pinned to its own absolute box — and arming
the flight translates the source's ink down onto the destination's box; once it
lands the destination's *own* content is revealed and the source unmounts. It is
carried on exactly one compositor channel, so redirecting it mid-flight through
the host's retarget door keeps the speed it had (B2).

Also exercised end to end in `crates/floom-snapshot/tests/golden_frames.rs`
(`a_shared_element_flight_is_golden_frame_by_frame`: the ink's top edge walks
down the lattice and the landing frame swaps the inky source card for the hot
destination card, a trajectory no single-pixel probe can hold).

### `floom-window`

The only example that opens a real window: winit + vello over wgpu.

```text
GPU adapter: Apple M4 Pro (Metal)
entry fade: 0.4s ease-out on the label (opacity 0 -> 1)
[idle] waits=4 frames=5 redraws_requested=7 auto_redraws=5 events=7 about_to_wait=4
idle probe after 8s: frames=26 redraws_requested=27 auto_redraws=25 events=28 about_to_wait=50; process CPU: 0.24s (2.9% of one core)
```

Run with `FLOOM2_WINDOW_IDLE=1 FLOOM2_WINDOW_IDLE_SECS=8 cargo run -p
floom-window --features real`. Release binary: **11,626,912 B ≈ 11.1 MiB** —
what the desktop + GPU bundle costs on top of the 2.6 MiB headless floor.

**The load-bearing claim is that `frames` stops climbing, not that it is small.**
Before the motion work landed the number was `frames=2`; the label now fades in
(0.4 s, ease-out, opacity), so a run reads `frames=26` — the mount frame plus the
fade's own 24 — and then freezes there for the remaining ~7.5 s, with
`about_to_wait` at 50 rather than in the thousands. That makes the case *stronger*
than it was: the number is no longer small for the trivial reason that nothing
ever animated, and an endless animation would have kept it climbing (which is why
the fade is one-shot — see `WindowApp::intro_fade`). The trace lines show the
climb frame by frame, so a wedged loop is visible in the output rather than only
in the total. `redraws_requested` and the CPU percentage move a little run to
run (27 / 2.9 % here, 3 / 2.1 % on the pre-motion run) because they count the
startup redraw and whatever the window server sends in the first second.

### `floom-macos`

The desktop counterpart of `floom-android`: it exists to produce the evidence
`docs/gates/macos.md` records rather than to demonstrate anything new about the
stack. Same `AppRunner<VelloGpuPresenter>`, same winit loop as `floom-window` —
what it adds is a text field, an IME request, a focus trace, and a clipboard
write, plus a log line for each, so a spot-check is a read of what the OS
actually did.

```text
GPU adapter: Apple M4 Pro (Metal)
floom-macos: widget tree mounted (4 nodes)
floom-macos: clipboard write=true read_back=Some("floom-macos clipboard probe 3e7b") matched=true
floom-macos: text field focused
floom-macos: button logical=(0.0,92.0 160.0x40.0) scale=1 content_px=x0..160 y92..132
floom-macos: field logical=(0.0,36.0 260.0x40.0)
floom-macos: focus None -> TextField@2
floom-macos: text field text="你好世界" preedit="" composing=false
```

The clipboard line is the load-bearing one: on macOS `Clipboard` is
`NSPasteboard` (see `crates/floom-platform-desktop/src/clipboard.rs`), so the
same string is then readable from a different process entirely — which is what
separates a clipboard from a `String` field. The `logical=` lines are there so a
click can be *aimed*: they are content-relative, so the window's frame and the
32 pt title bar have to be added before they are screen coordinates. The focus
line is what makes item 7 of the gate falsifiable from the outside — a Tab press
either moves focus or leaves a trace with no line for it.

The field is deliberately **preloaded with a non-ASCII string**: the log can
prove characters arrived, but only pixels can show whether they were drawn. That
is exactly what happened — the string rendered as four tofu boxes, and the
finding (parley's per-script fallback resolving Han back to a Latin-only font)
is written up in `docs/gates/macos.md` and now fixed in
`floom-text`'s default font stack. A probe that only logged would have reported a
working CJK path.

### `floom-android`

The mobile analogue of `floom-window`: the same `AppRunner<VelloGpuPresenter>`
widget tree, but pumped by the Android (winit) event loop instead of the desktop
one. `native-activity` loads `libfloom2_android_example.so` and calls
`android_main`, which hands the `AndroidApp` to `floom-platform-android`, mounts
a counter, and runs — the only Android-specific seam is the entry point and the
`AndroidRunner` callbacks that pump the app from winit's push loop. The GPU
surface is built from the Android `ANativeWindow*` by the renderer exactly the
way the desktop window is; no custom surface shim is involved.

The crate is a `cdylib` (a bin-only crate would make `cargo ndk` copy nothing
into `jniLibs/`), so the host build compiles only the placeholder `src/main.rs`.
The Android module is behind the `real` feature, so the cross-check is:

```text
cargo check -p floom-android-example --target aarch64-linux-android --features real
```

which passes with zero warnings from this crate. Keeping the presenter behind
`real` is what lets `cargo test --workspace` stay dependency-light: the default
host build never pulls wgpu.

It is **device-gated**: this machine has no emulator image, so the APK has never
been launched. What is verified here is compile-level (host + `aarch64-linux-android`
+ the architecture gate), not first-frame on a device.

### Removed with v1

The rows below used to advertise example crates that were deleted along with the
v1 stack. They are listed here rather than silently dropped, because "this
existed and does not" is the answer to "where did the iOS example go" and a
missing row does not answer it.

The names below are written without backticks on purpose: the index
table above is recognised by its first column being code, and a second
code-first table would be read as more advertised examples.

| was | what it was | why it is gone |
| --- | --- | --- |
| ios | the iOS app: counter, in-UI context menu, `UIPasteboard` clipboard, soft keyboard | v1 stack |
| ohos | the OpenHarmony app: v1 as a `cdylib` loaded by an ArkTS Stage project | v1 stack |
| webgpu | the browser app: v1 on `wasm32` + WebGPU | v1 stack |
| webview, video, camera | embedded OS-provided views | v1 native-view path |
| harmony | the HarmonyOS build | v1 stack |
| web, embedded | wasm32 / no-std builds of v1 | v1 stack |

Each of these is a target Floom 2.0 still has (`floom-platform-ios`,
`floom-platform-web`, `floom-platform-embedded`, `floom-native` for the embedded
views). What is missing is a *2.0 example* per target, not a backend.
