# Floem 2.0 进度盘点（2026-09-17）

主计划 `docs/superpowers/plans/2026-09-02-floem-2-complete-implementation-plan.md`
Phase 16 计划 `docs/superpowers/plans/2026-09-15-phase-16-native-composition-plan.md`

---

## 一、本轮完成的工作

### 16.4 快照过渡（steps 1–2，headless）

| 提交 | 内容 |
| --- | --- |
| `12757bf` | `freeze_native` / `thaw_native`：把参与者冻成纹理、藏起活视图、由 widget 画纹理 |
| `c80da2e` | `SceneNode::Image` 补 `alpha`（顺手修的既有缺陷，见下） |
| `4d0ab28` | `begin_native_transition`：冻结纹理跑动画，结束自动解冻 |
| `79b9590` | 台账 / 设备表 / 计划文档同步 |

关键设计决定：

- 第四个共享名词 `NativeSnapshot` 从 `floem-native`（L4）下沉到 `floem-types`（L0）——
  **绘制**冻结纹理是 widget 的活，L6 不能命名 L4 的类型。16.1 没预见到这点。
- 动画复用 R2 的通道（`AnimationScheduler` → `CompositorProp` → `paint_node`），
  **不造第二套机制**；过渡结束**自己 thaw**，不交给调用方（它可能已经滚走）。
- provider 不能快照 ⇒ `Ok(None)`，不是错误，且**不启动任何动画**。
- host 侧不重复 controller 的不变量（我加的 `AlreadyFrozen` 被变异测试证明没用 → 删）。

新增测试：`app/floem-app/tests/native.rs` +8（6 条冻结/解冻、2 条过渡）。
全部经 `scripts/mutate.py` 变异验证：step 1 四个变异各杀一条、step 2 两个变异各杀一条。

### 顺手修的既有缺陷：scene 图像没有 alpha

`apply_composite` 一直跳过 `Image` 节点，注释称"和 `Scale` 同样的结构性原因"。
只有 `Scale` 是结构性的（逐节点缩放会把子树拽开）；图像只是**没地方放 alpha**。
后果：含图片的浮层淡入时，底色变了、图片不变。

`SceneNode::Image` 现在带 `alpha: u8`；GPU 指纹**必须包含 alpha**（否则复用旧片段 =
缓存版的"视频冻住"）；vello 0.7 没有 per-draw alpha，用 `push_layer` 包一层绕开，
不透明时跳过图层。软件渲染器与 skia 同步。

### 顺手完成：推送与计划补漏

- 62 个未推提交全部推送，`origin/floem-2.0` 与本地一致（当前落后 0）。
- 主计划新增「附录——计划外已完成的工作」，把 R1–R6 与 Phase 16 的实际产出记进去。

---

## 二、四判据

| # | 判据 | 结果 |
| --- | --- | --- |
| 1 | 有会红的失败测试 | 本轮 9 个变异全部被杀 |
| 2 | `cargo test --workspace --no-fail-fast` | **EXIT=0，240 targets / 1642 tests / 0 failed** |
| 3 | `scripts/check-architecture-deps.sh` | **PASS (no new forbidden edges)** |
| 4 | Metal 真窗冒烟 | Apple M4 Pro (Metal)，900 帧，无 panic |

---

## 三、规划与进度

### 主计划：237 / 486 步（49%）

原报的 220/465 失真，两处原因已修正：

- Phase 16 与 Phase 31 用「Files / Interfaces」小节而非 checkbox，进度没被计入；
- R1–R6（IME、浮层、testkit、CI、结构、native composition）**根本没有对应步骤**，
  七十余个提交无处可勾。现已补进附录。

读法提示：这个数字表示**哪些 phase 已经开工**，不表示完成度——没有任何一步会在
后续审计发现缺口时被取消勾选。

### 路线图（R1–R6）

