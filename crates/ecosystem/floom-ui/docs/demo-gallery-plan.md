# floom-ui · Demo Show Gallery 方案（审批稿）

> 目标：参照 Vuetify 组件文档的展示方式（实况示例 + 属性/事件/方法 API 表 + 可切换代码），
> 把当前“98 个单一静态示例”的 Demo 升级为**可上手的交互式组件展示库**——每个组件清晰呈现
> 它的**属性（Props）、方法（Methods）、实用示例**并配以必要说明，使开发者一看即会。

---

## 1. 设计原则

1. **对标 Vuetify，适配 Rust/Floom。** Vuetify 每个组件页 = 实时 Usage 示例 + 可折叠 API 区
   （Props / Events / Slots / Exposed methods）。我们沿用这套心智模型；但因 Rust 无运行时反射，
   API 表与属性默认值以**手工元数据**维护（一次性成本，分批改）。
2. **“直接上手”优先。** 每个组件页都提供可交互的属性面板（改即所见），并把组件对外暴露的
   **方法**做成可点按的演示（如 `Dialog::open()`、`Form::reset()`、聚焦、步进），并显示可观察结果。
3. **说明到位。** 对平台/DS 差异、浮动层渲染约束、易错点给出 inline 提示（notes）。
4. **复用现有架构，不引入新依赖。** 沿用 `examples/demo` 的 master/detail 双栏壳、DS 切换器、
   Dark 切换器，以及窗口根 overlay 机制；仅扩展 `showcase` 辅助模块。

---

## 2. 现状基线（已确认）

- `examples/demo/src/gallery.rs`：98 个 `pages.push(...)`，两栏 master/detail；约 90% 是单一静态示例。
- `examples/demo/src/showcase.rs`：已提供 Vant 风格能力
  - `Demo` / `demo()` / `demo_note()` / `demo_ctl()`：静态变体 + 交互变体（组件 + 控件面板）。
  - `showcase(name, intro, demos)`：把一组 demo 渲染为卡片。
  - `toggle_row()` / `segmented()`：bool 开关、枚举分段选择器（已绑定 `RwSignal`）。
- 仅 **8 个**组件真正使用交互式 `showcase`（Button / Switch / Slider / Tabs / Chip / Card + 内联几例）；
  其余 90 个只有 `page(title, 单个示例)`。
- 缺失项：无**方法演示**、无 **API 参考表**、无**源码片段**、无系统化的**属性面板**。

---

## 3. 目标页面结构（每个组件的统一模板）

每个组件页由以下分区纵向组合（在 `showcase` 模块内扩展为统一渲染器）：

```
┌──────────────────────────────────────────────────────────┐
│ 组件名（18pt）                                             │
│ 一句话简介（13pt，on_surface 弱化色）                       │
├─────────────── 属性 Props ───────────────┐
│ ▸ 交互演示卡片（demo_ctl）                 │  ← 控件面板绑定组件 props，改即所见
│   组件实例 + [Variant ▢▢▢] [Size ▢▢▢]       │
│              [Disabled ⏻] [Loading ⏻]     │
├─────────────── 方法 Methods ──────────────┤
│ ▸ 方法演示卡片（demo_method，新增）         │  ← 按钮触发暴露的方法 + 显示结果
│   [Open dialog] → 窗口根浮层打开           │
│   [Reset form]  → 表单清空，valid 信号重置 │
├─────────────── 示例 Examples ─────────────┤
│ ▸ 静态变体卡片（demo / demo_note）          │  ← 各种状态/变体一目了然
│   Primary / Secondary / Danger / Disabled … │
├─────────────── 说明 Notes ────────────────┤
│ ⓘ 浮动层需在窗口根渲染；Cupertino 不支持 X  │
├─────────────── API 参考（Phase 2 可选）────┤
│ 表：名称 | 类型 | 默认值 | 说明            │
└──────────────────────────────────────────┘
```

---

## 4. 需要新增的基础设施（不引新依赖）

在 `showcase.rs`（或拆出 `showcase_widgets.rs`）中新增：

