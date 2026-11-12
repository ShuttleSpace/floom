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

**306 个 workspace member，全部能解析**（`cargo metadata`: packages=306, workspace_members=306）：

```
应用
  │
根 engine：crates/widgets/floom-widgets（盒子/文本等原语）、floom-core、floom-scene、floom-app …
  │
crates/ecosystem/floom-ui（satellite，306 members）
  ├─ floom-widget-runtime      运行时状态：主题 / DesignSystem / dark signals、布局注册、ripple
  ├─ floom-core-theme          Theme trait + tokens + 每组件 tokens/props + 全局主题状态
  ├─ floom-theme               跨 DS token 结构（Color/Typography/Spacing/Radius/Elevation/Motion、ColorScheme、ThemeScope）
  ├─ floom-design              DS 样式原语（Variant、ControlSize、InteractionState、Props/StyleSet）+ 颜色数学
  ├─ floom-ui-composites       47 个源文件（约 40 个组件）：dialog/popup/overlay/menu/tabs/tree/pickers…
  │                            基于根 engine `floom-widgets` 组合而来，DS 无关
  ├─ floom-material            Material 3 kit：MaterialRoles/MaterialTheme、button 指标/样式、M3 scale、gallery
  ├─ floom-cupertino           Cupertino/HIG kit：CupertinoColors/CupertinoTheme、button、touch target、gallery
  ├─ floom-fluent              Fluent kit：FluentColors/FluentTheme、button、gallery
  ├─ floom-harmony             HarmonyOS kit：HarmonyColors/HarmonyTheme（brand/warning/error/success）、button、gallery
  ├─ floom-testing             snapshot 测试助手（Snapshot、component_snapshot）
  └─ 296 个 per-component kit crate：98 material + 99 cupertino + 99 fluent
                                （薄 token 适配层，import `floom_ui_composites`）
```

**关键点（旧文档都写错了）：**

- **没有 `floom-elements` facade**。它在 "Stage D" 被移除（`Cargo.toml:361-365` 原话：
  "the `floom-elements` re-export layer went with it"）。每个 kit crate 直接依赖
  `floom-core-theme` / `floom-widget-runtime`，不再经一个统一 contract crate。
  因此旧计划"应用只依赖 `floom-elements`、靠 Cargo feature 切换 DS"的用法**当前不成立**。
- **没有 `examples/` 目录，也没有 `floom-demo` crate**。`justfile` 的三个目标
  （`cargo ... run -p floom-demo`）全部**已断**。
- **Harmony 只有 consolidated crate**（`floom-harmony`），没有像 material/cupertino/fluent 那样的
  per-component kit 子 crate。
- 每 DS 的 per-component crate **是薄适配器**；行为共享在 `floom-ui-composites` + engine `floom-widgets`，
  不是旧计划设想的"每组件 crate 自带 spec/props/state/style 实现"。
- `Cargo.toml` 顶部那段注释（"8 有 engine widget、90 待 promote、member 列表是 32"）**整体过期**：
  现在 306 members、`exclude = []`、kit crates 已适配。

## 2. 旧计划 vs 现实

| 2026-07-31 旧计划 | 今天（2026-11-11） |
|---|---|
| 三层：`floom-elements` contract / per-kit adapters / core | 已改为 engine `floom-widgets` → `floom-ui-composites` → per-DS kits；`floom-elements` 已删 |
| 每组件一 crate，内含实现（`spec/props/event/state/tests`） | 296 个 kit crate 仍存在，但是**薄 token 适配器**；共享逻辑在 composites + engine |
| Phase 1 MVP：8 个 Material 组件 | 远超：98 material + 99 cupertino + 99 fluent（+ harmony kit） |
| Phase 2：Cupertino ×8、Fluent ×8 | 远超：各约 99 个 kit crate |
| Phase 3 高级组件：Table/Tree/VirtualList/Chart/Navigation | 大部分已有：`md-table`、`md-treeview`、`md-virtual_scroller`、`md-sparkline`、`md-navigation_drawer`、`md-datatable`、`md-timeline`、`md-calendar` |
| `examples/demo` + `floom-demo` | **不存在**；`justfile` 目标已断 |
| `floom-testing` | 已存在 |
| Phase 4：Theme Builder / Figma Import / CLI Generator / docs site | **未开工** |

## 3. 真实剩余任务（按优先级）

### P0 — 让文档与磁盘一致（否则所有任务列表都在描述不存在的 crate）

