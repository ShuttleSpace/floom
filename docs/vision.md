<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# 目标：Rust 第一 GUI 库

"第一"不是下载量，是四条同时成立：

1. **性能与资源**：帧时间、空闲 CPU、内存、体积，在同类（自绘、保留模式、GPU）里最好或并列最好。
2. **平台覆盖**：≥3 个 target 有 CI 实测（含 ≥1 移动端），其余诚实标 experimental。
3. **开发友好**：一条 CLI 覆盖 init→run→check→test→package；改代码 <2s 看见结果；错误信息带修复建议。
4. **AI first**：agent 能读、能改、能跑、能验 —— 不是能写文档，是能把改动闭环。

一票否决项：为上述任何一条牺牲正确性、无障碍或安全。

## 抄谁的长处（每条都对应本仓的一个具体决定）

| 来源 | 值得抄的 | floom 的决定 | 状态 |
| --- | --- | --- | --- |
| BUI | 损伤渲染、稳定帧 0 分配、按用量裁剪、缓存预算 | `DamageRegion` + 有界 `SceneFrameQueue` + `just gate size` 3 档 | 损伤渲染有；0 分配/缓存预算**无** |
| nsh.dev | 只更新读了该 signal 的节点、perf trace 可量化、启动 75–155 µs | 细粒度响应式 + `floom-devtools` 逐帧快照 | 响应式有；trace 未接真实窗口 |
| slint | <300 KiB 运行时、no-std、AOT | no-std/embedded feature 臂、headless floor 485,904 B | 有臂；未实测 RAM |
| tauri | ~600 KB 二进制、不带 runtime、插件市场 | 默认 feature graph 不链窗口/GPU/工具（`feature_graph.rs` 守） | 有；无插件市场 |
| dioxus | 默认 feature 图小、headless 测试 | `docs_parity.rs` + snapshot 测试 | 有 |
| makepad | `cargo-makepad --strip/--brotli/--profile=small` 按用量裁剪 | `floom` CLI 的 `package` 子命令（**未实现**） | 缺 |
| freya | 增量渲染、Rope 富文本、tree-sitter 虚拟化 | 保留 scene + `floom-virtual` | 有 |
| rune | headless 数值化验证、agent 自验 | `floom-snapshot` + `floom-accuracy` agent 模块 | 部分 |
| flutter | CI 版本矩阵、DevTools performance、llms+MCP | `just gate` + `floom-devtools` + `llms.txt` | 部分 |
| xilem/masonry | view diff 最小化更新 | 元素树 + 脏传播 | 有 |
| gpui/gpui-kit | hybrid immediate+retained、75+ 组件 | 保留模式 + 设计系统三套 | 有 |
| rinf | 零拷贝、零隐藏线程 | 帧队列有界、无后台隐藏线程 | 有 |
| [tessera](https://github.com/tessera-ui/tessera) | 文件与代码组织：`src/name.rs` 禁用 `mod.rs`、每层一个职责 crate、`AGENTS.md` + 随 CLI 分发的 skill、错误码各有文档、依赖版本策略写进 `Cargo.toml` 注释、`scripts/check-imports.rs` 拒跨设计系统引用 | 29 个 `mod.rs` 全部改成 `name.rs` + `just gate module-paths`；新增根 `AGENTS.md`；`Cargo.toml` 补依赖策略注释 | 组织与注释已抄；`AGENTS.md`/门禁**新加**；随 CLI 分发的 skill、错误码文档**无** |

## 六维现状（1–5，推断；来自 2026-11 竞品评估，已归档）

| 维度 | floom | 5 分档选手 | 决定性问题 |
| --- | --- | --- | --- |
| ① 平台覆盖 | **2**（1 tested / 8 gated） | flutter、tauri、nsh、slint、rinf（均靠逐平台 CI 实测） | 这是"能不能叫工业级"的分水岭，且只靠补 CI + 设备，不改架构 |
| ② 渲染成熟 | 4 | flutter、freya | 已有 vello/software/web/tui + GPU 像素回归 |
| ③ API 丰富度 | 4 | flutter、freya、gpui-kit | Tier-0 + composites + 三套 DS |
| ④ 开发便利 | 3 | flutter、tauri、nsh、dioxus、freya、gpui-kit | 三者共享：`llms.txt`（已有）、单 CLI（缺 package/check/test）、热重载（有热重启） |
| ⑤ 扩展能力 | 5 | （floom 并列最强） | 22 插件 + provider vtable，缺 ABI 版本串 |
| ⑥ 生态地基 | 3 | flutter、tauri、slint | 治理最细但仍是 0.2.0；无 1.0 版本承诺 |

对照 [tessera](https://github.com/tessera-ui/tessera)（285 star，Apache-2.0，2025-05 起，同为 wgpu + 声明式 + 响应式 + 移动端）：它的 ① 与我们同档（有 `crates/tessera-mobile` + `cargo tessera` 打包，但同样没有逐平台 CI 实测），② 与我们同档（自绘 + 自维护 glyphon/cosmic-text fork），③ 领先（`md3` 全套 + `experiment::hig`），④ 领先（`cargo tessera` 的 dev/check/package/headless + 随 CLI 发 skill），⑤ 弱于我们（插件是 crate 内 feature，不是独立 vtable 插件），⑥ 领先（v3.0.0 已发、双许可、NOTICE、`pr-title.yml`）。**它的优势全部在"人读得懂、agent 改得动"这一层**，所以本仓的对应动作不是加功能，而是补组织与约定。

**结论**：③⑤ 已是一线，② 靠证据而非体感，①④⑥ 都是工程接线，不是重做引擎。

## 护城河（不要为了对标而放弃）

- **自绘 + 无浏览器依赖**：外观一致、无 WebView 内存与安全面。
- **细粒度响应式**：视图树只建一次，更新按 signal 精确传播。
- **一等插件**：能力插件是可组合的 crate，不是运行时字符串。
- **治理最硬**：13 条门禁、`skip ≠ pass`、基线能绿能红。

## 非目标

- 系统原生控件 / WebView 路线（那是 tauri/relm4 的地盘）。
- 即时模式（egui 的地盘）：与"只重画脏区"根本冲突。
- MCU 级资源上限（lvgl 的地盘）：不同量级，只借"上限意识"。
- VDOM 粗粒度更新（dioxus/leptos 的地盘）。
- Tailwind/CSS 样式层、签名沙箱动态模块、6 语言绑定、ImGui 兼容 —— 全部 1.0.x 之后。
- 把 `floom-ui` / `floom-editor` 纳入 core 锁定面。
- 为压数字而删渲染后端、降级计数或无障碍语义树。

## 命名

crates.io 上 `floom` 已被无关项目占用（1.0.0，2023-11-23，1,543 下载）；`floem` 是本项目（0.2.0，46,812 下载）。
可用名：`floom-gui`、`floom-ui`、`floom-app`、`xfloom`。发布前必须先在 [release-policy.md](release-policy.md) 里定下这一项。
