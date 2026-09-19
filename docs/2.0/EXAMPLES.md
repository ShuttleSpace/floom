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
| `floem2-hello` | the smallest app: everything from `floem2::prelude` | all | headless |
| `floem2-counter` | composites (`Stack`/`List`/`Grid`/`Tabs`/`Dialog`/`NavBar`) + reconciliation | all | headless |
| `floem2-list` | a keyed list: append / remove / reorder without rebuilding rows | all | headless |
| `floem2-virtual-list` | virtual scrolling: bounded element pool over 100 000 items | all | headless |
| `floem2-datagrid` | data grid: virtualized rows, sorting, selection, edit + undo | all | headless |
| `floem2-forms` | form validation, dirty/touched tracking, control bindings | all | headless |
| `floem2-navigation` | typed routes, history, page transitions, nav containers, deep links | all | headless |
| `floem2-editor` | text document: transactions, selection, undo/redo, IME pre-edit | all | headless |
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

### `floem2-counter`

Mounts a vertical `Stack` of composites, then mutates them and pumps frames,
printing what the reconciler did. The interesting number is *what does not
change*: appending a list row mounts one row, not a whole subtree.

### `floem2-list`

Rows are identified by a stable key, so the collection can be replaced freely.

```text
mounted : 1001 live widgets for 1000 rows
append  : 1002 widgets (+1), rows now 1001
remove  : 1001 widgets (released) after dropping row 500
reorder : 1001 widgets (same set, new order)
```

The assertion is the point: appending must grow the tree by at most a handful of
widgets — if a list ever rebuilds its rows, this example fails.

### `floem2-virtual-list`

"Virtualized" is a claim about cost, so this example scrolls a 100 000-item list
end to end and counts elements ever created.

```text
items        : 100000
viewport     : 800px @ 24px rows
visible rows : 36
elements created over the whole scroll : 38
pool high-water mark                   : 38
```

**0.04 %** of the list is ever materialized.

### `floem2-datagrid`

Two grids of *identical shape* — same ten-row window — differing only in the
number of rows behind them:

```text
rows behind the grid : 1,000 vs 100,000
paint ops           : 44 vs 44
row pool high-water : 10 live rows (allocated 10, in use 10)
```

Equal paint ops is the virtualization claim (comparing differently shaped
fixtures would prove nothing). The rest of the run exercises the three-state
sort cycle (ascending → descending → unsorted), selection, and an edit session
opened the way the UI opens one — a double click — with undo/redo:

```text
editor open           : row=RowId(1) column=ColumnId(1)
cell edit             : "1" -> "4242" (committed row RowId(1))
after undo            : "1"
after redo            : "4242"
```

### `floem2-forms`

Rules (`required`, `email`, `min_length(8)`), dirty/touched separation, and the
`FieldBinding` bridge between a form field and `TextField`:

```text
pristine : valid=false errors=[("email", "this field is required"), ("password", "must be at least 8 characters")]
email says : Some("must be a valid email address")
complete : valid=true email=Some("alice@example.com")
```

### `floem2-navigation`

Typed routes (`encode`/`decode` are exact inverses), a bounded history, a page
stack whose popped pages survive their exit transition, containers, and the one
stringly-typed edge — deep links:

```text
current=Settings depth=1 can_pop=true history=["/profile/7", "/settings"]
after pop: top="/profile/42" exiting=Some("/settings")
deep link -> path=/profile/42 route=Profile { id: 42 } tab=Some("posts")
```

### `floem2-editor`

`apply` changes text and returns the inverse; `commit` applies *and* records it.
Two histories are shown side by side — the document's own, and an app-owned
`UndoManager` fed by `apply`'s inverses — plus the IME rule that a pre-edit is
never part of the document:

```text
ime preedit      : Some("にほん")
ime committed    : "hi, world?にほん"
```

### `floem2-native-plugin`

Writes a plugin (a `NativeViewProvider` that can only do overlays), then proves
negotiation downgrades rather than silently granting, and that pooling keeps
live slots bounded across 1 000 rows:

```text
requested     : [Texture, Overlay]   →  negotiated : Some(Overlay)
plugin counts : created=1 destroyed=1 live=0 applies=1
pool          : live=8 free=1
recycled slot : generation=112 (was 1)
```

### `floem2-window`

The only example that opens a real window: winit + vello over wgpu.

```text
GPU adapter: Apple M4 Pro (Metal)
idle probe after 8s: frames=2 redraws_requested=3 auto_redraws=1 events=4 about_to_wait=84; process CPU: 0.17s (2.1% of one core)
```

Run with `FLOEM2_WINDOW_IDLE=1 FLOEM2_WINDOW_IDLE_SECS=8 cargo run -p
floem2-window --features real`. Release binary: **11,626,912 B ≈ 11.1 MiB** —
what the desktop + GPU bundle costs on top of the 2.6 MiB headless floor.

The load-bearing number is `frames=2`: an idle window must not keep producing
frames. `redraws_requested` and the CPU percentage move a little run to run
(3 / 2.1 % here, 2 / 1.7 % on an earlier run) because they count the startup
redraw and whatever the window server sends in the first second; the gate is on
frames, not on those.

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
