<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# 性能与资源

本库的硬性要求不是"跑分"而是**预算**：每个数字都有定义、有当前值、有门禁、有证据位置。
低于预算才允许发布；下降必须有书面理由，不能靠删功能达成。

## 指标台账

| # | 指标 | 定义 | 当前值 | 门禁 | 证据 |
| --- | --- | --- | --- | --- | --- |
| P1 | headless 体积底线 | 空 floom 应用 release 二进制 | **485,904 B（≈0.5 MiB）** | `just gate size`，3 档各 10% | `docs/superpowers/benchmarks/floom-2-size-baseline.json` |
| P2 | 实际应用体积 | showcase / 带窗口 demo | `floom-showcase` 11.9 MiB；`floom-window --features real` 11.1 MiB | 同上 | 同上 |
| P3 | 默认 feature 图 | 默认不链窗口/GPU/工具 | 由 `feature_graph.rs` 守 | `cargo test -p floom --test feature_graph` | `crates/app/floom/tests/feature_graph.rs` |
| P4 | 空闲帧数 | 无输入 8 s 内的重绘次数 | **2** | 冒烟测试，人工比对 | `FLOOM2_WINDOW_IDLE=1 FLOOM2_WINDOW_IDLE_SECS=8 cargo run -p floom-window --features real` |
| P5 | 空闲 CPU | 同上区间的 CPU 时间 | **0.15 s（≈0.5% 核）** | 同上 | 同上 |
| P6 | RSS / 峰值内存 | 进程常驻与峰值 | **未测** | **无** | — |
| P7 | GPU 时间 / 显存 | 每帧 submit→present | **未测** | **无** | — |
| P8 | 逐帧分配 | 稳定态每帧 heap 分配次数 | 稳定态帧 **≤8 次分配**，到 100k 节点不随节点增长（10/100/1k/10k 组 × 10 叶） | 有（单测断言） | `crates/app/floom-app/tests/allocation.rs` |
| P9 | 帧时间 | 热基准 median，10% 阈值 | 9 条预算 best-of-N 全绿，能绿能红 | `scripts/check-perf-budgets.sh` | `floom-2-budget-baseline.json` |
| P10 | 首帧 | 启动→首次绘制 | 2,000 节点树首帧 **≤1 帧**内全部绘制，之后无欠账（本机 6.4 ms，无头） | 有（帧数） | `crates/app/floom-app/tests/latency.rs` |
| P11 | 输入延迟 | tap → 画面提交 | tap **不需要额外帧**：点击自身的那一帧就把新状态画进场景（本机 308 µs，无头）；提交后 `needs_frame` 与 damage 均为空 | 有（帧数） | `crates/app/floom-app/tests/latency.rs` |
| P12 | 损伤面积 | 一帧内脏矩形占比 | `DamageRegion` ≥8 区溢出为整帧；`SceneFrameQueue` 有界、丢最旧 | 隐含在 P4 | `crates/core/floom-scene` |

**判据**：P1–P5、P8–P11 已有门禁；P6（RSS）、P7（GPU/VRAM）是 1.0 的必达缺口（G12）。P12 只有机制，没有量化。

## 基准清单（Criterion，9 组预算 + 11 个 bench 目标）

| crate | bench | 测什么 |
| --- | --- | --- |
| `floom-layout` | `incremental` | 子树变更后的重排 |
| `floom-runtime-core` | `element_arena` | 元素查找与脏传播 |
| `floom-virtual` | `scroll` | 可视区计算 |
| `floom-text` | `shaping` | 文本整形 |
| `floom-renderer-software` | `self_draw` | CPU 光栅化 |
| `floom-renderer-vello` | `scene_render` | GPU 场景渲染 |
| `floom-native` | `rows` | 原生视图槽复用 |
| `floom-signal` | `selector` | 选择器订阅成本 |
| `floom-accessibility` | `tree` | 语义树更新 |

