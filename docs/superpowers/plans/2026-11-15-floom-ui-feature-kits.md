# floom-ui Feature-Kit 合并 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 `crates/ecosystem/floom-ui` 每个 design system 的「薄 kit crate + ~99 个组件 crate」合并成 1 个 crate，每组件一个 `#[cfg(feature)]`，删 `floom-harmony`，workspace member 306 → ~9。

**Architecture:** 一次性生成脚本 `scripts/merge_kit.py <ds>` 把各组件 `src/lib.rs` 搬进 `crates/floom-<ds>/src/widgets/<feat>.rs`（feature 门控）、`tests/*.rs` 搬进 kit 的 `tests/`（`#![cfg]` 门控）、依赖并集写进单一 `Cargo.toml`。薄 kit 的令牌层 `mod button/gallery/scale/tokens` 原地保留，组件放 `widgets::` 子命名空间避免同名冲突。examples / 门禁 / CI 随之迁移。

**Tech Stack:** Rust (Cargo features, `#[cfg]`, integration tests), Python 3 标准库（生成/校验脚本）, bash (gate.sh), GitHub Actions (ci-plugins.yml)。

**Spec:** `docs/superpowers/specs/2026-11-15-floom-ui-feature-kits-design.md`（计划据此立论，执行者两篇都读）。

## Global Constraints

- 每个 Task 末尾一次 **Conventional Commit**；**绝不 `git push`**（用户明确要求等确认）。
- **工作树已有大量预先存在的未提交改动**（删 `sample/`、改 `app/floom-cli/src/toolchain.rs`、未跟踪 `examples/floom-plugin-demo/`、另一条 in-flight 的 `2026-11-14-selfcontained-packaging` 计划）。每次提交**只能精确 `git add <本任务文件>`**；禁止 `git add -A` / `git add .` / `git commit -a`。开工前先 `git status --short` 记录基线。
- 合并后的 kit crate 必须**写死 `edition = "2021"`**，不用 `edition.workspace = true`（workspace 是 2024）。组件原本是 2021，保住「纯物理搬运、不改组件代码」。
- `default = []`（全关）；`full = [所有组件 feature]`；每组件一条 feature，交叉依赖用 `dep = ["icon"]` 形式。
- feature 命名：去 `<prefix>-`、连字符→下划线。material prefix=`md`，cupertino/fluent prefix=各自名。
- 组件 crate 实名 `floom-widget-{md,cupertino,fluent}-<name>`；引用前缀 `floom_widget_{md,cupertino,fluent}_<name>`。
- 门禁/CI 必须加 `--all-features`，否则默认全关 → 组件测试静默不跑、gate 假绿。
- 不改组件行为/绘制/令牌取值；不重写 `docs/PLAN.md`/`COMPONENT_PARITY.md` 全文；不动 `scripts/scaffold_*.py`/`swap_*.py`（历史生成脚本）。
- 所有验证在 `crates/ecosystem/floom-ui`（satellite workspace）跑，除非注明根仓库。

## Review Focus

spec 隐含、最可能咬人、但需专门钉死的失败模式（各归入下方 Task 的验证步骤）：

1. **门禁放水**：`default=[]` 且 gate 忘加 `--all-features` → 全绿但零组件测试。→ Task 5 的「数测试数」验证 + CI 同步。
2. **feature 漏映射**：依赖 icon 的组件单开该 feature 时 `crate::widgets::icon` 不存在 → 编译失败。→ Task 1 校验脚本 + Task 2/3/4 的单组件编译。
3. **edition 2024 重编译**：kit 继承 2024 → 293 文件在 2024 规则下 break。→ Task 2 显式断言 `edition = "2021"`。
4. **删源与删 workspace 依赖不同步**：残留 `floom-widget-*-icon = { workspace = true }` → dangling 编译失败。→ Task 4 原子提交 + `cargo metadata` 校验。
5. **薄 kit golden 测试被覆盖**：`tests/gallery.rs`(×3)、`tests/motion.rs` 丢失且无人察觉。→ Task 1 校验脚本断言这些文件仍在 + Task 2/3/4 测试里包含它们。

## File Structure

