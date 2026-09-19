# 绘制与动画能力缺口（Canvas & Motion Gap）

> 对标对象：**Flutter** 与 **Compose Multiplatform（KMP 路线的 UI 层）**。
> 本文只回答一件事：把这两家"开箱即用"的能力当标尺，floem 2.0 现在缺什么、缺到什么程度、补上算达标。
> 每一条都带 `文件:行` 证据，且证据都在**当前工作区**里可复现（不是记忆、不是计划文本）。

---

## 0. 摘要（TL;DR）

- **底座已经在**：确定性动画调度器 + 可注入时钟、合成通道（compositor）、宿主冻结时钟（paused）、`settle()` 测试收敛、快照/像素黄金、分配计数与性能预算门。动画**不是从零开始**。
- **动画侧的原语层已补齐（S1，见 §2b）**：缓动、中断重定向、循环/往返、给任意节点起动画的公开入口都已落地并有会红的负例。**动作**：overlay 入场在 S1 前是全仓仅有的两处动画之一，Tween 也是纯线性——那两条描述已随 S1 过期，保留在此仅作对照。
- **自绘侧一行未动**（S2–S4 的范围）：`PaintContext` 仍是 5 个方法，场景 IR 仍是 4 种节点。画不了路径、描边、圆弧、渐变、裁剪、模糊、着色器。**这一半的缺口清单 A 完全未动。**
- **最贵的一条**：`SceneNode::Group { transform }` **有类型、无实现** —— 三个后端里没有一个应用它，宿主也从不产生它，而 `Transform` 只有 `translate` 构造器。这意味着**子树级变换（统一缩放/旋转）在架构上是空的**，它不是"加个字段"能补上的。
- **最便宜的一刀**：Material 设计系统里 `MotionDuration` token **零消费者**（`design-systems/floem-material/src/scale.rs:134`）—— 设计语言已经写好，引擎没有能消费它的 API。

---

## 1. 判据与优先级定义

**"补上算达标"必须可执行**，所以每条的判据都写成：*新增什么类型/API* + *哪条测试会因此变红* + *哪个门能守住它不腐烂*。

| 级别 | 含义 |
|---|---|
| **P0** | 当前**无法表达**该效果，不是画得难看，是画不出来 |
| **P1** | 对标必备、日常需求，缺了会被评价为"没有动画能力" |
| **P2** | 进阶 / 按需，可在 P0/P1 之后排 |

规模用 **S**（≤3 天）/ **M**（≤2 周）/ **L**（>2 周，跨 crate + 三后端）表示，均为**单工程师**口径。

---

## 2. 现状：已经有什么（不要把底座算成缺口）

| 已有 | 证据 |
|---|---|
| 帧驱动动画调度器 `AnimationScheduler`（`start`/`cancel`/`tick`） | `framework/floem-animation/src/lib.rs:26-85` |
| 可注入时钟（测试用 `FakeClock`）→ 动画可确定性断言 | `framework/floem-animation/src/clock.rs`、`framework/floem-animation/tests/scheduler.rs` |
| 四种动画种类 `Tween` / `Spring` / `Keyframe` / `Repeat` | `framework/floem-animation/src/kinds.rs`（S1 增 `Repeat`） |
| 合成通道 `CompositorProp`（不经布局的透明/位移/缩放） | `framework/floem-animation/src/kinds.rs:8-13` |
| 宿主逐帧推进 + 宿主时钟 + 暂停冻结 | `app/floem-app/src/lib.rs:1325`（`tick_animations`）、`anim_clock` |
| 原生参与者冻结/解冻过渡复用同一调度器 | `app/floem-app/src/lib.rs:2190`、`:2199`、`:2207` |
| overlay 入场动画（BottomSheet 位移 / Drawer 位移 / 其余淡入） | `framework/floem-overlay/src/lib.rs:85-107` |
| 测试收敛门（64 轮，明确覆盖 9 帧入场动画） | `tooling/floem-testkit/src/lib.rs:69` |
| 像素/场景/语义三层黄金 + 分配计数 + 性能预算门 | `tooling/floem-snapshot/`、`scripts/check-perf-budgets.sh` |
| 外部纹理通路（自带渲染器 → 作为场景 Image 画进来） | `render/floem-renderer/src/external.rs:1-52` |

**一句话**（S1 之后）：通道、时钟、可测性、**缓动/中断/循环/面向作者的入口**都齐了；剩下的是**绘制原语**（A 清单整片）与**隐式/过渡层**（B4–B6）。

