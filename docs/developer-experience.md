<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# 开发友好

判据只有一条：**改一行，两秒内看见结果；出错时知道下一步点哪里。**

## 单 CLI 全生命周期

`floom` CLI 在 `crates/app/floom-cli`。对标 nsh 的 `nsh test --trace`、tauri 的 `tauri dev`、makepad 的 `cargo-makepad`。
子命令在 `crates/app/floom-cli/src/main.rs:103-109`：`run` / `create` / `init` / `check` / `test` / `package`。

| 子命令 | 作用 | 状态 |
| --- | --- | --- |
| `floom create` / `init` | 从模板建工程（`--template app`或`plugin`、`--style rust`或`tsx`、`--platforms`） | 有，模板真实可用 |
| `floom run` | 构建 + 启动 | 有 |
| `floom run --watch` | 监听文件，改动后重建并重启 | 有（**文件级热重启**，非状态保留） |
| `floom check` | 转发 `cargo check`（带 `--features`） | 有，但**只是转发**：不跑门禁子集、不查文档漂移 |
| `floom test` | 转发 `cargo test` | 有，但**只是转发**：不输出 ops/mount/nodes 级 trace |
| `floom package` | 按平台打包（`--platform <target>`，`pack::pack`，自包含，不依赖仓内 `scripts/build.sh`） | 有，真实实现 |
| `floom doctor` | 环境自检（GPU、字体、SDK、设备） | **缺** |

所以 G5 的真实缺口不是"命令没有"，而是 **`check`/`test` 还是空壳**：没有把门禁子集、`docs_parity`、snapshot 与 trace 串进去；以及没有 `doctor`。这是"开发便利 3 分 → 5 分"的主要差距。

## 热重载

- 现状：`floom run --watch` 是**文件级热重启**——进程重启、状态丢失（`crates/devtools/floom-hot-reload`）。
- 明确不做：进程内保留状态的热补丁（等于重做 `dioxus-hotpatch` 那一层，1.0.x 之后再谈）。
- 延迟纪律：改一次文件到窗口重绘的时间必须**记录下来**，进 release notes；不测就不许说"快"。
- 已知限制：重启期间窗口会闪一下；watch 只覆盖 `src/`。

## Devtools

`floom-devtools` 的检查器是 Chrome 布局的：树 + 标签栏 + picker（鼠标点选元素、高亮盒子、看到自己那一行 style）。

- 已有：树、picker、逐帧 `budget_exceeded`、分配计数、`PerformanceSnapshot`、语义树视图。
- 缺口 G7：Style 行只显示**声明值**，没有计算值；缺 hover 高亮。
- 缺口（接真实窗口）：性能画像停在 headless 协议，`app wiring pending`。
- 快捷键接线在事件循环里，不在树里——这是有意的（面板不能吃掉应用的按键）。

## 模板与示例

- `floom new` 模板走 `crates/app/floom-cli`，模板仓库里带最小脚手架 + 一个按钮 + 一个 `cargo test`。
- golden examples 见 [examples.md](examples.md)（8 个，其清单被 `docs_parity.rs` 与 `scripts/run-golden-examples.sh` 双向校验）。
- `floom-showcase` 是 `/components` 页挂载全部 Tier-0 组件的**唯一**"设计回归"入口。

## 错误信息

要求（当前部分满足）：

1. 编译期：feature 组合错误给出"启用哪个 feature"的建议（`feature_graph.rs` 的报错已做到）。
2. 运行期：能力缺失报"这个平台支持什么"，不是 `Option::None`。
3. 平台侧：`PlatformCapabilities` 声明与实现不符是**故意可见的 bug**，不允许用 `unwrap_or_default()` 掩盖（见 [platforms.md](platforms.md)）。
4. 测试失败：`just gate` 的 skip 必须打印原因，`SKIP ≠ PASS`。

## TSX / DSL / 编辑器

- `floom-tsx`：TSX→Rust 前端。**注意**：`llms.txt` 曾声称发布了 `npm/@types/floom/index.d.ts`，仓库里没有 `npm/` 目录 —— 要么补，要么删，不允许文档先说。
- `floom-dsl`：`codegen` + `ui_ir` 两个模块；IR 是 agent 与热重载的共同底座。
- `floom-editor`：卫星 crate，富文本/代码编辑，不在 1.0 锁定面。

## 目标数字（要写进 release notes 的）

| 项 | 目标 |
| --- | --- |
| 文件改动 → 窗口重绘 | ≤ 2 s（`--watch`，中位） |
| 冷启动首帧 | 记录并逐版对比（当前未测） |
| `floom check` 全绿时间 | ≤ 60 s（中等工程） |
| 从零到跑起来的命令数 | 1（`floom new x && cd x && floom run`） |