| 文件 | 职责 | 动作 |
|---|---|---|
| `scripts/merge_kit.py` | 一次性生成：搬 src/tests、写 widgets/mod.rs、生成 Cargo.toml | 创建（Task 1）|
| `scripts/verify_merge.py` | 结构校验：feature 集合、cfg 行数、tests 数、无 dangling 引用、kit golden 仍在 | 创建（Task 1，先于实现，作失败测试）|
| `crates/floom-material/` `floom-cupertino/` `floom-fluent/` | 合并产物：`src/widgets/*.rs`、`src/widgets/mod.rs`、`tests/*`、`Cargo.toml`、`src/lib.rs` | 修改/新增（Task 2/3/4）|
| `crates/floom-harmony/` | 删除 | 删（Task 4）|
| `crates/{material,cupertino,fluent}/<prefix>-*/` | 组件源 crate，全部删除 | 删（Task 4，原子）|
| `crates/ecosystem/floom-ui/Cargo.toml` | members 收敛、`[workspace.dependencies]` 删 3 条 icon path | 修改（Task 4）|
| `examples/floom-demo/{Cargo.toml,src}`、`examples/floom-showcase/{Cargo.toml,src}` | 294+9 处引用迁移 | 修改（Task 5）|
| `scripts/gate.sh`、`.github/workflows/ci-plugins.yml`、`scripts/check-architecture-deps.sh` | 加 `--all-features`、更新注释数字、去 harmony | 修改（Task 6）|
| `docs/superpowers/{specs,plans}/2026-11-15-...` | 设计 + 计划文档 | 提交（Task 6）|

## 已核实硬事实（执行者不得假设对称）

- 组件目录数：material 98、cupertino 99、fluent 99。每个 = `src/lib.rs` + `tests/<name>.rs`；**仅 `md-icon` 多 `src/icons.rs` 且 lib.rs 含 `mod icons;`**。
- 组件间交叉依赖**只有 icon / icon-paths**：material 17 个依赖 `floom-widget-md-icon`；cupertino/fluent 各 4 个依赖 `*-icon` 或 `*-icon-paths`。**material 没有 `md-icon-paths`**（路径数据内嵌 `md-icon/src/icons.rs`）。feature 依赖图必须按每个组件自己的 `Cargo.toml` 推导，不得三套对称套用。
- 组件代码零 `crate::`/`super::`/`include!`/`#[macro_export]`/`build.rs`（reviewer 已核实），可机械搬运。
- 薄 kit 自带 golden 测试：`floom-material/tests/{gallery.rs,motion.rs}`、`floom-cupertino/tests/gallery.rs`、`floom-fluent/tests/gallery.rs`；无组件叫 gallery/motion，不冲突；三个 `gallery.rs` 同名 → 各自留原目录、不扁平化。
- `[workspace.dependencies]` 只有 5 条，其中 3 条 icon path：`floom-widget-md-icon`、`-cupertino-icon`、`-fluent-icon`（第 345–347 行）；`*-icon-paths` 用相对 path，不在该表。
- `floom-widget-runtime` / `floom-core-theme` 各有 `material/cupertino/fluent` feature，OR 组合语义等价；合并后单 crate 无条件带本 DS feature。
- 消费者：`examples/floom-demo` 294 个组件依赖 + 294 处 `floom_widget_*` 引用；`floom-showcase` 3 个依赖 + 6 处引用。二者是根 workspace member，gate 会编译。

---

### Task 1: 校验脚本 + 生成脚本

**Files:**
- Create: `crates/ecosystem/floom-ui/scripts/verify_merge.py`
- Create: `crates/ecosystem/floom-ui/scripts/merge_kit.py`

**Interfaces:**
- Consumes: 无（读真实目录树与 Cargo.toml）。
- Produces: `python3 scripts/merge_kit.py <ds>`（ds ∈ {material,cupertino,fluent}），幂等生成 `crates/floom-<ds>/` 合并产物；`python3 scripts/verify_merge.py <ds>` 返回 0/1 并打印结构断言结果。后续 Task 只调这两个命令。

- [ ] **Step 1: 先写失败的结构校验（红线）**

