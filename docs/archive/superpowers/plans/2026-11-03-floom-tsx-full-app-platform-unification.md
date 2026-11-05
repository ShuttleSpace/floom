# floom-tsx: TSX-as-full-app + unified platform API

## Goal

Two follow-on requirements to the floom-tsx crate already built (tasks 1–10):

1. **TSX as the whole app.** When developing in TSX, *no Rust is written by the
   author*. Layout / platform / app lifecycle are expressed in TSX and run directly
   (like nsh: a single `.tsx` compiles to a runnable binary, driven by `floom`).
   The `floom` CLI owns compile + run.
2. **One unified platform API.** Collapse `run_desktop` / `run_android` /
   `run_web` / `run_tui` into a single `floom::run` (and a `floom run <app.tsx>`
   CLI command) backed by a trait. Each platform backend implements the trait; a
   backend that cannot satisfy a call degrades explicitly — `warn!` / `err!` /
   `force exit` — rather than silently misbehaving.

## Current state (grounding)

- `crates/floom` facade re-exports each backend behind a feature flag
  (`desktop`, `mobile`, `tui`, `web`, `embedded`, …) and exposes separate
  one-call entries: `run_desktop`, `run_desktop_with_devtools`, `run_android`.
  Each takes a backend-specific app trait (`WinitApp`, `TuiApp`, `WebApp`,
  `AndroidRunner`).
- `crates/floom-platform/src/lib.rs` already defines a `Platform` trait
  (`id`, `capabilities`, `create_window`, `poll_events`, `request_frame`,
  `ime`, …) — i.e. the *backend* contract. What is missing is the *entry-point*
  unification and the author-facing single API.
- `crates/floom-cli` only scaffolds (`floom create`); it has no `run`/`dev`
  command and does not touch TSX.
- `crates/floom-tsx` already compiles a `.tsx` → `floom_ui_ir::UiDocument` →
  `floom_dsl::emit(&doc) -> Rust` (a `pub fn build() -> Box<dyn WidgetAny>`).
  The example `examples/floom-tsx-showcase` proves the path but still has a hand-
  written `main.rs`.

## Approach

### A. Unified entry: `floom::run`

Introduce one author-facing entry that dispatches to the compiled-in backend via
`#[cfg]`:

```rust
pub fn run<F>(build: F) -> Result<(), String>
where F: FnOnce(&mut WidgetHost) + 'static { … }
```

- `run` selects the backend from the active feature(s): `#[cfg(feature =
  "desktop")] floom_platform_desktop::winit_backend::run(app, windows)`, etc.
  A backend not compiled in is a hard error at the `run` call (clear message),
  not a silent no-op.
- The `F: FnOnce(&mut WidgetHost)` is wrapped into each backend's `*App` trait
  once, so authors never name `WinitApp`/`TuiApp`/`WebApp`. This is the "one
  external API" the requirement asks for.