---

## 2b. S1 已交付（2026-09-19）

> 本节引用**新代码一律用 `文件::符号` 而不写行号**：这一片代码刚落地，行号在下一个改动里立刻腐烂，符号名与测试名不会。旧条目的 `文件:行` 是 S1 之前的快照，保留未改。

| 缺口 | 落点 | 钉住它的测试 |
|---|---|---|
| **B1 缓动** | `foundation/floem-types/src/motion.rs::Easing`（Linear / CubicBezier / Spring 三种阻尼区）＋ `AnimKind::Tween.easing`（`framework/floem-animation/src/kinds.rs`，默认 `Linear` = 改造前的行为，曲线是 opt-in） | `floem-types` 的 `motion` 单测（逐点数值）、`framework/floem-animation/tests/motion.rs` |
| **B2 中断/重定向** | `floem-animation::AnimationScheduler::retarget`（tween → 带初速 spring）、`floem_app::WidgetHost::retarget_animation` | `floem-animation/tests/motion.rs`（不跳变）、`app/floem-app/tests/motion_host.rs::retargeting_carries_the_momentum_it_interrupted`（**真的保速度**：改目标后先沿原方向再走一帧，零初速的 spring 做不到） |
| **B3 循环/无限/往返** | `AnimKind::Repeat { inner, iterations: Option<u32>, mode }`、`RepeatMode::{Restart, Reverse}`；spring 无 span → 文档化的"退化为播一次" | `floem-animation/tests/motion.rs`（环绕/有界结束/往返/退化）、`floem-widgets` 的 `INDETERMINATE_SWEEP` |
| **B8 公开入口** | 命令式 `WidgetHost::{animate, retarget_animation, cancel_animation, animation_value, animations_running}`；声明式 `floem_core::Widget::animation()`；控件自绘侧 `PaintContext::animation_value()`；**facade 词汇**：`floem2::prelude` 导出 `AnimKind/Tween/Easing/CompositorProp/RepeatMode/AnimId`，`floem2::advanced` 导出 `AnimationSpec/RepeatSpec` | `app/floem-app/tests/motion_host.rs`（16 条）、`app/floem2/tests/prelude_surface.rs::the_prelude_can_animate_a_node_from_a_single_glob_import` |
| **B9 自检门** | 稳定帧预算（金帧留待 S2 与像素黄金一批补） | `app/floem-app/tests/allocation.rs::a_running_animation_does_not_allocate_per_node`（11k 节点、动画在跑 → **0 次分配/帧**）与 `::a_running_animation_does_not_allocate_per_animation`（10 → 400 个动画，分配数不得增长） |

**四判据的实测结果（2026-09-20）**：

- `cargo test --workspace --no-fail-fast` → **2304 passed / 0 failed / 74 ignored / EXIT=0**（基线 2254，**delta = +50**，逐项对账：`floem-types/src/motion.rs` +11、`floem-animation/src/kinds.rs` 内联 +4、`floem-animation/tests/motion.rs` +15、`floem-overlay/tests/overlay.rs` +1、`floem-app/tests/motion_host.rs` +16、`floem-app/tests/allocation.rs` +2、`floem2/tests/prelude_surface.rs` +1）。
- `bash scripts/check-architecture-deps.sh` → PASS。
- **真窗口**：`examples/floem2-window` 加了一条一次性入场淡入（0.4s / ease-out / opacity），`FLOEM2_WINDOW_IDLE=1 FLOEM2_WINDOW_IDLE_SECS=8` 实测 `frames` 从 2 涨到 **26**（含淡入的 24 帧）后**冻住**、`about_to_wait=50`（未空转）、CPU 2.9%（Apple M4 Pro / Metal）。**即：动画真实地在 winit+Metal 帧循环里跑过并释放了循环。** 淡放刻意做成一次性——无限动画会让 G1-4 这条用例失去可证伪性。EXAMPLES.md 里原本写死的 `frames=2` 已同步更新（这正是"点名的证据会腐烂"的又一例）。
- 未做：逐帧**像素金帧**（B9 的另一半）与 `Rotate`；`Scale` 仍受 A6 阻塞。

**落地时被测试逼出来的两条语义**（都不是实现细节，是契约，写在这里免得下次重新踩）：

