<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# Golden examples (Floom 2.0)

`examples/floom-showcase` is the one application: built-in widgets, signals,
the ported `floom-ui` kits, the editor, and the devtools, behind a router. The
remaining rows are the platform and device gates, which need an OS that CI does
not have.

`cargo run -p <name>` runs one; `cargo build --workspace` compiles all of them.

## The list

| example | topic | target | status |
| --- | --- | --- | --- |
| `floom-showcase` | the application: six routes over the engine (`/components`), `floom-signal` (`/signals`), the ported material kits (`/kits`), `floom-editor` (`/editor`), and the devtools entry (`/devtools`). The shell owns a `floom-navigation` router; each route is one page of a real window with a top bar, sidebar, status bar and a signal-backed readout. `F12` opens the devtools panel, which is attached by `run_desktop_with_devtools` | desktop | headless (`--headless` mounts and pumps every route with a hand-driven `WidgetHost`); `cargo run -p floom-showcase` opens the window |
| `floom-native-plugin` | writing a native provider plugin + slot pooling | all | headless |
| `floom-tsx-showcase` | a GUI authored in TSX and materialized at build time: `build.rs` lowers `src/counter.tsx` to a `UiDocument` and emits Rust that `main.rs` mounts — no separate codegen step | desktop | **integration-tested** — `cargo test -p floom-tsx-showcase` mounts and paints it through `floom-testkit` (software presenter); `--features real` opens a winit + vello window |
| `floom-probe` | device probe: one window holding every control the on-screen cases need | desktop | **display-gated** — `--features real`, driven by hand |
| `floom-window` | a real OS window: winit + vello GPU | desktop | **verified on macOS** |
| `floom-macos` | the macOS gate probe: a clickable tree, a text field an input method can type into, and a clipboard whose far end is the system pasteboard | macOS | **display-gated** — `--features real`, driven by hand (see `docs/gates/macos.md`) |
| `floom-android` | a real Android app: `native-activity` + vello GPU in one APK | Android | **device-gated** — `--features real`, packaged by `examples/floom-android/scripts/build-apk.sh` |

The device-, display- and target-gated rows need OS providers, a display or
hardware this machine does not have. Everything else is exercised on every
change: `bash scripts/run-golden-examples.sh` *runs* each headless example
(compiling them proves nothing — their assertions are the point), and
`crates/app/floom/tests/docs_parity.rs` asserts that this table, the `examples/`
directory and that script's list all agree, so a new example cannot land
undocumented and a documented status cannot quietly become false.

## One command per platform

`just build <crate> <platform>` (i.e. `scripts/build.sh`) is the single entry
point: Android delegates to `scripts/build-apk.sh`, iOS to `scripts/ios-bundle.sh`,
OHOS to the crate's hvigor Stage project, and each run ends with one honest
result line —

```text
TIER=<artifact|check-only|blocked> PLATFORM=<p> CRATE=<c> ARTIFACT=<path|->
```

A platform whose backend cannot render, or whose toolchain is missing, is
reported as `check-only`/`blocked`, never as `artifact`.

## Notes per example

### `floom-showcase`

Six routes, one shell. The router is a plain `Router<AppRoute>` owned by the
shell widget; `floom-navigation` is widget-free, so the shell re-declares its
subtree when `Router::current` changes and the host diffs it by `ChildKey`.

The state that must outlive a page lives in `AppState`: `SyncBaseSignal`s for the
counters and toggles, and an `Arc<Mutex<EditorWidget>>` for the document. The
`Sync` variants are the ones a `Child::new` builder (`Fn + Send + Sync`) can
capture; an `Rc`-based `RwSignal` cannot cross that boundary.

```sh
cargo run -p floom-showcase            # the window
cargo run -p floom-showcase -- --headless
# /            painted 65
# /components  painted 66
# /signals     painted 60
# /kits        painted 58
# /editor      painted 45
# /devtools    painted 46
```

The kits page names only the ported material crates it mounts
(`floom-widget-md-text`, `floom-widget-md-button`, `floom-widget-md-checkbox`);
`crates/ecosystem/floom-ui` is a satellite workspace and pulling it in whole
would build 300+ members for three components.