| 新增项 | 作用 |
|---|---|
| `demo_method(label, runner, result_view)` | 方法演示卡：一个按钮调用组件方法，下方/旁边显示可观察结果（signal 值、可见性等） |
| `api_table(title, rows: Vec<ApiRow>)` | 渲染 Props/Events/Methods/Slots 表（列：名称 / 类型 / 默认值 / 说明） |
| `ApiRow` + `ComponentApi { props, events, methods, slots }` | 每组件一个 `api()` 函数返回结构化元数据；与展示解耦，分批填充 |
| `code_snippet(label, src: &str)` | 等宽字体 + 横向滚动的源码卡（可折叠），展示该 demo 的真实写法 |
| 扩充控件：`slider_int`、`select_enum`、`color_pick` | 让属性面板能覆盖数值/枚举/颜色类 props，不止 bool/枚举分段 |

> 浮动层类组件（Dialog / Menu / Snackbar / NavigationDrawer / Sheet / Overlay）的方法演示
> **复用 `gallery::build()` 既有的窗口根 overlay 信号**（如 `dialog_open`），不内联，否则 scrim 塌缩。

---

## 5. 数据 / 渲染分层

- 每个组件一个 `xxx_showcase() -> AnyView`（已存在于 `showcase.rs`，其余在 `gallery.rs` 内联）。
- `gallery.rs` 的 `pages.push(("X", Box::new(xxx_showcase)))` 统一改为调用对应 showcase 函数。
- API 元数据独立到 `showcase_api.rs`，与展示代码分离，便于分批填充、单测。

---

## 6. 分阶段实施（每阶段可独立评审 / 验证）

- **Phase A — 基础设施**：扩展 `showcase` 模块（`demo_method`、`api_table`、`code_snippet`、扩充控件），
  不改动任何现有页面渲染，保证 `cargo check -p floom-demo` 仍绿。
- **Phase B — 试点样板（8 个代表组件，覆盖各类别）**：
  - 输入类：TextField / Select / Slider
  - 反馈类：Dialog / Snackbar / Alert
  - 导航类：Tabs / NavigationDrawer
  - 数据类：DataTable / Rating
  - 动作类：Button / FloatingActionButton
  - 提供类：ThemeProvider / LocaleProvider
  做成“属性 + 方法 + 示例 + 说明 + API 表”完整模板，作为后续 **参考样板**。
- **Phase C — 批量铺开**：按类别复用样板，把其余 ~86 个组件补齐到统一格式（属性面板 + 方法演示 + 示例 + 说明）。
- **Phase D — 验证**：`cargo check -p floom-demo`（multi）+ `--no-default-features --features cupertino`
  + `--features fluent`，确保三套 DS 均 0 错误。

---

## 7. 验证方式

- 编译：`cargo check -p floom-demo` 三种 feature 配置（material 默认 / cupertino / fluent / multi）。
- 不引入新依赖；纯 `floom-elements` 既有 API。
- 交互自检：DS 切换、Dark 切换、浮动层方法演示在窗口根正常开合。

---

## 8. 风险与注意

1. **API 表手工维护**工作量大但一次性；先用 Phase B 样板固化格式，C 阶段批量填充。
2. **三 DS 形态差异**：部分 Cupertino/Fluent 可能缺某些 builder → 属性面板用通用属性，或 `#[cfg(feature="multi")]`
   分支只展示共有 props，避免编译失败。
3. **浮动层方法演示**必须复用 `build()` 的窗口根信号，严禁在页面内联（会导致 scrim 塌缩、不可点的历史 bug）。
4. 保持 `main.rs` 的 headless 冒烟测试不破坏（构建最小子树而非整树）。

---

## 9. 待确认点（请审批时一并反馈）

- 是否要包含 **API 参考表** 与 **源码片段**（Phase 2）？还是先只做 属性+方法+示例+说明（Phase A–C）？
- 方法演示的“结果展示”偏好：内联文本回显（如 `open = true`）即可，还是需要更醒目的视觉反馈？
- 试点组件清单（Phase B）是否同意上述 12 个，还是你指定优先顺序？
