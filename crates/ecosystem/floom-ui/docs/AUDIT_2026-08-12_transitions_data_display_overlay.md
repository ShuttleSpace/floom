# 书面审计 — Data display / Feedback·overlays / Media·motion 组件

**日期**：2026-08-12
**范围**：用户清单中的 16 个组件（Data display 6 + Feedback/overlays 1 + Media/motion 7 + 用户标注"已有"的 DataTable/VirtualScroller 2）
**结论**：16 个组件此前已在 Batches 9/15/16/17/18 完整实现、公开暴露并通过编译。本次审计发现并修复了 **Batch 18 demo 段的 4 个编译错误**（此前被错误的"0 错误"结论掩盖），现全量编译绿灯。

---

## 1. 组件接线 / facade / demo 状态表

每个组件均已具备：三端 widget crate（Material/Cupertino/Fluent，真实 `view()`）、`floom-elements` 中立 facade 文件、`lib.rs` 模块注册、demo 展示段。

| 组件 | 三端 widget crate | facade 文件 | lib.rs 注册 | demo 段 (examples/demo/src/main.rs) | 来源 Batch |
|---|---|---|---|---|---|
| ListItem | `md`/`cupertino`/`fluent`-list_item | `list_item.rs` | `:76` | `list_items` `:1408` | 15 |
| Table | `md`/`cupertino`/`fluent`-table | `table.rs` | `:77` | `table_view` `:1409` | 15 |
| DataIterator | `md`/`cupertino`/`fluent`-data_iterator | `data_iterator.rs` | `:78` | `data_iter_view` `:1410` | 15 |
| Sparkline | `md`/`cupertino`/`fluent`-sparkline | `sparkline.rs` | `:79` | `sparkline_view` `:1411` | 15 |
| Kbd | `md`/`cupertino`/`fluent`-kbd | `kbd.rs` | `:80` | `kbd_view` `:1412` | 15 |
| Code | `md`/`cupertino`/`fluent`-code | `code.rs` | `:81` | `code_view` `:1413` | 15 |
| Overlay | `md`/`cupertino`/`fluent`-overlay | `overlay.rs` | `:83` | `overlay_view` `:1442` | 16 |
| Parallax | `md`/`cupertino`/`fluent`-parallax | `parallax.rs` | `:84` | `parallax_section` `:1545` | 17 |
| InfiniteScroller | `md`/`cupertino`/`fluent`-infinite_scroller | `infinite_scroller.rs` | `:85` | `inf_scroller` `:1509` | 17 |
| Hover | `md`/`cupertino`/`fluent`-hover | `hover.rs` | `:86` | `hover_view` `:1547` | 17 |
| FadeTransition | `md`/`cupertino`/`fluent`-fade_transition | `fade_transition.rs` | `:93` | `fade` `:1557` | 18 |
| ScaleTransition | `md`/`cupertino`/`fluent`-scale_transition | `scale_transition.rs` | `:94` | `scale` `:1573` | 18 |
| ExpandTransition | `md`/`cupertino`/`fluent`-expand_transition | `expand_transition.rs` | `:95` | `expand` `:1589` | 18 |
| SlideTransition | `md`/`cupertino`/`fluent`-slide_transition | `slide_transition.rs` | `:96` | `slide` `:1605` | 18 |
| DataTable（用户注"已有"） | `md`-datatable | `datatable.rs` | `:28` | `dt_view` `:743` | 9 |
| VirtualScroller（用户注"已有"） | `md`/`cupertino`/`fluent`-virtual_scroller | `virtual_scroller.rs` | `:65` | `vscroll_view` `:869` | 9 |

> 用户原始标注与事实对照：
> - *"Overlay 目前仅内部使用，未作为公开组件暴露"* → **与现状不符**。Overlay 早已是公开组件：`overlay.rs` 存在，`lib.rs:83` 有 `pub mod overlay;`，demo 含 `overlay_view`（`:1442`），且 `cargo check` 可正常编入 `floom-elements` facade。
> - *"Table(简单表，已有 DataTable)"* / *"InfiniteScroller(已有 VirtualScroller)"* → 两者均**已同时存在**为独立公开组件，无缺失。

---

## 2. 全量编译验证结果