1. **"跑完"与"停掉"是两件事。** 动画**跑完**要停在终点（终点是调用者要的状态；末帧清掉采样会让节点在接缝最显眼的一帧弹回原样），**停掉**（`cancel_animation`）则要放掉合成通道。否则节点会永远裹着一层没有动画在喂的合成属性，而除了再起一个动画没人能清掉它。两条都有对照测试（`a_finished_animation_parks_its_endpoint` / `cancelling_an_animation_releases_the_channel`）。
2. **"还在跑"必须自己把帧循环唤醒。** 一条时长足够长的动画（采到的 8 位 alpha 还没跨过一格、什么都没伤到）是 `needs_frame` 里动画子句**唯一可达**的场景；只要有像素在动，损伤子句就会把断言带过——`a_running_animation_keeps_the_loop_awake_when_no_pixel_changes` 专门隔离这一条。
3. **仍然是"每元素一个动画"。** 两条入口共用同一个合成属性槽：同时驱动淡入 + 位移需要 A6 的子树变换，`WidgetHost::animate` 的文档里写明了这一点。

---

## 3. 缺口清单 A：自绘 / Canvas

### A1. 绘制原语只有矩形与圆角矩形 — **P0 / M**

- **现状**：widget 面向的绘制接口一共 5 个方法：`bounds` / `fill_rect` / `fill_rounded_rect`（默认降级成 `fill_rect`）/ `fill_text`（默认 **no-op**）/ `draw_image`（默认 no-op）—— `foundation/floem-core/src/context.rs:166-194`。场景 IR 只有四种节点：`Group` / `Rect{radius}` / `Image` / `Text` —— `scene/floem-scene/src/node.rs:7-38`。**没有** `Path`、`Stroke`、`Circle`、`Oval`、`Arc`、`Line`、`Points`。
- **对标**：Compose `DrawScope` 有 `drawPath` / `drawLine` / `drawArc` / `drawOval` / `drawPoints` / `drawRoundRect` 且 `DrawStyle` 可选 `Stroke`；Flutter `Canvas` 同名一族 + `Paint..style = PaintingStyle.stroke`。
- **实际代价**：折线图、饼图/扇区、进度环、波浪、任意自定义形状、路径动画——全部画不出来。圆只能用 `Rect{radius}` 近似，非方形框里直接是错的。
- **补上判据**：`SceneNode::Path { path, fill: Option<Color>, stroke: Option<Stroke{width, cap, join, color}> }`；软件渲染器能栅格化二次/三次贝塞尔（抗锯齿水准与现有 rounded-rect 一致）；加一条**会红的负例**（改一个控制点 → 像素黄金变红）。

### A2. 没有裁剪能力（Clip） — **P0 / M**

- **现状**：场景 IR 无 clip 节点。裁剪只存在于渲染器内部，且是**宿主算好传进来的轴对齐矩形**：`app/floem-app/src/lib.rs:2557`（`fn clip(&self, index, rect, viewport) -> Option<Rect>`）、软件渲染器 `render/floem-renderer-software/src/lib.rs:531` 附近的 `clip_region`。
- **对标**：Compose 有 `Modifier.clip(shape)`，`DrawScope` 本身还带"内容不裁剪、可溢出"的明确语义；Flutter 有 `ClipRRect` / `ClipPath` / `canvas.clipPath`。v1 floem 也有：`Renderer::clip(&impl Shape)` + `clear_clip`（`legacy/floem-renderer/src/lib.rs:137,140`）与 `src/views/clip.rs`。
- **补上判据**：场景级 `Clip { shape, children }`（或等效的 push/pop clip 栈），三后端像素一致；圆角裁剪与路径裁剪各一条黄金。

### A3. 没有画刷（渐变 / 图像笔刷） — **P1 / M**

- **现状**：`SceneNode::Rect.fill` 是**单色 `Color`**（`scene/floem-scene/src/node.rs:12-20`），文字同理。
- **对标**：Compose 所有绘制都有 `color:` 与 `brush:` 两个重载（`Brush.linearGradient` / `radialGradient` / `sweepGradient`）；Flutter `Paint..shader = LinearGradient(...).createShader(rect)`。v1 有 `fill(path, brush: impl Into<BrushRef>, blur_radius)`（`legacy/floem-renderer/src/lib.rs:159`）。
- **补上判据**：`Brush { Solid(Color), Linear{stops, from, to}, Radial{..} }`，vello 映射 `peniko::Brush`、skia 映射 shader；一个渐变按钮/进度条黄金。