`verify_merge.py <ds>` 断言（全部基于真实目录，不写死数量到代码外）：
  - `crates/floom-<ds>/Cargo.toml` 含 `edition = "2021"`、`default = []`、`full = [`；
  - 每个原组件 `<prefix>-<name>` 在 `[features]` 有对应 `<feat>` 条目、`src/widgets/<feat>.rs` 存在、`src/widgets/mod.rs` 有 `#[cfg(feature = "<feat>")] pub mod <feat>;`；
  - 组件 tests 数 == 原组件 tests 数，且每个以 `#![cfg(feature = ...)]` 开头；
  - **薄 kit 的 `tests/gallery.rs`（及 material 的 `motion.rs`）仍存在**；
  - 生成产物中无字面 `floom_widget_{md,cupertino,fluent}_`（引用已改写）；
  - 无 `crates/floom-<ds>/Cargo.toml` 引用已删的 `floom-widget-*-icon` path。

- [ ] **Step 2: 跑校验，确认 FAIL**

Run: `cd crates/ecosystem/floom-ui && python3 scripts/verify_merge.py material`
Expected: 非零退出，报告 `Cargo.toml`/`src/widgets/` 缺失（此时尚未合并）。

- [ ] **Step 3: 实现 `merge_kit.py`**，按下列确定性规则：
  - 前缀映射 material→`md`、cupertino/fluent→各自名；feature = 去前缀后 `-`→`_`（`md-radio-group`→`radio_group`、`md-text-field`→`text_field`、`cupertino-icon-paths`→`icon_paths`）。
  - 组件 `src/lib.rs` → `src/widgets/<feat>.rs`；`md-icon` 的 `icons.rs` → `src/widgets/icon/icons.rs`（保留 `mod icons;`）；`tests/*.rs` → kit 的 `tests/`，统一命名 `<prefix>_<feat>.rs` 避免冲突，顶部加 `#![cfg(feature = "<feat>")]`。
  - 引用改写**按文件类型分两套**（edition 2021 下 crate 不能以自身名 `floom_<ds>::` 引用自己，已实测确认）：
    - **组件 `src/lib.rs` → `src/widgets/<feat>.rs`**：跨组件引用 `floom_widget_<prefix>_<x>::` → `crate::widgets::<x>::`（如 `floom_widget_md_icon::MdIcon` → `crate::widgets::icon::MdIcon`）。
    - **组件 `tests/*.rs` → kit 的 `tests/`**：外部 crate 视角，引用 `floom_widget_<prefix>_<x>::` → `floom_<ds>::widgets::<x>::`（如 `floom_widget_md_button::MdButton` → `floom_material::widgets::button::MdButton`）。
  - `src/lib.rs` 保留薄 kit 4 个 `mod` + `pub use`，追加 `pub mod widgets;`；生成 `src/widgets/mod.rs` 每行 `#[cfg(feature = "<feat>")] pub mod <feat>;`。
  - `Cargo.toml`：`[package]` 照抄薄 kit 全部字段（`name`/`description`/`version.workspace`/`license.workspace`/`repository.workspace`/`rust-version.workspace`），**仅把 `edition.workspace = true` 改成写死 `edition = "2021"`**（否则丢元数据 + 继承 2024）；`[dependencies]` = 薄 kit 5 个 + 组件层并集去重、全部无条件，`floom-widget-runtime`/`floom-core-theme` 带 `features=["<ds>"]`；`[dev-dependencies]` = 并集；`[features]` 每组件一条（依赖从原 Cargo.toml 的 `floom-widget-*-icon*` 引用推导，如 `checkbox=["icon"]`）+ `full` + `default=[]`。
  - **绝不**删源目录、改 workspace `Cargo.toml`、覆盖薄 kit 的 gallery/motion 测试（这些是 Task 4 的独立原子步骤）。

- [ ] **Step 4: 跑校验，确认 PASS（此时产物已在 material 上生成）**

Run: `python3 scripts/verify_merge.py material`
Expected: 退出 0，各断言 OK。

- [ ] **Step 5: Commit**

```bash
git add crates/ecosystem/floom-ui/scripts/merge_kit.py crates/ecosystem/floom-ui/scripts/verify_merge.py
git commit -m "build(floom-ui): add per-component feature-kit merge + structure verifier"
```