- [x] 重写本 `PLAN.md` 为 v2 现状 + 本任务清单（2026-11-11）
- [ ] `docs/COMPONENT_PARITY.md`（77KB）改为 v2：清理 `floom-elements`/`floom-demo`/`examples/demo` 的残留引用
      （约 32 行）；补 `floom-harmony` 与 `floom-ui-composites` 的说明；补 Batch-14 的 5 行
      （ButtonGroup/FAB/RangeSlider/FileInput/OTPInput）进 contract 表；修 `:969-978` 过期的 implementation-order 复选框。
  - 退出：文档不再出现已删除 crate 的可执行引用；数与 `cargo metadata`（306）一致。
- [ ] `docs/demo-gallery-plan.md`：按新架构重写（去掉 `examples/demo` 路径），或明确标为 superseded。
- [ ] `docs/AUDIT_2026-08-12_...md`：标注其前提（`floom-elements`/`floom-demo` 可编译）已不成立；
      保留其中仍有效的记录（`Transition::child` vs `Overlay::child` 的 API 不一致）。
- [ ] `docs/superpowers/plans/2026-07-31-floom-elements.md`（1586 行，全 v1、复选框未勾、路径指向别的机器）：标注 superseded。
- [ ] `Cargo.toml` 顶部过期注释改写为准确描述（306 members、`exclude = []`、kit 已适配）。
- [ ] `justfile`：`floom-demo` 三目标已断 —— 见 P1。

### P1 — 恢复可运行 gallery/demo（组件要可见、可测、可迭代）

- [ ] 在 floom-ui workspace 新建 demo/gallery crate（`floom-demo` 或 `examples/`），
      渲染 4 套 DS 的 gallery；接通 `just material|cupertino|fluent`（并考虑 harmony）。
  - 退出：`cargo run -p <demo> --features <ds>` 能起窗；`cargo test -p <demo>` headless 快照绿。
  - 说明：这是旧文档反复引用、但磁盘上不存在的那个 crate；不建它，`COMPONENT_PARITY`/`demo-gallery-plan` 的"已完成"都无法验证。
- [ ] `demo-gallery-plan.md` 的 A–D 阶段（Props 面板 / API 表 / method demo / 代码片段）在 demo 落地后再排。
- [ ] `justfile` 接通后，把 `just gate ecosystem` 的 demo 测试纳入。

### P2 — 组件 / 引擎缺口（真实、已在文档中点名，非推测）

- [ ] **headless primitives**：把可复用 component 行为抽成无样式原语（`COMPONENT_PARITY.md:666`）。
- [ ] **Row/Col 一等公民**：目前只有 `Grid`，Row/Col 不是独立原语（`COMPONENT_PARITY.md:537`）。
- [ ] **Batch-14 contract 表 5 行补齐**（同上 P0 文档任务的具体子项）。
- [ ] **Harmony 策略**：是否像另三套一样补 per-component kit 子 crate，还是保持 consolidated sample？写清楚并落文档。
- [ ] **警告债**：`h_stack`/`v_stack`/`text`/`label`/`empty` 弃用调用 + unused imports（`AUDIT:107-109`，
      "historical debt, not addressed"；本次未重新验证数量）。
- [ ] **10 个 consolidated crate 补 `//!` 模块文档**（当前 0 个有）。

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

- `cargo metadata --no-deps` → 306 packages / 306 workspace members（本文件所据）。
- `just gate ecosystem`（根仓库）或 CI `ecosystem-check` → `cargo test --workspace`（satellite 全量）。
  **本次 reconcile 未重跑 306 crate 测试**，只做了结构与元数据核对；P0/P1 的"绿"以实际跑通为准。

## 6. History（2026-07-31 旧计划摘要，superseded）

旧 PLAN.md 描述：在 Floom 上建跨 Design System 的组件框架，三层架构
（`floom-elements` contract / per-component 实现 crate / Floom core），
`cfg(feature)` 编译期分发 DS，token-first 主题，Phase 1 Material 8 组件 MVP，
Phase 2 Cupertino/Fluent，Phase 3 高级组件，Phase 4 生态工具（Theme Builder / Figma Import / CLI / docs）。
其源码依据 `SKILL.md` 与 `superpowers/plans/2026-07-31-floom-elements.md` 记录的是 v1 世界；
今天的代码已用 v2 engine-widget 架构取代，但命名/token/a11y/testing 等约定仍成立（§4）。