### A4. 没有模糊 / 阴影 / 滤镜 — **P1 / M**

- **现状**：场景 IR 与 `PaintContext` 都没有 blur / shadow / renderEffect 字段。
- **对标**：Compose `Modifier.blur`、`graphicsLayer { renderEffect = ... }`、`Modifier.shadow`；Flutter `ImageFiltered` / `BackdropFilter` / `BoxShadow` / `PhysicalModel`。v1 的 `fill(..., blur_radius)` 支持模糊（`legacy/floem-renderer/src/lib.rs:159`）。
- **补上判据**：至少一条 blur 通路（软件后端可显式跳过并记录降级），`shadow` 可由 blur + 偏移合成；黄金 + 一条"降级不静默"的断言。

### A5. 没有自定义着色器通路 — **P2 / L**

- **现状**：2.0 目录内**零** RuntimeEffect / AGSL / SkSL 相关代码。现阶段的自定义 GPU 内容只能走外部纹理通路（`render/floem-renderer/src/external.rs:1-27`：自带渲染器产出帧 → 作为普通 Image 节点参与遮挡/裁剪/变换）。
- **对标**：Compose 走 AGSL（Android，API 33+）/ SkSL（其余平台）**两套实现 + expect/actual**，且 composite 效果链只在 Android 完整——这是反面教材；Flutter 是单一 `FragmentProgram(.frag GLSL)` 全平台一致。
- **判据（含反面约束）**：见第 7 节——**不要**引入两套着色语言。

### A6. `Group` 有类型、无实现：子树变换是空的 — **P0 / L（最贵）**

- **现状**（三重证据）：
  1. IR 里存在 `SceneNode::Group { children, transform }`（`scene/floem-scene/src/node.rs:8-11`），且 `Group` 是唯一没有 bounds hint 的节点（`:55`）。
  2. **没有任何后端应用它**：软件后端 `render/floem-renderer-software/src/lib.rs:531` 是 `Group { .. } => {}`；skia 后端 `render/floem-renderer-skia/src/lib.rs:43` 同样是空分支；vello 只在 `node_fingerprint` 里读它做缓存键（`render/floem-renderer-vello/src/gpu.rs:210-228`）。
  3. **宿主从不产生它**：`app/floem-app/src/lib.rs:3183` 空分支，且注释明说"宿主构建的场景是平的——widget 画矩形/文字/图片，上面没有 group 节点"（`:3157-3158`）。
- **附带**：`Transform` 是完整六参数仿射（`foundation/floem-types/src/geometry.rs:208-217`），但**只有 `translate` 一个构造器 + `map_point`**（`:236`、`:248`）——没有 `rotate`、没有 `scale`、没有矩阵乘法、没有求逆。即使后端实现了 Group，**也无从构造嵌套变换**。
- **后果**：统一缩放（绕共同原点）、旋转、以及所有"变换类"动画在架构上无法落地。宿主对 `CompositorProp::Scale` 的处理就是这条缺口的直接证据——注释写着"Scale 故意不动：逐节点绕自身中心缩放会把子树扯散，绕共同原点需要 group，而宿主不建"（`app/floem-app/src/lib.rs:3164`）。
- **补上判据**：① `Transform::{scale, rotate, multiply, inverse}`；② 三后端在遍历时应用父变换（子节点坐标经变换、damage 区域按变换后包围盒计算）；③ 宿主在需要时真正建 group；④ 一个旋转/缩放动画示例 + 像素黄金；⑤ 架构门继续通过（Group 不得让 L3 依赖 L4）。
- **注意**：这条**先在需求上确认**再动手——如果不打算支持子树变换动画，就应该把 `Group` 从 IR 里删掉（见第 7 节"不要留无法失败的检查"）。**留着一个三个后端都不实现的节点类型，是纯粹的负债。**

### A7. 没有 Canvas / Clip / Transform 容器控件，也没有 Spinner — **P1 / S–M**

- **现状**：2.0 的控件清单（`widgets/floem-widgets/src/compose.rs`、`primitives.rs`、`controls.rs` 的 `pub struct`）里**没有** Canvas、Clip、Transform/Rotate、Spinner、Svg、RichText。v1 有 `src/views/canvas.rs`（`canvas(paint_fn)`，闭包式自绘 + 依赖信号自动重绘，`:42`）、`clip.rs`、`svg.rs`、`rich_text.rs`。
- **补上判据**：`Canvas` 控件（闭包式，尺寸来自布局，绘制走 `PaintContext`）＋ 依赖追踪触发重绘；Spinner 用 B3 的循环动画实现。

