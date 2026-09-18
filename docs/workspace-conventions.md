# Workspace conventions

Floem is a Cargo workspace organised after the model of
[EpicGames/lore](https://github.com/EpicGames/lore): a single namespace prefix,
single-responsibility crates, a `vendor/` directory for forked upstream code with
source attribution, and a `notices/` directory for license compliance.

## 1. Crate naming

- **Every crate is prefixed `floem-`** (or `floem_` in the package-name slot, which
  Rust renders as `floem_` on import). No bare-name crates at the workspace root.
- **Semantic suffixes** describe the crate's job:
  - `-types` foundational shared types (L0)
  - `-core` core runtime (L1)
  - `-renderer` / `-renderer-<backend>` backend-neutral ABI and per-backend adapters
  - `-platform` / `-platform-<os>` platform abstraction
  - `-native` native-view composition
  - `-widgets` built-in widgets
  - `-app` the application shell
  - `-testkit` testing harness
- **Legacy (v1) crates are quarantined under `legacy/` with the `floem-legacy-`
  prefix** so they are unmistakable next to their 2.0 successors. Example:
  `legacy/floem-renderer` (`floem-legacy-renderer`) vs
  `render/floem-renderer` (`floem-renderer`).

## 2. Layered directory topology

Directories encode the architecture layer (enforced by
`scripts/check-architecture-deps.sh`):

| Layer | Directory | Example |
|------|-----------|---------|
| L0 | `foundation/` | `floem-types`, `floem-core` |
| L1 | `runtime/` | `floem-runtime-core`, `floem-runtime`, `floem-scheduler` |
| L2 | `runtime/` | (runtime services) |
| L3 | `scene/`, `layout/`, `text/` | `floem-scene`, `floem-style`, `floem-text` |
| L4 | `render/`, `platform/`, `native/` | `floem-renderer`, `floem-platform-desktop`, `floem-native` |
| L5 | `framework/` | `floem-store`, `floem-task`, `floem-theme` |
| L6 | `widgets/` | `floem-widgets` |
| L7 | `app/` | `floem-app` |
| L8 | `tooling/` | `floem-testkit` |
| — | `legacy/` | `floem-legacy-renderer`, `floem-legacy-reactive`, `floem-legacy-test` |
| — | `ui-events/` | `floem-winit` — the ui-events ↔ winit input bridge |
| — | `vendor/` | forked third-party crates (excluded from the workspace) |
| — | `examples/` | runnable examples (`examples/*`) |
| — | `ecosystem/` | satellite projects, each its own nested workspace (excluded) |

A lower layer must never depend on a higher one. New 2.0 crates are registered in
the layer table in `scripts/check-architecture-deps.sh`.

## 2b. Satellites → `ecosystem/`

The official satellites (`floem-ui`, `floem-editor`, `flameo`) live in this
repository under `ecosystem/`, but each is its **own workspace** — the root
`Cargo.toml` excludes the whole directory. Consequences worth knowing:

- `cargo test --workspace` at the root does not build them. Run cargo from
  `ecosystem/<satellite>/` instead.
- A `[patch]` applies only to the workspace that declares it, so a satellite
  reaching `floem` by path must **re-declare** the vendored `[patch]`
  (`floem-winit`, `floem-winit-core`, `dpi` / `floem`). Skipping this silently
  produces two instances of a type across the boundary.
- Import history is preserved with `git subtree add` where the satellite had real
  history (`floem-ui`, 17 commits); single-commit scaffolds were imported flat.

## 3. Forked / vendored third-party code → `vendor/`

Forked upstream crates live in **`vendor/`** (lore uses the same name). A fork
that has to stay a workspace member — because it compiles against a
workspace-wide dependency pin — lives under its own directory instead (see
`ui-events/`), but the attribution rule is identical. **Every fork carries a
`NOTICE.md`** stating:

- the upstream project URL,
- the exact fork rev / version,
- the local delta (what was changed and why),
- the exit criterion (when the fork can be deleted).

They are wired in via `[patch]` in the root `Cargo.toml` and are listed in
`exclude = ["vendor"]` so they are not ordinary workspace members. License
compliance for them is recorded in `notices/manual-additions.toml`.

Current forks:

- `vendor/floem-winit` — fork of [lapce/winit](https://github.com/lapce/winit)
  (iOS open-URL forwarding). See `vendor/floem-winit/NOTICE.md`.
- `vendor/floem-winit-ohos` — fork of `tgui-winit-ohos` 0.4.11 (OHOS
  `floem-winit-core` retarget, renamed off the `tgui` namespace). See
  `vendor/floem-winit-ohos/NOTICE.md`.
- `vendor/harmony-app-rs` — patches for `taoyuan11/harmony-app-rs`
  (`cargo-ohos-app`) making the OHOS packaging flow work on macOS/Linux. See
  `vendor/harmony-app-rs/USAGE.md`.
- `ui-events/floem-winit` — fork of
  [endoli/ui-events](https://github.com/endoli/ui-events) (`ui-events-winit`
  0.1.0). This one stays a **workspace member** (it is built against the
  workspace's own `winit` pin), so it is not `exclude`d — but it carries a
  `NOTICE.md` all the same. See `ui-events/floem-winit/NOTICE.md`.

## 4. License compliance → `notices/`

`notices/` holds `deny.toml` (cargo-deny) and `about.toml` (cargo-about) configs.
Run `cargo deny --config notices/deny.toml check` and
`cargo about generate --config notices/about.toml -o THIRD-PARTY.html` after
installing the tools. See `notices/README.md`.

## 5. Shared workspace metadata

Root `Cargo.toml` uses `[workspace.package]` (license, version, edition,
rust-version, repository) and `[workspace.dependencies]` so every crate inherits
consistent metadata and dependency versions. Crates reference shared deps with
`{ workspace = true }`.

## 6. What NOT to do

- Do not add a bare-name crate at the workspace root (e.g. `renderer`, `skia`) —
  it breaks the `floem-` convention. Put it under its layer directory.
- Do not fork a crate without a `NOTICE.md` and a `[patch]` entry.
- Do not let a `legacy/` crate gain new dependents from 2.0 code; 2.0 should
  depend only on 2.0 crates.