| 检查项 | 命令 | 结果 |
|---|---|---|
| floom-elements (Material) | `cargo check -p floom-elements --no-default-features --features material` | ✅ 0 errors |
| floom-elements (Cupertino) | `cargo check -p floom-elements --no-default-features --features cupertino` | ✅ 0 errors |
| floom-elements (Fluent) | `cargo check -p floom-elements --no-default-features --features fluent` | ✅ 0 errors |
| floom-demo (Material) | `cargo check -p floom-demo` | ✅ 0 errors（修复后） |
| floom-demo (Cupertino) | `cargo check -p floom-demo --no-default-features --features cupertino` | ✅ 0 errors（修复后） |
| floom-demo (Fluent) | `cargo check -p floom-demo --no-default-features --features fluent` | ✅ 0 errors（修复后） |
| workspace | `cargo check --workspace` | ✅ 0 errors（修复后） |

日志证据：`/tmp/v_dm.log`、`/tmp/v_dc.log`、`/tmp/v_df.log`、`/tmp/v_ws.log` 均 `errors=0 finished=1`；`/tmp/aud_fem.log`、`/tmp/aud_fec.log`、`/tmp/aud_fef.log` 亦 `errors=0 finished=1`。

---

## 3. 修复记录：Batch 18 demo 段 4× `E0277`

### 问题
首次审计时 `cargo check --workspace` 被误报为 0 错误（详见第 4 节"教训"）。真实情况是 `floom-demo` 始终**无法编译**，4 个错误全部位于 Batch 18 新增的 Transitions demo 段：

```
error[E0277]: expected a Fn() closure, found `dyn View`   (×4: fade / scale / expand / slide)
note: required by a bound in `floom_elements::slide_transition::SlideTransition::child`
  --> crates/material/md-slide_transition/src/lib.rs:40:12
```

### 根因
Transition 组件的 `child` 签名为：

```rust
pub fn child<F, IV>(mut self, child: F) -> Self
where F: Fn() -> IV + 'static, IV: IntoView + 'static
```

它把 child 存为 `Option<Box<dyn Fn() -> AnyView>>`，**必须在每次 enter/leave 挂载时重建子视图**（`AnyView = Box<dyn View>` 不可 `Clone`）。因此 `child` 要求传入**闭包工厂**，而非一个值。

而 Batch 18 demo 写成了：

```rust
FadeTransition::new().child(
    label(|| "...".to_string()).style(...).into_any(),  // ← 传了一个 AnyView 值
)
```

`F` 被推断为 `AnyView`，但 `AnyView` 不是 `Fn()`，于是触发 `E0277`。对比：`Overlay::child` 接受 `impl IntoView` 值（其 child 不随挂载重建），所以 Overlay 的同款写法能编译——两套组件 API 在此处不一致，demo 照搬了 Overlay 的写法导致 Transition 段报错。

### 修复
将 4 处 `.child(<AnyView 值>)` 改为 `.child(|| { <IntoView> })` 闭包，并去掉内层 `.into_any()`（让组件内部 `.into_any()` 统一处理）：

```rust
let fade = FadeTransition::new().child(|| {
    label(|| "Content that fades in / out".to_string())
        .style(|s| s.padding(16.0).font_size(16.0))
});
// scale / expand / slide 同理（slide 保留 .direction(SlideDirection::X)）
```

改动文件：`examples/demo/src/main.rs`（仅 demo 段，未触碰任何组件 crate）。修复后 4 个 demo 检查 + workspace 全部 0 错误。

---

## 4. 教训（已写入项目 MEMORY.md）

1. **后台 `cargo check` 必须等真正结束再读日志**。上一轮用 `grep '^error'` 在读到日志的"窗口期"（workspace 仍在编译、demo 报错尚未 flush）误判为 0 错误。本轮改用 `TaskOutput(block=true)` 等待任务真正完成后，才确认 demo 实际失败。读日志前务必确认 `Finished` 行存在。
2. **Transition 组件 `child` 是闭包工厂，不是值**。与 `Overlay::child`（接受 `impl IntoView` 值）不一致——demo 写 Transition 子内容时必须传 `|| { ... }` 闭包。今后新增 Transition 用法或类似"需随挂载重建子视图"的组件时，沿用闭包工厂形态。

---

## 5. 遗留告警（非阻断，不属本次范围）

全量编译产生数百条 `warning`（主要来自 `h_stack`/`v_stack`/`text`/`label`/`empty` 等已弃用 API 的调用，以及若干 `unused_imports` 如 `floom_core_theme::tokens::color::Color`）。均为历史存量，不影响编译与功能，本次未处理。
