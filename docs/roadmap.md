<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# Roadmap 到 1.0

目标见 [vision.md](vision.md)。本文件只回答：**还差什么、按什么顺序、什么条件算完成。**

## 1. 十三道门槛与现状

`必达` = 不满足不许 tag 1.0；`1.0.x` = 可以 1.0 之后再补。

| # | 门槛 | 级别 | 现状 | 差距 |
| --- | --- | --- | --- | --- |
| G1 | 逐平台 CI 实测（≥3 tested，含 ≥1 移动端） | 必达 | ✗ 1 tested / 8 gated | 见 §2 |
| G2 | 版本承诺 + 破坏性变更政策 | 必达 | ✗ 0.2.0 | semver 面写死、breaking 流程 |
| G3 | 许可尽调 | 必达 | ✓ MIT | — |
| G4 | `llms.txt` | 必达 | **✓ 已有**（旧清单标 ✗） | 有 1 条死链：`npm/@types/floom/index.d.ts` |
| G5 | 单 CLI：init / check / test / package | 1.0.x | △ 命令都有，`check`/`test` 只是 cargo 转发；无 `doctor` | 把门禁子集、`docs_parity`、snapshot、trace 串进去 |
| G6 | 热重载 | 1.0.x | △ 文件级热重启，无状态保留 | 记录延迟并进 release notes |
| G7 | DevTools 全平台 | 1.0.x | △ Style 行无计算值、无 hover 高亮；性能画像停在 headless | 接真实 window loop |
| G8 | 插件 + ABI 版本 | 必达 | ✓ 领先（22 插件 + vtable） | 加 `abi_version` 字段；webview JS→Rust |
| G9 | 可视化回归进 CI | 必达 | △ GPU 像素测试仍 `#[ignore]` | 进 CI |
| G10 | 生态钉版 | 必达 | △ 兼容矩阵在 `release-policy.md` 的 targets 表（逐个标 tested/device-gated）；docs 索引已有（`README.md`）；`flameo` 从未存在于本仓库，声明已删 | 剩：生态 crate 钉版本号 |
| G11 | 体积 | 必达 | △ 3 档 10% 已落地 | 第 4 档（desktop+software）、发布记录表 |
| G12 | 资源（CPU/内存/GPU/分配） | 必达 | △ 只有空闲 frames/CPU | RSS、GPU、逐帧分配预算 |
| G13 | 渲染性能 | 必达 | △ 11 bench + 10% 脚本能绿能红 | 帧预算接真实 loop、数字进发布记录 |

细则：体积/资源/性能的指标定义与当前数字见 [performance.md](performance.md)；平台状态权威源是 [release-policy.md](release-policy.md)；逐平台证据在 [gates/](gates/)。

## 2. 与现状不符的文档声明（先修文档，再谈进度）

| 声明 | 实际 |
| --- | --- |
| "CI 门禁在跑" | origin 是 Gitee，`.github/workflows/` 从不执行；本地 `just gate` 才是真门禁 |
| "`check`/`test` 是完整生命周期" | 两个子命令只是 `cargo` 转发 |
| "`npm/@types/floom` 已发布 898 行类型" | 仓库无 `npm/` 目录 |
| "渲染后端有 vger / Skia-anyrender / tiny-skia" | 实际是 `-software`、`-vello`、`-web`、`-tui`；Skia 只是 `floom-renderer::skia_abi` 录制模块 |
| "没有 Clip / Group 无实现"（旧 motion 文档） | `floom-scene` 已有 `ClipPush/ClipPop`、`GroupPush{transform}/GroupPop` |
| "iOS/OHOS 是 `floom-platform` 的 feature" | 已是独立 crate |
| "camera 能力不存在" | 插件存在且四平台有原生 |

## 3. 顺序（按阻塞关系，不按好听程度）

**P0-1 让 CI 真的会跑。** 把仓库镜像到 GitHub 并推 workflow，或改用可访问的 runner。退出判据：一条公开的绿 build（macOS + Linux + Windows 三系统 `cargo test`）。
在这之前 G1 全阻塞，任何"已支持"都只是本地事实。

**P0-2 平台状态从 1 tested 推到 ≥3 tested（含 ≥1 移动端）。** 优先 macOS→Linux→Android（Android 已是 4 pass / 1 partial，边际成本最低）。退出判据：`release-policy` targets 表三行改 `tested`；对应 `gates/*.md` 无 `fail`。

**P0-3 OHOS 后端选型。** 二选一，不许再并行维护两个半成品：`floom-winit-ohos`（有事件循环/输入/IME，但需解 N-API 单模块约束）或 `floom-platform-harmony`（只有上屏，需补 ~5 个 C 导出 + 事件注入）。退出判据：写进 [platforms.md](platforms.md) §4 的一个决定 + 一条能收 touch 的路径。

**P1-1 插件 `abi_version`。** 修很小，拖着就锁不住 vtable。退出判据：旧 provider 在运行期被拒绝并给出"重新编译插件"的错误。

**P1-2 资源门禁（G12）。** RSS / 逐帧分配 / GPU 时间三类基线与阈值。退出判据：`just gate resources` 能绿能红。

**P1-3 组件 token 门禁。** 清零 82 处硬编码颜色（26/29/27），再上 `just gate style`。退出判据：门禁存在且绿。详见 [components.md](components.md)。

**P2 生态（G10）与 1.0.x 三项（G5/G6/G7）。** 兼容矩阵、`flameo`、docs 索引；CLI 补 `doctor` 与真实 `check`/`test`。

## 4. 风险

| 风险 | 触发信号 | 应对 |
| --- | --- | --- |
| CI 建不起来 | P0-1 两周无绿 build | 降级为自托管 runner；在此之前不许宣传"CI 门禁" |
| 设备 flaky | 同一 commit 时红时绿 | 假红记为 `n/a` 并写原因，不静默重试 |
| OHOS/移动端缺口爆炸 | 平台门禁 #2–#7 长期 fail | 保留"明确降级 experimental"的诚实出口，别为 tag 假装 pass |
| 范围爆炸 | 又开始讨论 6 语言绑定/Tailwind 样式层 | 回到 [vision.md](vision.md) 的非目标 |
| 基线漂移 | 基线文件被移出仓库（曾发生） | 基线跟代码同 PR，`exit 2` 分支守 |
| 单主机基线假红 | 异构 runner 比绝对时间 | 热基准不进 CI 是有意的；进的是归档数字 + 人工复核 |

## 5. 不做

见 [vision.md](vision.md#非目标)。一句话：不为压数字删功能、不把 `floom-ui`/`floom-editor` 纳入锁定面、不做 WebView/即时模式/VDOM 路线。