### A8. 自绘内容无法声明语义角色 — **P1 / S**

- **现状**：`SemanticsContext` 只有 `bounds` / `set_label` / `set_bounds` / `size_hint`（`foundation/floem-core/src/context.rs:196-203`）——**不能声明 role**。而 `SemanticsRole::Image` 是存在的（`framework/floem-accessibility/src/semantics.rs:23`），widget 侧的 `SemanticsHint` 也能带角色（`widgets/floem-widgets/src/primitives.rs:47`）。
- **对标**：Compose 的 canvas **默认不可访问**，必须在外面包 `Modifier.semantics { contentDescription = ... }`；Flutter 的 `CustomPaint` 同样需要自己加 `Semantics`。两边都栽过，所以这是"照抄教训"而不是"抄功能"。
- **补上判据**：自绘控件可声明 role（挂到 `SemanticsHint` 或 `PaintContext` 上），且 `Canvas` 控件默认产出 `Role::Image` + label；testkit 断言。

### A9. 没有自绘规模的性能门 — **P2 / S**

- **现状**：已有分配计数与 best-of-N 性能预算门（`scripts/check-perf-budgets.sh`），但**没有任何路径/图元规模的基准**。
- **补上判据**：`benches/` 增加"10k 路径编码 + 栅格化"与"1k 圆角矩形"两个基准，并入预算门。**没有这条，A1/A3 一落地就会悄悄吃掉稳定帧预算。**

---

## 4. 缺口清单 B：动画

### B1. ~~没有缓动函数（easing）~~ — **P0 / S** — ✅ 已交付（S1）

- **现状**：`Tween` 是纯线性插值 `from + (to - from) * u`（`framework/floem-animation/src/kinds.rs:43-49`）。2.0 全目录里 `easing` 这个词**只出现过一次**，还是文档注释里的 M3 规范链接（`design-systems/floem-material/src/scale.rs:133`）——**没有任何缓动实现**。v1 有完整的 `Easing` trait 与实现：`Linear` / `Step`(+`StepPosition`) / `Bezier`(三次贝塞尔参数) / `Spring`（`src/animate/easing.rs:53,62,78,159,208`）。
- **对标**：Compose `tween(easing = FastOutSlowInEasing)` 等；Flutter `Curves.easeInOut / bounceOut / elasticIn`。
- **代价**：所有动画线性 → 观感机械，这是"开箱即用动画"的第一道门槛，也直接卡住 B6。
- **补上判据**：`Easing`（至少 linear / cubic-bezier / spring 参数化）接入 `AnimKind::Tween`；**按 t=0.25/0.5/0.75 逐点断言数值**（不是只测端点）。

### B2. ~~没有中断 / 重定向（interruption & retarget）~~ — **P1 / M** — ✅ 已交付（S1）

- **现状**：`AnimationScheduler` 只有 `start` / `cancel` / `tick`（`framework/floem-animation/src/lib.rs:38-84`）——取消即丢失进度，无法"从当前值继续飞"。
- **对标**：Compose `Animatable.animateTo` 可从当前值**和当前速度**续飞；Flutter `AnimationController.animateTo` 同理。手势跟随（拖拽回弹）依赖这条。
- **补上判据**：`retarget(id, new_target)` 保留当前值与速度；测试断言"中途改目标不跳变"（相邻两帧的值差有界）。

### B3. ~~没有循环 / 无限 / 往返~~ — **P1 / S** — ✅ 已交付（S1）

- **现状**：`AnimKind` 只有 `Tween` / `Spring` / `Keyframe`（`kinds.rs:23-37`），`tick` 到 `finished` 即从表里移除（`lib.rs:76-83`）。
- **对标**：Compose `repeatable(iterations, repeatMode = RepeatMode.Reverse)` / `infiniteRepeatable`；Flutter `controller.repeat(reverse: true)`。
- **代价（已可见）**：spinner、骨架屏（shimmer）、marquee、脉冲全都做不了——2.0 自己的不确定态进度条就卡在这，注释写着"animation comes later with `floem-animation`"（`widgets/floem-widgets/src/controls.rs:1725`，控件本体见 `:1627`）。
- **补上判据**：`AnimKind::Repeat { spec, iterations: Option<u32>, mode: Reverse|Restart }`；不确定态 Progress 用它实现；**并加一条"无限动画不等于每帧分配"的预算断言**（否则稳定帧预算会被它吃掉）。