---

### Task 2: 合并 material（打通模式）

**Files:**
- Modify: `crates/floom-material/Cargo.toml`、`crates/floom-material/src/lib.rs`
- Create: `crates/floom-material/src/widgets/**`、`crates/floom-material/tests/md_*.rs`
- Test: material 全部组件 tests + `tests/gallery.rs`、`tests/motion.rs`

**Interfaces:**
- Consumes: Task 1 的 `merge_kit.py material` / `verify_merge.py material`。
- Produces: crate `floom-material`，feature 集合含 98 个组件 feature + `full`；下游 Task 4 依赖其存在。

- [ ] **Step 1: 生成**

Run: `cd crates/ecosystem/floom-ui && python3 scripts/merge_kit.py material && python3 scripts/verify_merge.py material`
Expected: verify 退出 0。

- [ ] **Step 2: 断言 edition（Review Focus #3）**

Run: `grep -n 'edition = "2021"' crates/floom-material/Cargo.toml`
Expected: 命中一行；且 `cargo check -p floom-material`（default=[]）通过——刻意证明默认不编组件代码。

- [ ] **Step 3: 单组件裁剪编译（Review Focus #2）**

Run: `cargo check -p floom-material --no-default-features --features checkbox`
Expected: PASS（`checkbox=["icon"]` 生效，`crate::widgets::icon` 存在）。

- [ ] **Step 4: 全量测试（Review Focus #5）**

Run: `cargo test -p floom-material --all-features --no-fail-fast`
Expected: 98 组件 tests + `gallery` + `motion` 全绿。

- [ ] **Step 5: Commit**（精确 add：`crates/floom-material/`，不 add 源 `crates/material/`——留到 Task 4）

```bash
git add crates/ecosystem/floom-ui/crates/floom-material
git commit -m "feat(floom-ui): merge material components into floom-material with per-component features"
```

---

### Task 3: 合并 cupertino

**Files:** 同 Task 2 形状，目标 `crates/floom-cupertino/`。

**Interfaces:** Consumes `merge_kit.py cupertino` / `verify_merge.py cupertino`；Produces crate `floom-cupertino`（99 feature + `full`）。

- [ ] **Step 1: 生成 + 校验**

Run: `python3 scripts/merge_kit.py cupertino && python3 scripts/verify_merge.py cupertino`
Expected: verify 退出 0（`icon_paths` feature 存在、`tests/gallery.rs` 仍在）。

- [ ] **Step 2: 单组件裁剪编译**

Run: `cargo check -p floom-cupertino --no-default-features --features checkbox`
Expected: PASS（依赖 `icon` + `icon_paths`）。

- [ ] **Step 3: 全量测试**

Run: `cargo test -p floom-cupertino --all-features --no-fail-fast`
Expected: 99 组件 tests + `gallery` 全绿。

- [ ] **Step 4: Commit**

```bash
git add crates/ecosystem/floom-ui/crates/floom-cupertino
git commit -m "feat(floom-ui): merge cupertino components into floom-cupertino with per-component features"
```

---

### Task 4: 合并 fluent + 删 harmony + 收 workspace（原子）

**Files:**
- Create/Modify: `crates/floom-fluent/`（同 Task 2 形状）
- Delete: `crates/floom-harmony/`
- Delete: `crates/material/md-*`、`crates/cupertino/cupertino-*`、`crates/fluent/fluent-*`（全部组件源目录）
- Modify: `crates/ecosystem/floom-ui/Cargo.toml`（members 收敛至 ~9、删 3 条 icon path 依赖、删 harmony member 行）

**Interfaces:**
- Consumes: Task 2/3 的三个合并 crate；Task 1 的脚本。
- Produces: 收敛后的 floom-ui workspace；下游 Task 5/6 依赖无组件源 crate 与新 member 集合。

- [ ] **Step 1: 生成 fluent + 校验**

Run: `python3 scripts/merge_kit.py fluent && python3 scripts/verify_merge.py fluent`
Expected: verify 退出 0。