| 项 | 内容 | 状态 |
| --- | --- | --- |
| R1 | `floem-ime` → `TextField` | **DONE** `f6d5819` |
| R2 | 浮层（提升、轻关闭、进入动画） | **DONE** `61894b2` + `3cba342` |
| R3 | `tooling/floem-testkit` | **DONE** `5187233` |
| R5 | 2.0 专属 CI + 格式化 | **DONE** `44cff56` + `879401b` |
| R6 | 结构与约定（目录、层级表） | **DONE** `36c8f3c` + `3d7da27` + `a5c8a71` |
| R4 | **Phase 16 native composition** | **进行中**，见下 |

### Phase 16 分解

| 任务 | 状态 |
| --- | --- |
| 16.1 ABI + fake + controller + pool | **DONE** `d25bf07` |
| 16.2 无头半（注册表、放置/裁剪分离、命中减法、焦点让位） | **DONE** `dab2547` `6773950` `6d0b4a6` |
| 16.2 step 7 桌面桥——翻译层 | **DONE** `6829445` |
| 16.2 step 7 —— AppKit host + scale 修复 | **DONE** `7276d27` `7b6e4c6` |
| 16.2 step 7 —— **Win32 / X11** | **未做**（本机 `cfg` 掉，编译不到 → 四判据全废） |
| 16.3 step 1 能力协商矩阵 | **DONE** `4203ef9` |
| 16.3 step 2 外部纹理 | **DONE** `c33f758` |
| 16.3 step 3 surface compositor | 未做（设备门控：KMS / Android / iOS） |
| 16.3 step 4 帧时基准 | 未做 |
| 16.4 step 1 冻结/解冻 | **DONE** `12757bf` |
| 16.4 step 2 冻结纹理动画 | **DONE** `4d0ab28` |
| 16.4 step 3 —— **D11 设备用例** | **未跑**（解锁了，但探针还没真的发起过渡） |

### 设备用例：3 / 29 已跑

| 组 | 数量 | 状态 |
| --- | --- | --- |
| R1 IME | 4 | 全 PENDING |
| R2 浮层 | 8 | 全 PENDING |
| D1–D12 native composition | 12 | **D1 PASS**，D11 已解锁未跑，其余 PENDING |
| G1 真窗 | 5 | **G1-1 / G1-5 PASS**，G1-2…4 PENDING |

已 PASS 的三条：

- **G1-1**（2026-09-15、09-16 两次）M4 Pro Metal，1200+ 帧，`fragments reused 3 encoded 1`，无 panic。
- **G1-5**（2026-09-16）探针装好 provider，`native views installed, NATIVE_VIEW true`，四个 surface 全部挂载。
- **D1**（2026-09-16）真实 `NSView` 落点与框架期望完全一致：
  `native view 0: frame (0.0, 278.0, 200.0, 80.0) points` ==
  `framework expects: (0.0, 278.0, 200.0, 80.0) points = (0, 556, 400, 160) device pixels`。
  这是 16.2 step 7 唯一的设备证据，也顺带确认了 origin 是 `TopLeft`。

---

## 四、下一步

按「headless 可验证 → 解锁设备用例」这个排序规则：

1. **16.4 step 3 = D11（推荐）** —— 唯一还能在本机往前推的一项。
   要让 `examples/floem2-probe` 真的调一次 `begin_native_transition`，然后肉眼判断
   "是一张在动的图，接缝处不闪"。headless 半已就绪，缺的只是探针的发起 + 一双眼。
2. **16.2 Win32 / X11** —— 只剩两个 trait impl（翻译层与桥都已就位），但本机
   `cfg` 掉，编译不到，四判据一条都过不了。需要 Windows / Linux 构建目标。
3. **16.3 step 3 / 4** —— 设备门控，同上。
4. **跑设备用例 R1-1…4、R2-1…8、G1-2…4** —— 探针已具备全部前置控件，缺的是一次
   完整的肉眼巡检（约 30 分钟，一次性）。