### B4. 没有"改值即动"的隐式层 — **P1 / M**

- **现状**：2.0 **没有**任何样式属性过渡机制（`Transition` / `easing` 在 2.0 目录零命中）。v1 有 `.transition(Background, Transition::ease_in_out(500.millis()))`（`examples/themes/src/main.rs:25-93`、`src/style/transition.rs`）。
- **对标**：这正是 Compose 的 `animate*AsState` 家族（Float/Dp/Color/Int + 自定义类型的 `TwoWayConverter`）与 Flutter 的 `AnimatedXxx` 组件（`AnimatedContainer`/`AnimatedOpacity`/…）。**它覆盖两家约 70–80% 的实际用例**。
- **代价**：hover / pressed 变色、展开收起、透明度变化——最常见的三个需求在 2.0 全部无 API。
- **补上判据**：样式属性可声明过渡（属性 + 时长 + 缓动），状态变化时自动启动；hover 变色与展开收起各一条像素黄金。**注意：这条依赖 B1。**

### B5. 没有过渡层（挂载 / 内容切换 / 共享元素） — **P1 / L**

- **现状**：只有 overlay 入场（`framework/floem-overlay/src/lib.rs:85-107`，时长硬编码为常量 `ENTRY_SECS = 0.15`，见 `:16`）与原生参与者冻结/解冻（`app/floem-app/src/lib.rs:2190`）。**没有**组件挂载/卸载过渡、内容切换、共享元素。
- **对标**：Compose `AnimatedVisibility(enter, exit)` / `AnimatedContent` / `Crossfade` / `SharedTransitionLayout`（1.7+）；Flutter `AnimatedSwitcher` / `AnimatedCrossFade` / `Hero`。
- **补上判据**：`Conditional`（`widgets/floem-widgets/src/compose.rs:161`）支持 enter/exit 过渡；`AnimatedContent` 或 Crossfade 之一；共享元素给一个示例 + 快照（不做完整 SharedTransitionLayout 也算达标，但必须**在文档里写明只做了哪个子集**）。

### B6. 组合层缺失 + 设计系统 motion token 零消费者 — **P1 / S（最便宜的一刀）**

- **现状**：`MotionDuration`（Material 3 时长 token）已定义并 re-export（`design-systems/floem-material/src/scale.rs:134`、`design-systems/floem-material/src/lib.rs:25`），但**全仓没有任何消费者**；也**没有** `MotionEasing`（缓动 token）。讽刺的是，定义它的上一行正是 M3 的 easing-and-duration 规范链接（`scale.rs:133`）——**duration 实现了，easing 没有**。
- **对标**：Compose 侧对应 `updateTransition`（多值共用同一时钟）与 Material 的 motion token；Flutter 侧对应同一份 M3 时长/曲线表。
- **意味**：设计语言已经写好了，引擎没有能消费它的 API。**先补 B1 + B4，然后把 token 接上**，是投入产出比最高的一步。
- **补上判据**：至少一个真实消费点（overlay 入场或隐式过渡按 token 取值），并且**删掉"零消费者 token"**这个状态本身。

### B7. 合成属性不全 + `Scale` 不可用 — **P1 / S（但依赖 A6）**

- **现状**：`CompositorProp` 只有 `Opacity` / `TranslateX` / `TranslateY` / `Scale`（`kinds.rs:8-13`）——**没有 Rotate**；而 `Scale` 被宿主明确跳过（`app/floem-app/src/lib.rs:3164`），原因就是 A6。
- **补上判据**：增加 `Rotate`；`Scale` 在 group 上真正生效；一个缩放/旋转动画的像素黄金。**这条是 A6 的下游。**

### B8. ~~没有"给任意节点起动画"的入口~~ — **P0 / S** — ✅ 已交付（S1）

