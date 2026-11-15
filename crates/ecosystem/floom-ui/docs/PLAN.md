<!--
Reconciled: 2026-11-11 — supersedes the 2026-07-31 v1 architecture plan.
Status: working task list for today's `crates/ecosystem/floom-ui`.
Why replaced: the 2026-07-31 plan assumed v1 (`floom-elements` three-layer facade +
per-component crates that held the implementation + `examples/demo`). None of that
architecture exists in this tree any more. §1–§2 record the real v2 architecture and
what changed; §3 is the actual remaining work. The old plan survives in git history and,
in more detail, in `docs/superpowers/plans/2026-07-31-floom-elements.md` (itself v1-era).
Conventions (naming / token-first / a11y / testing) are still valid — see §4.
-->

# floom-ui 架构与任务清单（Reconciled 2026-11-11）

Version: 0.2.0 (workspace shows root's `0.2.0`)

## 1. 当前架构（准确）

`crates/ecosystem/floom-ui` 是一个**独立的 satellite workspace**（自有 `Cargo.toml`、`justfile`、`Cargo.lock`），
根仓库的 `crates/ecosystem` 被 root manifest exclude，所以根 `cargo test --workspace` **不会**跑到它。
它由 `just gate ecosystem` 与 CI `ecosystem-check`（`.github/workflows/ci-plugins.yml`）用
`cargo test --workspace` 覆盖（每个 member 都是 headless 测试，渲染进 buffer，无窗口/无 GPU）。

**9 个 workspace member，全部能解析**（`cargo metadata`: packages=9, workspace_members=9）：

```
应用
  │
根 engine：crates/widgets/floom-widgets（盒子/文本等原语）、floom-core、floom-scene、floom-app …
  │
crates/ecosystem/floom-ui（satellite，9 members）
  ├─ floom-widget-runtime      运行时状态：主题 / DesignSystem / dark signals、布局注册、ripple
  ├─ floom-core-theme          Theme trait + tokens + 每组件 tokens/props + 全局主题状态
  ├─ floom-theme               跨 DS token 结构（Color/Typography/Spacing/Radius/Elevation/Motion、ColorScheme、ThemeScope）
  ├─ floom-design              DS 样式原语（Variant、ControlSize、InteractionState、Props/StyleSet）+ 颜色数学
  ├─ floom-ui-composites       47 个源文件（约 40 个组件）：dialog/popup/overlay/menu/tabs/tree/pickers…
  │                            基于根 engine `floom-widgets` 组合而来，DS 无关
  ├─ floom-material            Material 3 kit：MaterialRoles/MaterialTheme、button 指标/样式、M3 scale、gallery
  ├─ floom-cupertino           Cupertino/HIG kit：CupertinoColors/CupertinoTheme、button、touch target、gallery
  └─ floom-fluent              Fluent kit：每组件一 `#[cfg(feature)]`，`full` 开全部（99 个）
  └─ floom-testing             snapshot 测试助手（Snapshot、component_snapshot）
```

**关键点（旧文档都写错了）：**

- **没有 `floom-elements` facade**。它在 "Stage D" 被移除（`Cargo.toml:361-365` 原话：
  "the `floom-elements` re-export layer went with it"）。每个 kit crate 直接依赖
  `floom-core-theme` / `floom-widget-runtime`，不再经一个统一 contract crate。
  因此旧计划"应用只依赖 `floom-elements`、靠 Cargo feature 切换 DS"的用法**当前不成立**。
- **`floom-demo` crate 现在存在了**（2026-11-12）：在**根 workspace** 的 `examples/floom-demo`，
  不是本 workspace。它把 98 个组件在 Material/Cupertino/Fluent **三列并排**渲染，
  props 表在运行期从 `floom-core-theme` 自己的 props 源解析，headless 测试覆盖
  全部 98 页与每一个 sample。`justfile` 的目标已接通（`just demo` / `just demo-headless`）。
- **`floom-harmony` 已于 2026-11-15 删除**（它只是 consolidated crate，无 per-component 子 crate，
  也不在 `floom-demo` 的三列之内）。
- 每 DS 的 per-component crate **是薄适配器**；行为共享在 `floom-ui-composites` + engine `floom-widgets`，
  不是旧计划设想的"每组件 crate 自带 spec/props/state/style 实现"。
- `Cargo.toml` 顶部那段注释（"8 有 engine widget、90 待 promote、member 列表是 32"）**整体过期**：
  现在 9 members、`exclude = []`、kit crates 已合并为 feature 门控。

## 2. 旧计划 vs 现实

| 2026-07-31 旧计划 | 今天（2026-11-11） |
|---|---|
| 三层：`floom-elements` contract / per-kit adapters / core | 已改为 engine `floom-widgets` → `floom-ui-composites` → per-DS kits；`floom-elements` 已删 |
| 每组件一 crate，内含实现（`spec/props/event/state/tests`） | 已合并进 3 个 kit crate 的 per-component feature（`floom-material`/`floom-cupertino`/`floom-fluent`），每个组件一 `#[cfg(feature)]`；共享逻辑在 composites + engine |
| Phase 1 MVP：8 个 Material 组件 | 远超：98 material + 99 cupertino + 99 fluent |
| Phase 2：Cupertino ×8、Fluent ×8 | 远超：各约 99 个组件，合并进单 crate 的 per-component feature |
| Phase 3 高级组件：Table/Tree/VirtualList/Chart/Navigation | 大部分已有：`md-table`、`md-treeview`、`md-virtual_scroller`、`md-sparkline`、`md-navigation_drawer`、`md-datatable`、`md-timeline`、`md-calendar` |
| `examples/demo` + `floom-demo` | **已建**：`examples/floom-demo`（根 workspace），98 组件 × 3 DS 并排；见 P1 |
| `floom-testing` | 已存在 |
| Phase 4：Theme Builder / Figma Import / CLI Generator / docs site | **未开工** |

## 3. 真实剩余任务（按优先级）

### P0 — 让文档与磁盘一致（否则所有任务列表都在描述不存在的 crate）

- [x] 重写本 `PLAN.md` 为 v2 现状 + 本任务清单（2026-11-11）
- [ ] `docs/COMPONENT_PARITY.md`（77KB）改为 v2：清理 `floom-elements`/`floom-demo`/`examples/demo` 的残留引用
      （约 32 行）；补 `floom-ui-composites` 的说明（`floom-harmony` 已于 2026-11-15 删除）；补 Batch-14 的 5 行
      （ButtonGroup/FAB/RangeSlider/FileInput/OTPInput）进 contract 表；修 `:969-978` 过期的 implementation-order 复选框。
  - 退出：文档不再出现已删除 crate 的可执行引用；数与 `cargo metadata`（9）一致。
- [ ] `docs/demo-gallery-plan.md`：按新架构重写（去掉 `examples/demo` 路径），或明确标为 superseded。
- [ ] `docs/AUDIT_2026-08-12_...md`：标注其前提（`floom-elements`/`floom-demo` 可编译）已不成立；
      保留其中仍有效的记录（`Transition::child` vs `Overlay::child` 的 API 不一致）。
- [ ] `docs/superpowers/plans/2026-07-31-floom-elements.md`（1586 行，全 v1、复选框未勾、路径指向别的机器）：标注 superseded。
- [x] `Cargo.toml` 顶部过期注释改写为准确描述（9 members、`exclude = []`、kit 已合并为 feature 门控，2026-11-15）。
- [x] `justfile`：接通 `floom-demo` —— 三个断目标换成 `just demo` / `just demo-headless`（2026-11-12）。

### P1 — 恢复可运行 gallery/demo（组件要可见、可测、可迭代）

- [x] 建 demo/gallery crate（2026-11-12）：`examples/floom-demo`，在**根 workspace**（本 workspace 保持 headless）。
      渲染 **3 套 DS** 的 gallery —— Material/Cupertino/Fluent **并排三列、一个二进制**；
      `justfile` 目标接通（`just demo` / `just demo-headless`）。
  - 退出条件（实测）：`cargo run -p floom-demo` 起窗；`cargo run -p floom-demo -- --headless`
    逐页打印 painted 计数并退出（98 页 / 23045 painted）；`cargo test -p floom-demo` 全绿
    —— 98 页逐页 mount+paint、每个 sample 单独 mount、props 表逐字段对齐库自己的 props struct、
    按钮/复选框的交互真的走通 handler。
  - **与计划的两处偏差**（有意为之）：
    1. **不是每 DS 一个 build**，而是一个二进制三列并排 —— 三列同屏比较才是这个 gallery 的价值；
    2. **只有 3 套 DS**（Material / Cupertino / Fluent）；`floom-harmony` 已于 2026-11-15 删除。
- [ ] `demo-gallery-plan.md` 的 A–D 阶段（Props 面板 / API 表 / method demo / 代码片段）在 demo 落地后再排。
  - 已完成的部分：Props/API 表（props 表已从库自己的 props 源解析，不手写）、每组件 ≥2 个实况示例、notes。
  - 未完成：可交互的属性面板、把组件方法做成可点按的演示、源码片段。
- [x] `justfile` 接通后，把 demo 测试纳入 gate（根 workspace 的 `cargo test -p floom-demo`）。

### P2 — 组件 / 引擎缺口（真实、已在文档中点名，非推测）

- [ ] **headless primitives**：把可复用 component 行为抽成无样式原语（`COMPONENT_PARITY.md:666`）。
- [ ] **Row/Col 一等公民**：目前只有 `Grid`，Row/Col 不是独立原语（`COMPONENT_PARITY.md:537`）。
- [ ] **Batch-14 contract 表 5 行补齐**（同上 P0 文档任务的具体子项）。
- [x] **Harmony 策略**：`floom-harmony` 已于 2026-11-15 删除（consolidated-only，无 per-component 子 crate），无需再决策。
- [ ] **警告债**：`h_stack`/`v_stack`/`text`/`label`/`empty` 弃用调用 + unused imports（`AUDIT:107-109`，
      "historical debt, not addressed"；本次未重新验证数量）。
- [ ] **9 个 consolidated crate 补 `//!` 模块文档**（当前 0 个有；`floom-harmony` 已删除）。

### P3 — 旧计划 Phase 4 生态工具（1.1+，未开工）

- [ ] Theme Builder（可视化编辑 tokens）
- [ ] Figma Token Import（从 Figma 导入设计令牌）
- [ ] `floom generate` CLI generator（自动生成组件模板）
- [ ] 文档站点

> 以上 P3 属差异化/生态，非 1.0 阻塞项；按 `docs/competitive-analysis.md` 的 P2 批次口径是 1.1+。

## 4. 仍然有效的约定（沿用 `SKILL.md` / 旧计划，与架构无关的部分）

- 组件名 `PascalCase`（`Button`、`TextField`），**API 层不暴露 `MdButton`/`CupertinoButton`** 这类 DS 名。
- Props 字段 `snake_case`（`on_click`、`max_length`）；文件名 `snake_case`。
- **token-first**：禁止硬编码 `Color::rgb(...)` / magic padding，一律走 `floom-core-theme` / `floom-theme` 的 token。
- 每个组件至少支持 `aria_label()`、键盘导航、focus 管理；Button 支持 Space/Enter/Tab。
- 测试：props/state 单测、每 variant×size 的 snapshot、每 DS 的 integration demo。
- `cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo test`（satellite 内）。

> 旧计划"每组件 crate 内含 `spec.rs/props.rs/event.rs/state.rs/tests.rs`"的文件布局是 v1 形态；
> v2 下 kit crate 是薄适配器，该布局**不再强制**。上面的跨架构约定仍适用。

## 5. 验证口径

- `cargo metadata --no-deps` → 9 packages / 9 workspace members（本文件所据，2026-11-15 合并后）。
- `just gate ecosystem`（根仓库）或 CI `ecosystem-check` → `cargo test --workspace`（satellite 全量）。
  **本次 reconcile 未重跑全部组件测试**，只做了结构与元数据核对；P0/P1 的"绿"以实际跑通为准。
- **gallery（2026-11-12）**：`cargo test -p floom-demo` 全绿 —— 98 页逐页 mount+paint、
  全部 sample 单独 mount、props 表与库自己的 props struct 逐字段对齐、按钮/复选框交互走通 handler；
  `cargo run -p floom-demo -- --headless` 逐页打印 painted 计数（98 页 / 23045 painted）。
  代价：gallery 落地时给 6 个 composites + 2 个 engine widget 补了 `WidgetAny`（之前能建不能挂）。

## 6. History（2026-07-31 旧计划摘要，superseded）

旧 PLAN.md 描述：在 Floom 上建跨 Design System 的组件框架，三层架构
（`floom-elements` contract / per-component 实现 crate / Floom core），
`cfg(feature)` 编译期分发 DS，token-first 主题，Phase 1 Material 8 组件 MVP，
Phase 2 Cupertino/Fluent，Phase 3 高级组件，Phase 4 生态工具（Theme Builder / Figma Import / CLI / docs）。
其源码依据 `SKILL.md` 与 `superpowers/plans/2026-07-31-floom-elements.md` 记录的是 v1 世界；
今天的代码已用 v2 engine-widget 架构取代，但命名/token/a11y/testing 等约定仍成立（§4）。