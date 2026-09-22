# Golden examples (Floem 2.0)

Every example below is a runnable binary. Unless a row says otherwise it runs
**headless** — a `WidgetHost` pumped by hand — so it can be verified in CI and
on this machine. Each prints a short narrative and **asserts** the invariant it
demonstrates, so an example that stops being true fails rather than printing
something stale.

`cargo run -p <name>` runs one; `cargo build --workspace` compiles all of them.

## The list

| example | topic | target | status |
| --- | --- | --- | --- |
| `floem2-chart` | self-drawn content: a line chart on a `Canvas`, clipped to its own box, redrawn from data | all | headless |
| `floem2-hello` | the smallest app: everything from `floem2::prelude` | all | headless |
| `floem2-counter` | composites (`Stack`/`List`/`Grid`/`Tabs`/`Dialog`/`NavBar`) + reconciliation | all | headless |
| `floem2-list` | a keyed list: append / remove / reorder without rebuilding rows | all | headless |
| `floem2-virtual-list` | virtual scrolling: bounded element pool over 100 000 items | all | headless |
| `floem2-datagrid` | data grid: virtualized rows, sorting, selection, edit + undo | all | headless |
| `floem2-forms` | form validation, dirty/touched tracking, control bindings | all | headless |
| `floem2-navigation` | typed routes, history, page transitions, nav containers, deep links | all | headless |
| `floem2-progress-ring` | self-drawn motion: an animated ring + a `Spinner` that animates itself | all | headless |
| `floem2-transform` | subtree transforms: a card that scales and rotates as one picture | all | headless |
| `floem2-style` | style that is made of something: a gradient card, a real drop shadow, and an implicit hover fade armed with a design-system motion token | all | headless |
| `floem2-editor` | text document: transactions, selection, undo/redo, IME pre-edit | all | headless |
| `floem2-motion` | eased tweens, interruption, a control that animates itself, presence enter/exit, and a crossfade that keeps both sides alive | all | headless |
| `floem2-native-plugin` | writing a native provider plugin + slot pooling | all | headless |
| `floem2-probe` | device probe: one window holding every control the on-screen cases need | desktop | **display-gated** — `--features real`, driven by hand |
| `floem2-window` | a real OS window: winit + vello GPU | desktop | **verified on macOS** |
| `floem2-android` | a real Android app: `native-activity` + vello GPU in one APK | Android | **device-gated** — `--features real`, packaged by `examples/floem2-android/scripts/build-apk.sh` |
| `webview` | embedded web view | OS provider | device-gated (Phase 17) |
| `video` | embedded video | OS provider | device-gated (Phase 17) |
| `camera` | camera preview | OS provider | device-gated (Phase 17) |
| `harmony` | HarmonyOS (OpenHarmony) build | OpenHarmony | device-gated (Phase 19) |
| `web` | `wasm32` build | wasm32 | target-gated (Phase 19) |
| `embedded` | no-std / bare-metal build | embedded | target-gated (Phase 19) |

The device-, display- and target-gated rows are the same class of gap as phases
17 and 19: they need OS providers, a display or hardware this machine does not
have. Everything else here is exercised on every change:
`bash scripts/run-golden-examples.sh` *runs* each headless example (compiling
them proves nothing — their assertions are the point), and
`app/floem2/tests/docs_parity.rs` asserts that this table, the `examples/`
directory and that script's list all agree, so a new example cannot land
undocumented and a documented status cannot quietly become false.

## Notes per example

### `floem2-hello`

The whole app comes from one glob import — the Phase 29.1 surface contract.

```text
hello floem 2.0: mounted and painted 2 widgets
```

Release binary (macOS, Apple M4 Pro, unstripped): **2,694,256 B ≈ 2.6 MiB** —
the floor of the size table, because it pulls the headless core only.

### `floem2-chart`

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

### `floem2-progress-ring`

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

### `floem2-transform`

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

### `floem2-style`

Style realised in primitives (MOTION-GAP A3/A4/B4/B6): a gradient background,
a blurred drop shadow instead of the pseudo-shadow rect v1 faked elevation
with, an implicit transition the author never has to start, and Material's own
motion tokens as its timing. Every act is asserted — including the control that
makes each positive claim mean something: a flat background really does put the
same pixel at both ends, an unraised card really does leave the pixels below it
empty, and an untransitioned button really does snap in one frame.

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
```

### `floem2-motion`

Motion, end to end (MOTION-GAP B1/B2/B3/B5): a curve is not a duration,
interruption carries what it interrupted, a control animates itself, a presence
transition delays unmount, and a crossfade keeps *both* sides alive.

```text
== 4. a presence transition delays unmount ==
  enter fades in; exit keeps the child mounted until it finishes, then it unmounts
== 5. a crossfade keeps both sides live ==
  A and B both stay on screen during the swap; A unmounts only when its exit finishes
```

Act 4 is the enter/exit half of B5: a subtree leaving the screen stays mounted
until its exit animation has played, because a departure nobody sees is not a
transition. Act 5 is the `AnimatedContent` / `Crossfade` half: both contents are
stacked in one box, so mid-swap the scene holds **two** mounted children — the
leaving one at full opacity, the arriving one at none — and only the arriving one
is left once the swap completes.

### `floem2-window`

The only example that opens a real window: winit + vello over wgpu.

```text
GPU adapter: Apple M4 Pro (Metal)
entry fade: 0.4s ease-out on the label (opacity 0 -> 1)
[idle] waits=4 frames=5 redraws_requested=7 auto_redraws=5 events=7 about_to_wait=4
idle probe after 8s: frames=26 redraws_requested=27 auto_redraws=25 events=28 about_to_wait=50; process CPU: 0.24s (2.9% of one core)
```

Run with `FLOEM2_WINDOW_IDLE=1 FLOEM2_WINDOW_IDLE_SECS=8 cargo run -p
floem2-window --features real`. Release binary: **11,626,912 B ≈ 11.1 MiB** —
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

### `floem2-android`

The mobile analogue of `floem2-window`: the same `AppRunner<VelloGpuPresenter>`
widget tree, but pumped by the Android (winit) event loop instead of the desktop
one. `native-activity` loads `libfloem2_android_example.so` and calls
`android_main`, which hands the `AndroidApp` to `floem-platform-android`, mounts
a counter, and runs — the only Android-specific seam is the entry point and the
`AndroidRunner` callbacks that pump the app from winit's push loop. The GPU
surface is built from the Android `ANativeWindow*` by the renderer exactly the
way the desktop window is; no custom surface shim is involved.

The crate is a `cdylib` (a bin-only crate would make `cargo ndk` copy nothing
into `jniLibs/`), so the host build compiles only the placeholder `src/main.rs`.
The Android module is behind the `real` feature, so the cross-check is:

```text
cargo check -p floem2-android-example --target aarch64-linux-android --features real
```

which passes with zero warnings from this crate. Keeping the presenter behind
`real` is what lets `cargo test --workspace` stay dependency-light: the default
host build never pulls wgpu.

It is **device-gated**: this machine has no emulator image, so the APK has never
been launched. What is verified here is compile-level (host + `aarch64-linux-android`
+ the architecture gate), not first-frame on a device.