### `floom-native-plugin`

The native-provider seam: a plugin negotiates a capability, the host grants a
slot, and a pool reuses it. Headless, with assertions rather than a window.

### `floom-window`

The GPU/window baseline: winit + vello over wgpu.

```text
GPU adapter: Apple M4 Pro (Metal)
entry fade: 0.4s ease-out on the label (opacity 0 -> 1)
[idle] waits=4 frames=5 redraws_requested=7 auto_redraws=5 events=7 about_to_wait=4
idle probe after 8s: frames=26 redraws_requested=27 auto_redraws=25 events=28 about_to_wait=50; process CPU: 0.24s (2.9% of one core)
```

Run with `FLOOM2_WINDOW_IDLE=1 FLOOM2_WINDOW_IDLE_SECS=8 cargo run -p
floom-window --features real`. Release binary: **11,626,912 B ≈ 11.1 MiB** —
what the desktop + GPU bundle costs on top of the native-plugin floor.

**The load-bearing claim is that `frames` stops climbing, not that it is small.**
Before the motion work landed the number was `frames=2`; the label now fades in
(0.4 s, ease-out, opacity), so a run reads `frames=26` — the mount frame plus the
fade's own 24 — and then freezes there for the remaining ~7.5 s, with
`about_to_wait` at 50 rather than in the thousands. That makes the case *stronger*
than it was: the number is no longer small for the trivial reason that nothing
ever animated, and an endless animation would have kept it climbing (which is why
the fade is one-shot — see `WindowApp::intro_fade`). The trace lines show the
climb frame by frame, so a wedged loop is visible in the output rather than only
in the total.

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
`NSPasteboard` (see `crates/platform/floom-platform-desktop/src/clipboard.rs`), so the
same string is then readable from a different process entirely — which is what
separates a clipboard from a `String` field. The `logical=` lines are there so a
click can be *aimed*: they are content-relative, so the window's frame and the
32 pt title bar have to be added before they are screen coordinates.

### `floom-android`

The mobile analogue of `floom-window`: the same `AppRunner<VelloGpuPresenter>`
widget tree, but pumped by the Android (winit) event loop instead of the desktop
one. `native-activity` loads `libfloom2_android_example.so` and calls
`android_main`, which hands the `AndroidApp` to `floom-platform-android`, mounts
a counter, and runs — the only Android-specific seam is the entry point and the
`AndroidRunner` callbacks that pump the app from winit's push loop.

The crate is a `cdylib`, so the host build compiles only the placeholder
`src/main.rs`. The Android module is behind the `real` feature, so the
cross-check is:

```text
cargo check -p floom-android-example --target aarch64-linux-android --features real
```

It is **device-gated**: what is verified here is compile-level (host +
`aarch64-linux-android` + the architecture gate), not first-frame on a device.

### Removed examples

The rows below used to advertise example crates that were deleted. They are
listed here rather than silently dropped, because "this existed and does not" is
the answer to "where did the iOS example go" and a missing row does not answer
it.

The names below are written without backticks on purpose: the index
table above is recognised by its first column being code, and a second
code-first table would be read as more advertised examples.

| was | what it was | why it is gone |
| --- | --- | --- |
| ios | the iOS app: counter, in-UI context menu, `UIPasteboard` clipboard, soft keyboard | targeted the removed stack |
| ohos | the OpenHarmony app: a `cdylib` loaded by an ArkTS Stage project | targeted the removed stack |
| webgpu | the browser app: `wasm32` + WebGPU | targeted the removed stack |
| webview, video, camera | embedded OS-provided views | targeted the removed native-view path |
| harmony | the HarmonyOS build | targeted the removed stack |
| web, embedded | wasm32 / no-std builds | targeted the removed stack |

Each of these is a target Floom still has (`floom-platform` with the `ios` /
`embedded` features, `floom-platform-web`, `floom-native` for the embedded
views). What is missing is a real example per target, not a backend.