纪律：基线是**单主机**数字（macOS / Apple M4 Pro）。异构 runner 上比绝对时间会假红，所以热基准不进 CI，进的是"固定参考机 + 归档数字 + 人工复核"。`SKIP` 不算 `PASS`。

## 竞品对照（官方自报，未在同机复测）

| 项目 | 体积 | 空闲 / CPU | 内存 | 性能 |
| --- | --- | --- | --- | --- |
| BUI | 单文件 6.3–6.9 MB（按 features 裁剪） | idle 0% CPU，hover 只重画碰到的 widget | 图片缓存预算 256 MiB | 稳定帧 0 分配；1000 缩略图 p99 2.9 ms / 100 MB；~5 ms 首帧 |
| nsh.dev | Web ~1.1 MB；Rust 版 12.4–16.5 MB | 引擎启动 75–155 µs | macOS ~19 MB（裸窗 17 MB） | 120 FPS；p95 <4 ms；tap→commit ~0.12 ms |
| slint | runtime <300 KiB RAM | 嵌入式导向 | 极低 | AOT，可纯 CPU |
| tauri | ~600 KB（借系统 WebView） | 前端 HMR | 借 WebView | 由前端决定 |
| dioxus | Web hello ~50 KB；桌面 <3 MB | — | — | VDOM 粗粒度 |
| makepad | `--strip / --wasm-opt / --brotli / --profile=small / --no-threads` | — | — | 自研 GPU 2D/3D |
| flutter | 引擎自带（大） | Impeller 预编译 shader | — | stateful hot reload ~978 ms |

读法：floom 的 P1（0.5 MiB）与 nsh(1.1 MB)/dioxus(<3 MB)/BUI(6.3 MB) 同量级或更优；P4/P5 是 BUI "idle 0% CPU" 的可测等价物；P9 是本清单里最严的性能证据。
**差距不在引擎，在"预算没接线"**：P6、P7 全缺；P8、P10、P11 已有帧级/分配级门禁（见上表）。

## 命令

```sh
just gate size                        # P1/P2，3 档 10%
just gate perf                        # P9，读基线 JSON
./scripts/check-perf-budgets.sh       # 直接跑热基准
FLOOM2_WINDOW_IDLE=1 FLOOM2_WINDOW_IDLE_SECS=8 \
  cargo run -p floom-window --features real   # P4/P5
```

## 已知会伤害预算的实现缺陷

- **OHOS 上屏路径**：`RequestBuffer` 之后的早退不 `AbortBuffer`（队列泄漏）、失败仍 `Flush`（脏帧上屏）、三个 atomic 非成组（与 release 竞争）、`stride` 单位按字节用。详见 [platforms.md](platforms.md#已定位缺陷)。
- **iOS 汉字渲染成豆腐**：`parley`/`fontique` 在 iOS 找不到汉字字体，OHOS 有 `ohos_font.rs` 同款修复。这是"渲染 bug"里唯一会让人误判为性能问题的（观感黑块）。
- **组件硬编码颜色**：不影响帧时间，但让主题切换与暗色模式产生整树重绘（因为 token 变了而组件不读 token）。

## 待办（对应 G11–G13）

1. `just gate resources`：RSS / 峰值 / 逐帧分配三类基线与阈值（P6、P8）。
2. GPU 时间与显存采集（P7）——先做能出数字的探针，再谈门禁。
3. ~~首帧与输入延迟基线（P10、P11）~~ → 已用**帧数**而非毫秒落地（`tests/latency.rs`）：毫秒数在无头 runner 上不可持守，帧数跨机器成立；真机毫秒仍待设备。
4. `floom-devtools` 的 `budget_exceeded` / 分配计数从 headless 协议接到真实 window loop（当前 app wiring pending）。
5. 体积第 4 档（desktop + software 组合）。
6. 发布记录表：每次发布写六类数字（体积 / 空闲 CPU / 内存 / GPU / 帧时间 / 分配）。