- **现状**：全仓只有 **2 处** `AnimationScheduler::start`：宿主的原生过渡（`app/floem-app/src/lib.rs:2199`）与 overlay 入场（`framework/floem-overlay/src/lib.rs:99`）。**app 与 widget 作者无法让任意节点动起来。** 同时 2.0 **没有** per-widget 重绘请求 API（`request_paint` / `invalidate_paint` 零命中；v1 有 `id.request_paint()`，见 `src/views/canvas.rs:82-84`）。
- **补上判据**：`host.animate(node, AnimKind) -> AnimHandle` 之类的公开入口（或控件级 API）；并**用帧统计断言它只驱动重绘、不触发重排**——否则每个动画都会退化成全树布局。

### B9. 动画缺少自检门（金帧 + 稳定帧预算） — **P2 / S** — ◐ 半交付（S1 只补了稳定帧预算，金帧并入 S2）

- **现状**：已有 `framework/floem-animation/tests/scheduler.rs`（FakeClock 确定性）与 `benches/frame_rates.rs`；testkit 的 `settle()` 能跑完动画（`tooling/floem-testkit/src/lib.rs:69`）。**缺**：动画中途的像素金帧、以及"动画帧不新增分配"的门。
- **补上判据**：任选一个动画，在 t=0 / 50% / 100% 三个时刻断言场景（或像素）；并加一条稳定帧分配断言。

---

## 5. 依赖顺序（不要打乱）

```
B1 (easing) ──► B4 (隐式层) ──► B6 (接 design-system token)
    │
    └──► B5 (过渡层：AnimatedVisibility/AnimatedContent)

B3 (循环) ──► indeterminate Progress / Spinner

A6 (Group/Transform 真正落地) ──► B7 (Scale/Rotate 合成属性)

B8 (公开动画入口) ──► 任何"用户能看见"的动画示例
    └── 也是 A7(Canvas 控件) 与 B4/B5 的前置

A1 (路径/描边) ──► A2 (裁剪) ──► A3/A4 (画刷/模糊) ──► A5 (着色器)
                        └── A9 (性能门) 应在 A1/A3 落地同一批补上
A8 独立，随时可做（但越早越好——它是"自绘控件在无障碍树里是否可见"的分水岭）
```

**必须配套的两条**：每落地一批原语/动画，**同一批**补上 A9 的性能门与 B9 的金帧——本项目已有"文档里点名的证据会腐烂"的前车之鉴，动画这类"看起来对就行"的能力最容易悄悄退化。

---

## 6. 建议切片（每片可独立交付、独立验证）

| 切片 | 内容 | 规模 | 交付物 |
|---|---|---|---|
| **S1 让动画"像样"** ✅ 已交付 | B1 + B2 + B3 + B8 + B9 | M | 缓动/中断/循环 + 公开入口；**示例未做**（`examples/` 零改动）——不确定态 `Progress` 已作为真实消费点接上，hover 渐隐与拖拽回弹随 B4 一起做（它们本质是隐式层） |
| **S2 让自绘"能用"** | A1 + A2 + A7 + A8 + A9 | M–L | 路径/描边/裁剪 + Canvas 控件 + 语义 role + 规模基准；示例：折线图 + 进度环 |
| **S3 让变换"成立"** | A6 + B7（**需先确认需求**） | L | 子树变换在三后端落地 + 缩放/旋转动画黄金；若需求不成立则改为**删除 Group** |
| **S4 让风格"成套"** | A3 + A4 + B4 + B5 + B6 | L | 画刷/模糊 + 隐式过渡 + 挂载过渡 + motion token 接通 |

四个切片都完成后，与 Flutter / Compose Multiplatform 在"canvas + 动画"这两项上的差距只余 **A5（自定义着色器）**，而 A5 恰好是**唯一一个"两边都需要平台分支"的能力**（见下）。

---

## 7. 不建议做的事（反面清单）

1. **不要引入两套着色语言。** Compose 的 AGSL(Android) + SkSL(其余) 路线意味着每个自定义效果都要写两遍，且效果链在 Android 才完整。若做 A5，只选**一条**语言 + 明确的降级行为（软件后端跳过并记录），而不是复制 expect/actual。
2. **不要做脱离场景 IR 的即时模式 canvas。** 那样会绕过 damage、缓存与快照黄金 —— 本项目"审计每帧分配"的整套门都会失效。自绘必须**翻译成场景节点**。
3. **不要给每个样式属性都加过渡。** 先只做 B1+B4 的最小集合（颜色、尺寸、透明度、位移），否则会造出一堆"有字段没实现"的假能力。
4. **不要留着三后端都不实现的节点类型。** A6 的 `Group` 现在就是这种负债：要么三后端都实现（S3），要么从 IR 删除。二者必须选一个。
5. **不要用"再加一个 token"代替接通。** `MotionDuration` 零消费者的状态说明：设计系统侧的加法已经很便宜了，缺的是消费端。

