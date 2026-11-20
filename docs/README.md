<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# Floom 文档

目标：**Rust 第一 GUI 库** —— 集竞品之长，高性能、低资源占用、开发友好、AI first。

这套文档只有五根支柱 + 一个证据账本。一个主题一个文件；能删的删，能合表的合表。
2026-11-20 之前那份 26 文件的文档集已归档到 `../floom-wip/2026-11-20-01-docs-consolidation/`（只搬不删）。

## 五根支柱

| 支柱 | 文档 | 一句话 |
| --- | --- | --- |
| 为什么是它 | [vision.md](vision.md) | 目标、抄谁的长处、护城河、明确不做什么 |
| 跑得动吗 | [architecture.md](architecture.md) | 分层、三套 ABI、渲染后端矩阵、线程与帧模型 |
| 快不快、省不省 | [performance.md](performance.md) | 体积 / CPU / 内存 / GPU / 帧时间 / 输入延迟的指标、现状、门禁 |
| 好不好用 | [developer-experience.md](developer-experience.md) | 单 CLI 全生命周期、热重载、devtools、模板、错误信息 |
| 好不好被 AI 用 | [ai-first.md](ai-first.md) | agent 读 / 改 / 跑 / 验四件事的契约与缺口 |

## 横切

| 文档 | 内容 |
| --- | --- |
| [progress.md](progress.md) | **执行台账**：6 阶段 18 步的退出判据 + 验证命令 + 当前状态表。改代码前先看这里 |
| [platforms.md](platforms.md) | 9 个 target 矩阵 + **每平台能力 binding 真相**（输入、IME、剪贴板、a11y、文件、安全区怎么过桥）+ 已定位缺陷 |
| [components.md](components.md) | 组件分层清单 + 设计系统一致性（token 违规是量化的）+ 组件契约 checklist |
| [plugins.md](plugins.md) | provider vtable 契约 + 22 个能力插件矩阵 + 缺口 |
| [renderer-backend-support.md](renderer-backend-support.md) | 每个渲染后端的能力矩阵（哪些画、哪些只计数）+ 已知不可实现项 |
| [roadmap.md](roadmap.md) | 1.0 的 13 条门槛、现状偏差、排序、风险、非目标 |
| [workspace-conventions.md](workspace-conventions.md) | 目录、命名、批次推进、暂存区纪律 |
| [release-policy.md](release-policy.md) | 版本承诺、semver 面、支持目标表、发布六步（**权威源**） |
| [examples.md](examples.md) | golden examples 表（被 `docs_parity.rs` 与 `run-golden-examples.sh` 读取，**不可改名**） |
| [api/](api/) | 稳定 API 面说明 |
| [gates/](gates/) | 逐平台 8 项实测账本（`pass`/`fail`/`n/a`，带逐行证据）。tag 1.0 时不允许任何 `fail` |
| [superpowers/](superpowers/) | 计划与性能基线 JSON（历史归档区） |

## 现状仪表盘（可核实，2026-11-20）

| 指标 | 数字 | 证据 |
| --- | --- | --- |
| 代码规模 | 1,864 个 `.rs` / 441,657 行 | `find` + `wc` |
| 测试 | ~5,934 个 `#[test]`，12 个 `#[ignore]` | `grep` |
| 图 | 38,933 节点 / 87,985 边 / 1,538 社区 | `graphify-out/GRAPH_REPORT.md`（已 gitignore） |
| headless 体积底线 | 485,904 B（≈0.5 MiB） | `just gate size` |
| 空闲 | 8 秒 2 帧，CPU 0.15 s（≈0.5% 核） | `FLOOM2_WINDOW_IDLE=1` 冒烟 |
| 性能基线 | 11 个 Criterion bench，10% 阈值，能绿能红 | `scripts/check-perf-budgets.sh` |
| 平台 | 1 tested（`aarch64-apple-darwin`）/ 8 gated | [release-policy.md](release-policy.md) targets 表 |
| 平台门禁 | macOS 8/8 pass；Android 4 pass、1 partial；iOS 3 pass、1 partial、1 fail；OHOS 2 pass；Linux/Windows 全 gated | [gates/](gates/) |
| 插件 | 22 个，0 个带 ABI 版本串 | 本仓 grep |
| 设计系统 | material/cupertino/fluent 组件里 26/29/27 处硬编码颜色 | 见 [components.md](components.md) |

## 读法

- **写应用的人**：`README.md` → `examples/floom-showcase` → [developer-experience.md](developer-experience.md)。
- **写组件的人**：[components.md](components.md) → [architecture.md](architecture.md) 的 layering。
- **接平台后端的人**：[platforms.md](platforms.md) → [architecture.md](architecture.md) 的「三套 ABI」→ [plugins.md](plugins.md)。
- **agent**：`../llms.txt` 是入口，本文件是地图，[ai-first.md](ai-first.md) 是契约。
- **要 tag 1.0 的人**：[roadmap.md](roadmap.md) → [release-policy.md](release-policy.md) 的六步。

## 文档纪律

1. 一个主题一个文件。写进已存在的文件，不新开近重复文件。
2. `examples.md`、`architecture.md`、`release-policy.md` 被 `crates/app/floom/tests/docs_parity.rs` 与 `scripts/` 读取，**改名会红**。其它文件可自由合并。
3. 数字必须带证据位置（命令、文件:行、基线 JSON）。没有证据的"已支持"要写成"声明"。
4. 新的战略/状态文档用中文；对外契约（architecture、release-policy、api）保持英文。
5. 删除文档走 `../floom-wip/<批次>/` + `MANIFEST.md`，不直接 `rm`。