- [ ] **Step 2: 原子删除 + 收 workspace（一次性完成，不可拆提交）**

  1. `rm -rf crates/floom-harmony crates/material/md-* crates/cupertino/cupertino-* crates/fluent/fluent-*`
  2. `Cargo.toml` 删所有 `crates/{material,cupertino,fluent}/<prefix>-*` 与 `crates/floom-harmony` member 行；删 `[workspace.dependencies]` 第 345–347 行的 3 条 icon path 定义。
  - 依据（Review Focus #4）：组件里 `md-rating`/`md-list_group`/`md-checkbox_btn`/`md-select`/`md-expansion_panels`/`md-stepper`/`md-speed_dial`/`md-confirm_edit` 等用 `floom-widget-*-icon = { workspace = true }`；删源与删 workspace 定义必须同一步，否则 dangling。

- [ ] **Step 3: metadata 校验（Review Focus #4）**

Run: `cd crates/ecosystem/floom-ui && cargo metadata --no-deps --format-version 1 >/dev/null`
Expected: 退出 0，无 unresolved workspace 依赖报错；`cargo metadata --no-deps | grep -c '"name"' ` ≈ 9 个 member。

- [ ] **Step 4: 全 workspace 验证**

Run: `cargo test --workspace --all-features --no-fail-fast` 与 `cargo check --workspace`
Expected: 前者 293 组件 tests + 3 kit golden 全绿；后者（default=[]）能编译。

- [ ] **Step 5: Commit**

```bash
git add -A crates/ecosystem/floom-ui/crates crates/ecosystem/floom-ui/Cargo.toml
git commit -m "refactor(floom-ui): merge fluent, drop floom-harmony, collapse 306 members to ~9"
```
（`git add -A` 仅限明确列出的 `crates/` 子树与 `Cargo.toml`，不越界到根仓库的用户改动。）

---

### Task 5: 迁移 examples

**Files:**
- Modify: `examples/floom-demo/Cargo.toml`、`examples/floom-demo/src/**`
- Modify: `examples/floom-showcase/Cargo.toml`、`examples/floom-showcase/src/pages.rs`

**Interfaces:**
- Consumes: Task 4 的三个合并 crate（feature 名 = 去前缀目录名）。
- Produces: 可编译的 examples；根 `cargo check -p floom-demo -p floom-showcase` 通过。

- [ ] **Step 1: 先 grep 真实引用集合**（不套用文档假设）

Run: 分别统计 `examples/floom-showcase` 与 `floom-demo` 的 `floom_widget_{md,cupertino,fluent}_<x>` 不同 `<x>` 集合与所属 DS。
Expected: showcase = material `{text, button, checkbox}`；demo = 三 DS 全量。

- [ ] **Step 2: 改 Cargo.toml**

`floom-demo`：294 行组件依赖 → 3 行 `floom-material`/`floom-cupertino`/`floom-fluent`，各 `features = ["full"]`。
`floom-showcase`：3 行依赖 → `floom-material = { path = ..., features = ["text","button","checkbox"] }`（保留 path 指向 `crates/ecosystem/floom-ui/crates/floom-material`）。

- [ ] **Step 3: 批量改源码引用**

`floom_widget_<prefix>_<x>::` → `floom_<ds>::widgets::<x>::`（按 Step 1 的映射，覆盖 demo 的 294 处 + showcase 的 6 处）。

- [ ] **Step 4: 编译验证**

Run: `cd <root> && cargo check -p floom-demo -p floom-showcase`
Expected: PASS，无 `floom_widget_*` 未解析。

- [ ] **Step 5: Commit**

```bash
git add examples/floom-demo examples/floom-showcase
git commit -m "refactor(examples): migrate demo/showcase to merged floom-{material,cupertino,fluent}"
```

---

### Task 6: 门禁 / CI / 注释 + 全量验证 + 提交文档

