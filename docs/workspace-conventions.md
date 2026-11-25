<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# Workspace conventions

Floom is a Cargo workspace organised after the model of
[EpicGames/lore](https://github.com/EpicGames/lore): a single namespace prefix,
single-responsibility crates, a `crates/vendor/` directory for forked upstream code with
source attribution, and a root `deny.toml` for license compliance.

## 1. Crate naming

- **Every crate is prefixed `floom-`** (or `floom_` in the package-name slot, which
  Rust renders as `floom_` on import). No bare-name crates at the workspace root.
- **Semantic suffixes** describe the crate's job:
  - `-types` foundational shared types (L0)
  - `-core` core runtime (L1)
  - `-renderer` / `-renderer-<backend>` backend-neutral ABI and per-backend adapters
  - `-platform` / `-platform-<os>` platform abstraction
  - `-native` native-view composition
  - `-widgets` built-in widgets
  - `-app` the application shell
  - `-testkit` testing harness
- **Legacy (v1) crates are quarantined under `legacy/` with the `floom-legacy-`
  prefix** so they are unmistakable next to their 2.0 successors. Example:
  `crates/render/floom-renderer` (`floom-legacy-renderer`) vs
  `crates/render/floom-renderer` (`floom-renderer`).

## 2. Layered directory topology

Directories encode the architecture layer (enforced by
`scripts/check-architecture-deps.sh`):

| Layer | Directory | Example |
|------|-----------|---------|
| L0 | `foundation/` | `floom-types`, `floom-core` |
| L1 | `runtime/` | `floom-runtime-core`, `floom-runtime`, `floom-scheduler` |
| L2 | `runtime/` | (runtime services) |
| L3 | `scene/`, `layout/`, `text/` | `floom-scene`, `floom-style`, `floom-text` |
| L4 | `render/`, `platform/`, `native/` | `floom-renderer`, `floom-platform-desktop`, `floom-native` |
| L5 | `framework/` | `floom-store`, `floom-task`, `floom-theme` |
| L6 | `widgets/` | `floom-widgets` |
| L7 | `app/` | `floom-app` |
| L8 | `tooling/` | `floom-testkit` |
| — | `legacy/` | `floom-legacy-renderer`, `floom-legacy-reactive`, `floom-legacy-test` |
| — | `ui-events/` | `floem-winit` — the ui-events ↔ winit input bridge |
| — | `crates/vendor/` | forked third-party crates (excluded from the workspace) |
| — | `examples/` | runnable examples (`examples/*`) |
| — | `crates/ecosystem/` | satellite projects, each its own nested workspace (excluded) |

A lower layer must never depend on a higher one. New 2.0 crates are registered in
the layer table in `scripts/check-architecture-deps.sh`.

## 2b. Satellites → `crates/ecosystem/`

The official satellites (`floom-ui`, `floom-editor`) live in this
repository under `crates/ecosystem/`, but each is its **own workspace** — the root
`Cargo.toml` excludes the whole directory. Consequences worth knowing:

- `cargo test --workspace` at the root does not build them. Run cargo from
  `crates/ecosystem/<satellite>/` instead.
- A `[patch]` applies only to the workspace that declares it, so a satellite
  reaching `floom` by path must **re-declare** the vendored `[patch]`
  (`floem-winit`, `floem-winit-core`, `dpi` / `floom`). Skipping this silently
  produces two instances of a type across the boundary.
- The `floom-editor` satellite is one self-contained crate at
  `crates/ecosystem/floom-editor/floom-editor`: it owns the editor model as
  `floom_editor::core` and exposes the `text_editor()` / `TextEditor` API.
- Import history is preserved with `git subtree add` where the satellite had real
  history (`floom-ui`, 17 commits); single-commit scaffolds were imported flat.

## 3. Forked / vendored third-party code → `crates/vendor/`

Forked upstream crates live in **`crates/vendor/`** (lore uses the same name). A fork
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
compliance for them is recorded in the archived
`../floem-archive/notices/manual-additions.toml`.

Current forks:

- `crates/vendor/floem-winit` — fork of [lapce/winit](https://github.com/lapce/winit)
  (iOS open-URL forwarding). See `crates/vendor/floem-winit/NOTICE.md`.
- `crates/vendor/floom-winit-ohos` — fork of `tgui-winit-ohos` 0.4.11 (OHOS
  `floem-winit-core` retarget, renamed off the `tgui` namespace). See
  `crates/vendor/floom-winit-ohos/NOTICE.md`.
- `crates/vendor/harmony-app-rs` — patches for `taoyuan11/harmony-app-rs`
  (`cargo-ohos-app`) making the OHOS packaging flow work on macOS/Linux. See
  `crates/vendor/harmony-app-rs/USAGE.md`.
- `ui-events/floem-winit` — fork of
  [endoli/ui-events](https://github.com/endoli/ui-events) (`ui-events-winit`
  0.1.0). This one stays a **workspace member** (it is built against the
  workspace's own `winit` pin), so it is not `exclude`d — but it carries a
  `NOTICE.md` all the same. See `ui-events/floem-winit/NOTICE.md`.

## 4. License compliance → `deny.toml`

The root `deny.toml` is the dependency-veto / license policy CI enforces
(`cargo deny check`, see `ci-core.yml`). The `cargo-about` configuration and the
manual license annotations for the in-tree forks live in the external archive
under `../floem-archive/notices/` (`about.toml`, `manual-additions.toml`,
`README.md`); generate a third-party notice there with
`cargo about generate --config ../floem-archive/notices/about.toml -o THIRD-PARTY.html`.

## 5. Shared workspace metadata

Root `Cargo.toml` uses `[workspace.package]` (license, version, edition,
rust-version, repository) and `[workspace.dependencies]` so every crate inherits
consistent metadata and dependency versions. Crates reference shared deps with
`{ workspace = true }`.

## 6. What NOT to do

- Do not add a bare-name crate at the workspace root (e.g. `renderer`, `skia`) —
  it breaks the `floom-` convention. Put it under its layer directory.
- Do not fork a crate without a `NOTICE.md` and a `[patch]` entry.
- Do not let a `legacy/` crate gain new dependents from 2.0 code; 2.0 should
  depend only on 2.0 crates.

## 7. 组织与风格约定（2026-11-22 起，参照 tessera）

`tessera-ui/tessera` 与本项目定位几乎相同（wgpu + 声明式 + 响应式 + 移动端），它的文件组织与注释纪律比我们严，故直接采用；API 设计与引擎选型仍按本仓自己的判断，不跟随。

- **模块路径**：`src/name.rs`，不用 `src/name/mod.rs`。2026-11-22 把 29 个 `mod.rs` 全量改名（编译器保证等价），并由 `scripts/check-module-paths.sh` 守住——两种写法都能编译，只有脚本能发现"又退化成两套约定"。`crates/vendor` 例外：保持上游布局，便于再同步。
- **`AGENTS.md`**：根目录一份，写给改代码的 agent 与人：分层、语言、风格、门禁、两条常设规则。内容只写"这个仓库的判断"，通用的 Rust 建议不写；指向 `docs/` 而不是复制。
- **依赖版本策略**写在 `Cargo.toml` 的 `[workspace.dependencies]` 上方注释里：默认 caret、不用开区间与上界区间、跨 crate 边界的类型必须解析到同一版本。
- **注释只写理由**：一段 ≤3 行；删掉复述代码的；"暂时没做"要写明什么时候该补（`shortcut:` 前缀）。
- **每个结构不变量一条脚本**：`check-design-tokens.sh`（颜色字面量）、`check-module-paths.sh`（`mod.rs`）、`check-architecture-deps.sh`（层间依赖）。脚本必须能证明它会红（注入一次再撤）。

未抄的（有意）：tessera 要求全英文文档，本仓 `docs/` 面向中文读者；它的 `error_codes/T0001.md` 一码一档、随 CLI 分发的 `skills/` 目录，本仓尚无对应物，已记在 `progress.md` 的 S0.4 未做项里。