---

## 8. 验证方式（每条判据怎么跑）

```bash
export PATH="/opt/homebrew/bin:$PATH"
cd /Users/humphrey/Project/Rust/floom/floem

# 1) 会红的失败测试：每个缺口补上时，负例必须变红（改一个控制点/时长/端点）
cargo test --workspace --no-fail-fast          # 全绿 + delta 逐项对账
bash scripts/check-architecture-deps.sh        # 层级不得新增倒挂（A6 尤其要盯）
cargo test -p floem-animation                  # FakeClock 确定性断言
cargo bench -p floem-animation --bench frame_rates
bash scripts/check-perf-budgets.sh             # 并入 A9 的自绘基准后仍须 PASS

# 2) 像素/场景黄金（新增动画金帧后）
UPDATE_SNAPSHOTS=1 cargo test -p floem-snapshot
# 3) 真窗口冒烟（动画必须在真实帧循环里跑过一次）
FLOEM2_WINDOW_IDLE=1 FLOEM2_WINDOW_IDLE_SECS=30 \
  cargo run -p floem2-window --features real
```

**注意**：动画的"看起来对了"不能作为证据。本项目的既有纪律是 **① 会红的负例 ② 对照组 ③ 明确退出码**——动画这类主观能力，这三条一个都不能省。

---

## 附：能力对照速查

| 能力 | floem 2.0 现状 | Compose Multiplatform | Flutter |
|---|---|---|---|
| 画矩形 / 圆角矩形 / 文字 / 图片 | ✅ | ✅ | ✅ |
| 路径 / 描边 / 圆弧 / 椭圆 / 线 | ❌ (A1) | ✅ `drawPath`/`drawArc`… | ✅ `Canvas.drawPath`… |
| 裁剪 | ❌ 仅宿主内部轴对齐矩形 (A2) | ✅ `Modifier.clip` | ✅ `ClipRRect`/`ClipPath` |
| 渐变 / 画刷 | ❌ (A3) | ✅ `Brush.*` | ✅ `Paint.shader` |
| 模糊 / 阴影 / 滤镜 | ❌ (A4) | ✅ `blur`/`renderEffect` | ✅ `ImageFilter`/`BoxShadow` |
| 自定义着色器 | ❌ 仅外部纹理绕行 (A5) | ✅ 需 AGSL+SkSL 两套 | ✅ `.frag` 一套 |
| 子树变换（缩放/旋转） | ❌ 类型存在、无人实现 (A6) | ✅ `graphicsLayer` | ✅ `Transform` |
| Canvas 控件 | ❌ (A7，v1 曾有) | ✅ `Canvas` | ✅ `CustomPaint` |
| 自绘内容语义 | ❌ 不能声明 role (A8) | ⚠️ 需手动 `semantics` | ⚠️ 需手动 `Semantics` |
| 缓动函数 | ❌ 纯线性 (B1，v1 曾有) | ✅ 多种 | ✅ `Curves` |
| 中断 / 重定向 | ❌ (B2) | ✅ `Animatable` | ✅ `AnimationController` |
| 循环 / 无限 / 往返 | ❌ (B3) | ✅ `infiniteRepeatable` | ✅ `repeat(reverse:)` |
| 隐式动画（改值即动） | ❌ (B4，v1 曾有) | ✅ `animate*AsState` | ✅ `AnimatedXxx` |
| 挂载/内容过渡/共享元素 | ⚠️ 仅 overlay 入场 (B5) | ✅ `AnimatedVisibility`/共享元素 | ✅ `AnimatedSwitcher`/`Hero` |
| 多值编排 / motion token 消费 | ❌ token 零消费者 (B6) | ✅ `updateTransition` | ✅（组件层） |
| 合成属性（不重排的动画） | ⚠️ 4 个，`Scale` 不可用 (B7) | ✅ 完整 `graphicsLayer` | ✅ `Transform`/`Opacity` |
| 公开"起动画"入口 | ❌ 仅 2 处框架内部调用 (B8) | ✅ 公开 API | ✅ 公开 API |
| 动画自检门（金帧/分配） | ⚠️ 有调度器测试与 bench，缺金帧与分配门 (B9) | — | — |