**Files:**
- Modify: `scripts/gate.sh`（ecosystem_check 加 `--all-features`，注释「306 members」改真实数）
- Modify: `.github/workflows/ci-plugins.yml`（ecosystem-check 循环里 `cargo test --workspace --all-features --no-fail-fast`，改注释数字）
- Modify: `scripts/check-architecture-deps.sh`（DS 列表去 `floom-harmony`）
- Modify: `docs/PLAN.md` / `COMPONENT_PARITY.md`（**仅追加一行**注释指向本设计文档，不改正文）
- Create/Track: `docs/superpowers/specs/2026-11-15-...`、`docs/superpowers/plans/2026-11-15-...`
- Regenerate: `crates/ecosystem/floom-ui/Cargo.lock`

**Interfaces:**
- Consumes: Task 4/5 的收敛 workspace 与 examples。
- Produces: 门禁在 `default=[]` 下仍跑全部组件测试；文档入仓。

- [ ] **Step 1: 改门禁与 CI（Review Focus #1）**

`gate.sh::ecosystem_check` → `cargo test --workspace --all-features --no-fail-fast`；`ci-plugins.yml` 同。更新两处注释里的 member 数字为 Task 4 实际值。

- [ ] **Step 2: 断言「测试真的跑了」（Review Focus #1）**

Run: `cd crates/ecosystem/floom-ui && cargo test --workspace --all-features --no-fail-fast 2>&1 | grep -c 'test result: ok'`
Expected: 数量 ≥ 组件 crate 数（293 量级），而非仅 3 个 kit golden。对照再跑一次**不带** `--all-features` 的 `cargo test --workspace` 证明它测得少——两数不等即证明门禁加 flag 有效、未放水。

- [ ] **Step 3: 架构注释 + 重新生成 Cargo.lock**

`check-architecture-deps.sh` 去 harmony；`docs/PLAN.md`/`COMPONENT_PARITY.md` 顶部各加一行 `<!-- 架构已变更：见 docs/superpowers/specs/2026-11-15-... -->`；`cargo generate-lockfile --manifest-path crates/ecosystem/floom-ui/Cargo.toml`（或让 `cargo check` 自然更新）。

- [ ] **Step 4: 全量验证闸门（缺一不可）**

```bash
cd crates/ecosystem/floom-ui && cargo test --workspace --all-features --no-fail-fast   # 全绿
cargo check --workspace                                                              # default=[] 能编译
cargo check -p floom-material --no-default-features --features button               # 单组件裁剪
cd <root> && cargo check -p floom-demo -p floom-showcase                            # examples
bash scripts/gate.sh ecosystem                                                      # 门禁
```
Expected: 5 条全部退出 0。

- [ ] **Step 5: Commit（精确 add，勿卷入未提交的用户改动）**

```bash
git add scripts/gate.sh scripts/check-architecture-deps.sh .github/workflows/ci-plugins.yml \
        crates/ecosystem/floom-ui/docs/PLAN.md crates/ecosystem/floom-ui/docs/COMPONENT_PARITY.md \
        crates/ecosystem/floom-ui/Cargo.lock \
        docs/superpowers/specs/2026-11-15-floom-ui-feature-kits-design.md \
        docs/superpowers/plans/2026-11-15-floom-ui-feature-kits.md
git status --short   # 确认只暂存了上述文件
git commit -m "ci(floom-ui): gate ecosystem with --all-features; add feature-kit design + plan docs"
```

---

## 验证总闸门（全部满足才算完成）

1. `cargo test --workspace --all-features --no-fail-fast`（floom-ui）→ 293 组件 + 3 kit golden 全绿。
2. `cargo check --workspace`（default=[]）→ 能编译。
3. `cargo check -p floom-material --no-default-features --features button` → 单组件裁剪通过。
4. 根 `cargo check -p floom-demo -p floom-showcase` → examples 迁移正确。
5. `bash scripts/gate.sh ecosystem` → 门禁通过。

## 风险

- **门禁放水（最危险）**：默认全关 + 漏 `--all-features` = 假绿零测试。Task 6 Step 2 用「测试数对比」专门钉死。
- **feature 漏映射**：脚本按各 crate 自己的 Cargo.toml 推导 + `--all-features` 全编译兜底 + 单组件编译抓典型。
- **大 diff 不可逆**：删源集中在 Task 4 一次原子提交；每 Task 独立验证 + commit 兜底。
- **工作树污染**：开工前记录 `git status` 基线，全程精确 `git add`；未提交的用户改动一律不碰。