- Keep `run_desktop` / `run_android` as thin backward-compat shims delegating to
  `run` (so existing callers don't break), or deprecate them.

### B. TSX as full app

Add to `floom-tsx`:

- `floom_tsx::compile_crate` (or extend `compile_project`) that, given a `.tsx`
  file, returns both the emitted `ui_gen.rs` **and** a generated `main.rs` that
  calls `floom::run(|host| host.mount_root(LayoutStyle::default(), ui_gen::build()))`.
- A TSX app is a single `.tsx` with one default-exported root component; the
  compiler emits the tree and the mount. Window/platform are configurable via a
  small declarative header (e.g. a top-level `<App title="…" size="480x320">…</App>`
  or attributes), so the author stays in TSX.

`floom-cli` gains a `run` command:

```
floom run app.tsx            # compile -> build a temp crate -> cargo run
floom run app.tsx --platform desktop   # optional target override
```

The command writes a throwaway crate (generated `Cargo.toml` wiring `floom` +
`floom-tsx` + `floom-dsl` + the chosen platform features), includes the two
generated files, builds, and runs it. This is the "no Rust at all" path.

### C. Unified platform trait + graceful degradation

Wrap the existing `floom_platform::Platform` capabilities behind a thin
`floom::platform` facade whose methods return `Result<…, PlatformError>` (or
`Option`/`()` with logging). A backend that cannot implement a call:

- returns `Err(PlatformError::Unsupported(..))`; the facade logs `warn!` for
  recoverable gaps and `error!` (or `std::process::exit(1)`) for hard ones,
  per the requirement's "warn! / err! / force exit" instruction. Which one is
  per-capability, decided in the facade (e.g. clipboard missing on a headless
  target → `warn!`; no GPU presenter → `exit`).

This keeps the *backends* thin (they already return `Option`/error where they
can) and centralizes the "what to do when unsupported" policy in one place.

## Phasing (recommend building in this order)

1. **Unified `floom::run`** + collapse `run_desktop`/`run_android` into it (desktop
   first, then wire mobile/web/tui under the same `run`). Tests: a host test
   that mounts a tree via `run` headlessly (software presenter) and checks a
   frame painted.
2. **`floom-tsx` full-app emit** (`main.rs` generation) + **`floom run app.tsx`**
   CLI command (desktop target). Tests: `floom run` on the showcase `.tsx`
   produces a binary that paints a frame (reuse `floom-testkit`).
3. **Platform facade + graceful degradation** (`warn!`/`err!`/`exit`). Tests:
   a backend missing a capability yields the expected warning/exit policy.

## Non-goals (carry over from the original plan)

Route B runtime VM, TSX hot reload, GPUI backend, native-view rendering, mixed
Rust+TSX, web-target bundling specifics. The unified API is a *compile-time
dispatch* over the existing backends, not a new runtime layer.

## Decisions (owner, 2026-11-03)

- **Scope:** desktop closure first (phases 1–2); mobile/web/tui later.
- **App declaration:** a top-level `<App title="…" size="WxH">` wrapper.
- **Backward compat:** `run_desktop` stays as a deprecated shim for `run`.

## Progress

### Phase 1 — unified `floom::run` ✅

- `crates/floom/src/platform.rs`: `PlatformBackend` trait (name / capabilities /
  `run`), `AppRequest`, `BuildFn`, `PlatformError`, `OnUnsupported`
  (`Warn` / `Fail` / `Exit`), `current()` (cfg-selected) and `require(cap, policy)`.
- `floom::run(title, size, build)` is now backend-agnostic and defined for every
  feature set. A build with no backend compiles and fails at call time with a
  message naming the feature to enable.
- `run_desktop` is a `#[deprecated]` alias.
- Backend impls: desktop (winit + vello). `NoBackend` stands in for the rest and
  is what `--features ""` exercises.

### Phase 2 — TSX full app + `floom run` ✅

- `crates/floom-tsx/src/app.rs`: `compile` / `compile_file` (unwrap `<App>`,
  return `AppConfig`), `emit_ui_gen`, `emit_main`, `generate`.
- `crates/floom-cli/src/run_app.rs`: `floom run app.tsx [--release] [--no-run]
  [--features LIST] [--floom-path PATH]`. Writes a throwaway crate under
  `<app dir>/.floom/run` (its own `[workspace]`, `build.rs` depends on
  `floom-tsx` + `floom-dsl`), then `cargo run`/`cargo build` it. The CLI itself
  stays dependency-free.
- `examples/floom-tsx-app/`: the whole app is `app.tsx` + README — no
  `Cargo.toml`, no `main.rs`. Excluded from the root workspace because it has no
  manifest by design.
- Verified: `floom run app.tsx --features ""` generates, compiles and reports
  the missing backend; `floom run app.tsx --no-run` links the full desktop stack.

### Phase 3 — capability degradation ✅ (facade only)

`OnUnsupported` is the policy seam; per-backend wiring for mobile/web/tui lands
with those backends.

## Still open

- GUI visual acceptance of `examples/floom-tsx-app` on a machine with a GPU.
- Reactive handler lowering (`onClick={() => …}`) so the counter demo can
  change text — the same gap tracked in the original plan's Task 10.

### Phase 4 — nsh-style entry surface (added)

nsh drives the runtime from TSX (`mount(App)` + `window.*` / `platform.*`
callables) rather than returning a `defaultView`. floem-tsx now offers the same
shape, compile-time:

- `crates/floom-tsx/src/entry.rs` (new): `resolve()` scans top-level
  `mount(App)`, `window.setTitle/setSize/setResizable`, `app.setName`,
  `platform.require(cap, policy)`, resolves the named component's returned JSX,
  and produces `Entry { root, AppConfig }`. `mount(() => <Stack/>)` inline works
  too. Unknown window/app/platform methods and unknown capabilities/policies are
  span-located errors.
- `lower` no longer detects the `<App>` wrapper; `resolve` unwraps it (the
  wrapper is not a widget) and applies its attributes as config, with explicit
  `window.*` calls winning.
- Generated `main.rs` goes through the unified `floom::platform::current().run(
  AppRequest { … resizable }, …)` and emits a `floom::platform::require(..)` call
  per `platform.require` — so the OnUnsupported policy is chosen in TSX.
- `.d.ts` exports `declare module "floom"` with `mount` and the `window` / `app`
  / `platform` handles.
- `examples/floom-tsx-app/app.tsx` rewritten in this style.

Verified: `floom run app.tsx --features ""` prints the IME `require` warning
then the "no backend" error; `--no-run` links desktop + vello.

### Decisions updated

- **Scope**: still desktop-first. mobile/web/tui backends remain to be wired into
  `floom::platform::current()`; the unified facade and the TSX entry surface are
  backend-agnostic and already compile/run on desktop.

### Known limitation restated

- Event handler lowering (`onClick={() => …}`) is still not implemented — the
  demo is static. That is the gap that keeps the GUI from being interactive.
