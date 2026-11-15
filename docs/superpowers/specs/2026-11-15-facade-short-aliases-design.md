<!-- SPDX-License-Identifier: MIT -->
# Facade 短名别名：全 workspace crate 统一 `floom::xxx`

2026-11-15。

## 意图

`floom` 门面当前只把 6 个 crate 以全名导出（`floom::floom_core`），
用户要求把**整个根 workspace** 的所有库 crate 统一以短名导出
（`floom::core`、`floom::widgets`、`floom::signal` …），用别名实现。

## 目标 / 非目标

- 目标：根 workspace 每个有 `[lib]` 的 `floom-*` crate 都能从 `floom::<短名>` 访问。
- 目标：保留现有全名导出与所有既有 API，零调用点改动（代码里 73 处 `floom::floom_xxx` 不动）。
- 非目标：satellite workspace（`floom-ui` 9 个、plugins 22 个、
  `crates/ecosystem/floom-editor`）——它们被根 `exclude`，门面顶部注释明确
  禁止 app 二进制依赖 satellite。
- 非目标：改动 feature 图语义或取消 headless floor。

## 约束（为什么不能"一行全挂"）

1. **命名冲突**：`floom::platform`、`floom::devtools` 已是手写 `pub mod`
   （`platform::require`、`devtools::attach`）。不能改成 crate 别名，否则
   破坏既有 API。做法：在这两个模块内补 `pub use floom_platform::*;` /
   `pub use floom_devtools::*;`，把 crate 内容并进去，而非替换模块。
2. **feature_graph.rs**：`default_features_are_headless_and_backend_free` 断言
   默认构建里不出现 `floom-renderer-software` 等 backend，也不出现子串
   `winit`（断言用 `graph.contains` 子串匹配）。`floom-accuracy`、
   `floom-snapshot`、`floom-testkit` 把 `floom-renderer-software` 作为 normal
   依赖；`floom-ui-events-winit` 整条子树是 vendored `floem-winit`。若把它们
   设为 facade 的**默认**依赖，会拉 backend 进默认图、推翻 headless floor、
   破测试。
   做法：默认依赖只加"不拉 backend/winit"的 crate；这 4 个设为 `optional`，
   统一挂 `full` feature，别名 `#[cfg(feature = "full")]` 门控。
3. **bin-only**：`floom-cli` 无 `[lib]`，不能 `pub use`，跳过。
4. **架构门禁**：facade 是 L9，`UPWARD` 检查对它放行（聚合全栈即其定义），
   `FACADE_ALLOW` 也已 allowlist。新增依赖不会产生新的 FORBIDDEN 边。
5. **循环依赖**：根 workspace 内无 crate 依赖 facade（仅 satellite
   `floom-editor/examples/*` 依赖，不在根图里），无环。

## 方案（选定）

- `Cargo.toml`：根 workspace 每个库 crate 补 `workspace = true` 依赖；
  按"是否拉 backend"分为默认依赖与 optional+feature 两组。
- `lib.rs`：每个 crate 加 `pub use floom_xxx as <短名>;`（短名 = 去 `floom_`
  前缀）；feature 门控的加对应 `#[cfg]`；`platform`/`devtools` 用 glob 并入
  现有模块。
- 保留全部现有 `pub use floom_xxx;` 全名导出，不删。
- 不改 `feature_graph.rs` 断言（保持 headless floor 为真）。

## 验证

`cargo check -p floom`（默认）、`cargo check -p floom --all-features`、
`cargo test -p floom --test feature_graph`、`bash scripts/check-architecture-deps.sh`、
`cargo check --workspace --all-targets --all-features`。

## 未做

- 不迁移现有调用点改用短名（兼容优先，后续可单独做）。
- 不验证 satellite workspace 内的等价 API。
