# Hot reload in floom (`floom run --watch`)

> Companion to [`1.0-launch-milestone.md`](1.0-launch-milestone.md) WS-4.4 / P1-5.
> Read this before claiming floom has "stateful hot reload".

## What `floom run --watch` does

```
floom run app.tsx --watch          # rebuild + relaunch on source change
floom run app.tsx --no-relaunch    # watch + rebuild only (no window)
floom run app.tsx --watch --poll-ms 100 --debounce-ms 100
```

It builds the generated crate once, launches the binary directly (so it is
killable), and on every **settled** change to a watched source file
(`.tsx`/`.ts`/`.json` under the app's directory) it kills the running copy,
rebuilds, and relaunches. This is a **hot *restart*** — the window closes and
reopens, and app state is lost. It is the same shape as `cargo-watch` or slint's
`--auto-reload`.

The watcher is **pure std** (`floom-cli` is deliberately dependency-free; see
`crates/app/floom-cli/src/watch.rs`). There is no `notify` crate: we poll
modification times on a configurable interval (`--poll-ms`, default 200) and
debounce bursts (`--debounce-ms`, default 100) so one editor "save" — which
writes several events — collapses into a single rebuild. The directory walk
skips `target`/`.git`/`node_modules`/`.floom`, so the rebuild's own output never
re-triggers the watch.

## What it does NOT do, and why

It is **not** stateful, in-window hot reload (flutter ~978ms, dioxus `--hotpatch`).
That would require bridging `floom-hot-reload`'s `UiPatch` to a live host, and the
code to do that does not exist yet:

- `floom-hot-reload`'s `UiPatch` patches a **headless inspector `LiveTree`**, not a
  running app's `WidgetHost`. The crate's own module docs state that widget
  properties "a real host forwards those separately" — that forwarding is
  unimplemented. No code path connects `diff()` / `LiveTree::apply` to a window.
- `.tsx` is **compiled into Rust at build time** (`floom run`'s generated
  `build.rs` → `floom_tsx::generate` → `include!` into the binary; the builder is
  `FnOnce`, called once). There is no runtime IR interpreter to re-feed.
- There is **no dynamic loading** anywhere in the app crates — no `libloading`,
  `dylib` hotpatch, or `cdylib` reload entry. A running process cannot swap its
  widget tree for newly compiled code without a restart.

Building the stateful path is a real subsystem: a runtime IR interpreter, a
`UiPatch`→`WidgetHost` bridge, and a dylib load/replace. It is unsafe-leaning and
device-gated (needs a real window/GPU). It is appropriate for a **1.0.x** item,
not 1.0.0.

## Latency measurement — discipline

The milestone ([`1.0-launch-milestone.md`](1.0-launch-milestone.md) WS-4.4, and
`release-policy.md` step 6) asks for a measured save→running latency. Practice:

- The number is produced on demand by `--no-relaunch` (or `--watch`), which
  prints a breakdown: `detect +Nms, debounce Mms, rebuild Rms, relaunch Lms =>
  save→running ~Tms (detection bounded by poll Kms)`. The "save" anchor is the
  instant a change was first observed (not the raw save), so the printed `T`
  is `observed→running`; the underlying save→observed gap is bounded by
  `--poll-ms` and is stated explicitly.
- **Do not** quote a number in the 1.0.0 release notes as a floom-vs-flutter
  comparison. Per the milestone, if hot reload is a 1.0.x deliverable, the
  release note says "未测" rather than a misleading figure.
- Record any number you do measure **with its conditions**: machine, profile
  (debug is far slower than release for a cold build), and whether the crate's
  target/ was warm (incremental) — a cold build is not representative of the
  steady-state edit loop.

## Files

- `crates/app/floom-cli/src/watch.rs` — the pure-std watcher (snapshot, debounce,
  `Watch::wait_for_change`, which rebases onto each settled change so the same
  edit does not fire in a tight loop).
- `crates/app/floom-cli/src/run_app.rs` — `--watch` / `--no-relaunch` /
  `--poll-ms` / `--debounce-ms`; `build_app` + `binary_path`; the watch loop.
- `crates/app/floom-cli/src/main.rs` — USAGE text.
- `floom-hot-reload` is **untouched** — it is the intended future stateful path.
