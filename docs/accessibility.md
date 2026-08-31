# Floem Accessibility (Phase 4)

> Status: **Phase 4 Step 5 — HarmonyOS (OpenHarmony) 屏读桥 (2026-08-12)**。Step 0–4 已完成（中性协议 + 视图接线 + 桌面 accesskit 桥 + iOS VoiceOver 桥 + Android TalkBack 桥）；**HarmonyOS 现已接 N-API 屏读桥**（`src/accessibility/backend_ohos.rs`，与桌面/iOS/Android 共享同一 `PendingAction` 通道，经 N-API 把中性树序列化推到 ArkTS、再注册到 OHOS Accessibility Kit）。
> Capability `BackendCapabilities::accessibility`：桌面 target（macos/linux/windows）`true`；iOS `true`；Android `true`；**HarmonyOS 现已 `true`**（`ohos_capabilities()` cfg 翻开）。四端（桌面三 + iOS + Android + OHOS）屏读桥齐备。
>
> **Step 5 验证状态（trust-but-verify）**：a11y 后端 napi 表面经**隔离 probe crate** 针对真实 `aarch64-unknown-linux-ohos` target + napi 2.16.17 编译 **0 error**（见 §4.7）。整 crate `cargo check --target aarch64-unknown-linux-ohos` 受 **Phase 3 依赖/sysroot 缺口** gate（`tgui-winit-ohos` retarget 已通过 `[patch.crates-io]` → `../tgui-winit-ohos-fork` 应用，非 gate；真正卡点 = `accesskit_unix`/`muda(gtk)` 的 glib-sys【本步已修】、随后 `yeslogic-fontconfig-sys` 的 fontconfig pkg-config【OHOS 无 sysroot】、wgpu vulkan 链接器），独立于 Step 5；本步已顺带修掉一个真实 OHOS 编译 blocker（glib-sys）。

## 1. 目标与范围

给 floem 加工业级无障碍能力：让屏幕阅读器（VoiceOver / TalkBack / NVDA·ORCA / Narrator）能感知并操作 floem 应用。

Phase 4 的设计**刻意镜像 embedder 子系统**（`src/embedder/`）：先建一个**平台无关的中性协议**（`AccessNode` / `AccessTree` / `AccessTreeUpdate` + Rust 适配 trait `AccessibilityBackend`），后续再按平台接桥。本协议不依赖任何具体平台或窗口库（纯 `std` + `bitflags` + `kurbo`），因此编译于所有 target 无差异。

调研结论（2026-08-11）：floem 此前**无任何无障碍实现**——只有 `BackendCapabilities::accessibility: bool` 占位 flag（默认 false）。winit fork 也未暴露无障碍 API（grep 命中全是 changelog 历史），故无障碍树必须自建，再各自桥接：
- macOS / iOS: `NSAccessibility`（`NSAccessibilityElement`）
- Android: `AccessibilityNodeInfo` / `AccessibilityNodeProvider`
- Linux: AT-SPI2 (D-Bus)
- Windows: UI Automation (UIA)

## 2. 协议设计

### 2.1 数据模型（`src/accessibility/node.rs`, `role.rs`, `state.rs`）

- `AccessNodeId(u64)` — 协议级稳定节点 id（跨 `AccessTreeUpdate` 不变，便于增量 patch 原生树）。
- `AccessRole`（`#[non_exhaustive]`）— 平台无关角色：Button / CheckBox / RadioButton / Switch / Slider / TextField / Heading / List / ListItem / Tab / Menu / Image / Alert / … 覆盖 WAI-ARIA + 各平台原生角色词汇。
- `AccessState`（`bitflags`，`#[non_exhaustive]`）— FOCUSED / DISABLED / HIDDEN / SELECTED / CHECKED / EXPANDED / COLLAPSED / READ_ONLY / REQUIRED / INVALID / VERTICAL / HORIZONTAL / …（ARIA 状态子集）。
- `AccessAction`（`#[non_exhaustive]`）— 屏幕阅读器可请求的动词：Focus / Default(激活) / Increment / Decrement / ShowContextMenu / ScrollIntoView / Expand / Collapse / Select / SetValue。
- `AccessNode` — 单节点：`id` / `role` / `label` / `value` / `description` / `states` / `actions` / `bounds: Option<Rect>`(逻辑像素) / `children: Vec<AccessNodeId>`。提供 builder（`with_label` / `with_value` / `with_states` / `with_actions` / `with_bounds` / `with_children`）+ `supports(action)`。

### 2.2 树与增量更新（`src/accessibility/tree.rs`）

- `AccessTree` — `HashMap<AccessNodeId, AccessNode>` + `root` + `focus`。方法：`get` / `contains` / `iter` / `len` / `focused` / `apply(update)` / `node_at_point(point)`。
- `AccessTreeUpdate` — 可 diff 的增量：`root: Option` / `nodes: Vec`(upsert) / `removed: Vec` / `focus: Option`；`is_empty()`。`AccessTree::apply` 在删除时同步清理父节点的悬挂 child 引用，并清除指向被删节点的 `root` / `focus`。
- `node_at_point` — 命中测试返回**最内层**（面积最小）的包含节点，用于触摸探索 / 按坐标聚焦。

### 2.3 后端适配 trait（`src/accessibility/backend.rs`）

`AccessibilityBackend`（镜像 `EmbedderWindow` / `PlatformServices`）：
- `update_tree(&mut self, AccessTreeUpdate)` — core 推送树 diff；默认 no-op。
- `focused_node(&self) -> Option<AccessNodeId>` — 默认 `None`。
- `node_at_point(&self, Point) -> Option<AccessNodeId>` — 默认 `None`。
- `perform_action(&mut self, node, action, payload) -> bool` — 平台屏读请求的动作，返回是否处理；默认 `false`。

`MockAccessibility` — 内存后端（测试 / headless 用）：保存 `AccessTree`、记录每次 `perform_action`（`performed_actions()` / `clear_actions()`），`perform_action` 仅接受节点声明支持的 action。

## 3. Capability（`src/embedder/capabilities.rs`）

- `BackendCapabilities::accessibility: bool` — 此前纯摆设（同 `safe_area` 在 Step 9 之前）。
- **新增 `with_accessibility()` builder**：后端接好 `crate::accessibility` 桥后调用置 `true`。当前 `winit_default()` / `headless()` 仍 `false`（无真实屏读桥）。

## 4. 模块布局

```
src/accessibility/
  mod.rs        — 模块文档 + re-export
  role.rs       — AccessRole
  state.rs      — AccessState (bitflags) + AccessAction
  node.rs       — AccessNodeId + AccessNode
  tree.rs       — AccessTree + AccessTreeUpdate (含 7 测试)
  backend.rs    — AccessibilityBackend trait + MockAccessibility (含 3 测试)
  attrs.rs      — AccessibilityAttrs (每视图声明属性：role/label/value/description/actions/hidden)
  bridge.rs     — core 遍历 ViewStorage + ViewState + LayoutChanged + focus 产出 AccessTreeUpdate (含 2 测试)
```
`src/lib.rs` 注册 `pub mod accessibility;`（line 180，紧邻 `action`）。

## 4.1 Step 1 — 视图属性接线 + core 发射（2026-08-11）

把 Step 0 协议接到真实 UI：**声明侧** + **产出侧**。

### 声明侧（`src/view/state.rs` + `src/view/id.rs`）
- `ViewState` 新增 `pub(crate) accessibility: Option<AccessibilityAttrs>`（默认 `None`，表示用默认：仍暴露为 `AccessRole::Unknown`）。
- `ViewId` 新增 builder（镜像 `set_cursor`）：`accessibility_role` / `accessibility_label` / `accessibility_value` / `accessibility_description` / `accessibility_actions` / `accessibility_skip(hidden)`。内部 `state().borrow_mut().accessibility.get_or_insert_with(Default::default)`。

### 存储侧（`src/window/state.rs`）
- `WindowState` 新增 `pub(crate) accessibility: AccessTree`（权威中性树）与 `pub(crate) accessibility_backend: Option<Rc<RefCell<dyn AccessibilityBackend>>>`（平台桥，未接时为 `None`）。
- 新增 `set_accessibility_backend(...)` setter（Step 2 由平台后端调用）。

### 产出侧（`src/accessibility/bridge.rs` + `src/window/handle.rs`）
- `build_access_tree_update(storage, root_view_id, focused_view) -> AccessTreeUpdate`：从 `ViewStorage::children` 自窗口根遍历；读每视图 `ViewState.accessibility`（role/label/value/description/actions）+ `ViewState.layout.box_window()`（窗口坐标 `Rect` 作 `bounds`）+ focus（`ElementId::owning_id()` 比对）；`AccessNodeId` 取 `ViewId.data().as_ffi()`（稳定 u64）。
- **隐藏再父化**：`AccessibilityAttrs::hidden` 的视图从树中删除，其 children 重挂到最近的非隐藏祖先（镜像 ARIA `role="presentation"`/`none`）。
- 发射点：在 `WindowHandle::process_update_no_paint` 末尾（layout commit 之后、`UpdatePhaseEvent::Complete` 路由之前）调用新增的 `update_accessibility_tree()`：构建 update → `window_state.accessibility.apply(update)` → 若 `accessibility_backend` 为 `Some` 则 `backend.update_tree(update)` 转发给平台桥。
- 当前每帧都重建并全量 apply（Step 2 之后可加 dirty 节流）；`update.is_empty()`（根视图被隐藏）时跳过。

### 关键约定 / 坑
- `AccessNodeId` 用 `ViewId` 的 `KeyData.as_ffi()`，跨 update 稳定，便于平台树增量 patch。
- `bounds` 取 `LayoutChanged::box_window()`（窗口坐标、不含 transform）；若需视觉位置用 `VisualChanged`（Step 2 视平台而定）。
- 每帧全量重建，无 dirty 跟踪（Step 1 正确性优先；节流留待后续）。
- overlay（`ViewStorage::overlays`）暂未并入 a11y 树（Step 1 只走主 `root_view_id`）；菜单/弹层无障碍留 Step 2+ 细化。

### 4.2 Step 2 — 桌面后端桥 (2026-08-11)

把 Step 0/1 产出的中性树真正桥到原生屏幕阅读器。本步经调研**放弃直接手搓原生 API**，改用 **accesskit** 作平台传输层（用户确认「accesskit 适配器」方案）。

#### 架构转向：accesskit 中性传输层（用户确认）

调研发现 `atspi` / `uiautomation` crate 都是 AT-SPI2 / UIA 的**客户端**库——只能"消费"无障碍、无法"提供"无障碍（提供方需手搓 zbus D-Bus 对象或 COM 接口）。因此改走 accesskit：

- `accesskit` 0.24 定义中性 `Tree`/`Node`/`NodeId`/`Role`/`Action`/`TreeUpdate`/`ActionHandler`/`ActionRequest`/`ActionData`。
- 平台适配器是**独立 crate**（非 `accesskit::platform`）：`accesskit_macos` 0.26 / `accesskit_unix` 0.22 / `accesskit_windows` 0.34，三者均依赖 `accesskit ^0.24.1`。
- 三平台统一经 accesskit 覆盖：macOS→NSAccessibility、Linux→AT-SPI2、Windows→UIA。
- floem 用 `winit` 的 **lapce fork**（`floem-winit`），故 `accesskit_winit`（依赖上游 `winit`）**不可直接用** → 直接驱动 per-OS `Adapter`，绕开 winit 集成层。

#### 平台无关映射层 `src/accessibility/translate.rs`（新建，单测）

唯一把中性 `AccessNode`/`AccessTree` 翻译成 accesskit 类型的地方，可在 macOS 本机单测：

- `role_to_accesskit(AccessRole) -> accesskit::Role`：全变体映射（Panel→Pane、Divider→GenericContainer、ProgressBar→ProgressIndicator、TextField→TextInput、SearchField→SearchInput、Grid→Grid、Cell→GridCell…）。
- `action_to_accesskit(AccessAction) -> Option<accesskit::Action>`：`Select`→`None`（accesskit **无** `Select` action）；`Default`→`Click`；`Focus`→`Focus`；`Increment`/`Decrement`/`ShowContextMenu`/`ScrollIntoView`/`Expand`/`Collapse`/`SetValue` 直映射。
- `build_accesskit_node(&AccessNode) -> accesskit::Node`：`Node::new(role)` + `set_label/set_value/set_description` + `set_children` + 状态→`set_disabled/set_hidden/set_selected/set_expanded/set_multiselectable/set_toggled` + `add_action` + `set_bounds(Rect{f64})`。
- `build_tree_update(&AccessTree) -> accesskit::TreeUpdate`：`root = AccessNodeId::ROOT`、`focus = tree.focus`(或 root)、`nodes` 全量、`tree = Some(Tree::new(root))`、`tree_id = TreeId::ROOT`。
- 含 3 个 `#[cfg(test)]`：`role_roundtrip_known` / `action_select_has_no_accesskit_equivalent` / `build_node_maps_states_and_bounds`。

#### macOS `accesskit_macos`（已接，host 验证）

- `src/accessibility/backend_nsaccessibility.rs`：`MacOSAccessibilityBackend` 包装 `accesskit_macos::Adapter`。
  - `new(ns_view: *mut c_void)`：null 返回 `None`；否则 `unsafe { Adapter::new(ns_view, true, MacOSActionHandler { tx }) }`（主线程，裸 `NSView` 指针；NSView 子类化与 `NSAccessibility` 全部由 accesskit_macos 接管）。
  - `update_tree(update)`：`tree.apply` 同步中性缓存 → `adapter.update_if_active(|| build_tree_update(&tree))` → `events.raise()`（`QueuedEvents`）。
  - `focused_node` / `node_at_point` 委托中性 `AccessTree` 缓存（无需重查原生树）。
  - `MacOSActionHandler: ActionHandler`：`do_action` 反向映射 `accesskit::Action → AccessAction`（payload 取 `ActionData::Value`），经 `mpsc::Sender<PendingAction>` 回传 core。

#### Linux `accesskit_unix`（已接，cfg 编译验证）

- `src/accessibility/backend_atspi.rs`：`LinuxAccessibilityBackend` 包装 `accesskit_unix::Adapter`。
  - `Adapter::new(activation, action, deactivation)` — **不收 zbus Connection**（适配器自拥 session bus）。
  - `SharedTree = Arc<Mutex<AccessTree>>`：`LinuxActivationHandler::request_initial_tree()` 在屏读器首次连入时返回 `build_tree_update(&tree)`；`LinuxDeactivationHandler::deactivate() {}` no-op。
  - `update_tree`：`tree.lock().apply(update)` + `adapter.update_if_active(|| build_tree_update(&tree))`。
  - action 回传同 macOS（`mpsc` channel）。

#### Windows `accesskit_windows`（已接，cfg 编译验证）

- `src/accessibility/backend_uia.rs`：`WindowsAccessibilityBackend` 包装 `accesskit_windows::Adapter`。
  - `new(hwnd: *mut c_void)`：null 返回 `None`；否则 `let hwnd = HWND(hwnd as *mut _); Adapter::new(hwnd, true, WindowsActionHandler { tx })`（`HWND` 是 accesskit_windows 的 newtype 包装 `*mut _`）。
  - **`WM_GETOBJECT` 转发（已实现）**：`Adapter::handle_wm_getobject` 必须由窗口过程手动转发（accesskit_windows 无内置子类）。`new` 中对 HWND **子类化**：`SetWindowLongPtrW(GWLP_USERDATA, Arc::into_raw(data))` 存 `SubclassData`，再 `SetWindowLongPtrW(GWLP_WNDPROC, subclass_proc)` 替换窗口过程。`subclass_proc` 拦截 `WM_GETOBJECT` → `adapter.handle_wm_getobject(WPARAM, LPARAM, &mut activation_handler)` → `result.into()` 转 `LRESULT` 返回；其余消息 `CallWindowProcW` 链回原过程；`WM_NCDESTROY` 时还原 `GWLP_WNDPROC` 并 `Arc::from_raw` 释放。`Adapter` 与 `WindowsActivationHandler` 经 `Arc<Mutex<>>` 在 backend 与 WndProc 间共享；初始树用独立的 `cached_tree` Mutex 提供，避免与 `update_if_active` 的锁序死锁。
  - action 回传同 macOS。

#### 接线（core ↔ 平台桥）

- `mod.rs::native_backend_for_window`：macOS 收 `ns_view`、Windows 收 `hwnd`、Linux 无参；各自 `.map` 闭包显式标注 `Rc<RefCell<dyn AccessibilityBackend>>`。
- `WindowHandle::new`：macOS 传 `a11y_ns_view`（`raw_window_handle::AppKitWindowHandle.ns_view.as_ptr()`）、Windows 传 `a11y_hwnd`（`raw_window_handle::Win32WindowHandle.hwnd.get() as *mut c_void` —— **注意 rwh 0.6 的 Win32 变体叫 `Win32`（非 `Windows`），`hwnd` 是 `NonZero<isize>` 用 `.get()`**）；iOS 两者皆无。
- `WindowHandle::update_accessibility_tree()`：在 `backend.update_tree(update)` 之后 drain `poll_actions()` → `window_state.handle_accessibility_action(action)`（每帧）。
- `WindowState::handle_accessibility_action`：已落地真实派发（见 4.3）。
- `PendingAction = (AccessNodeId, AccessAction, Option<String>)`（`backend.rs` 类型别名）；`AccessibilityBackend::poll_actions` 默认空、由接了 channel 的后端覆盖。

#### Capability

- `BackendCapabilities::winit_default()` 在桌面 target（macos/linux/windows）cfg 翻开 `accessibility = true`（Step 2 时）；**移动端在 Step 3（iOS）/ Step 4（Android）已分别由 `ios_capabilities()` / `android_capabilities()` cfg 翻开 `true`**（见 §4.4 / §4.5）。`winit_default()` 与 `headless()` 本身仍 `false`（无真实屏读桥时）。

#### 关键坑（accesskit 0.24 实战）

- **`atspi` / `uiautomation` 是客户端库**，无法"提供"无障碍 → 死路，已用 accesskit 绕开。
- **per-OS adapter 是独立 crate**（`accesskit_macos`/`accesskit_unix`/`accesskit_windows`），非 `accesskit::platform`；三者均依赖 `accesskit ^0.24.1`。
- **`ActionRequest` 无 `target` 字段** → 用 `target_node`（`NodeId`）；可用字段为 `action`/`target_tree`/`target_node`/`data`。`NodeId` 是 `NodeId(pub NodeIdContent)`，`NodeIdContent=u64`，故 `request.target_node.0` 取回 u64 → `AccessNodeId(u64)` 回合。
- **焦点是树级概念**：`TreeUpdate::focus: NodeId` 每次更新必填；无 per-node `set_focused`。`AccessState::FOCUSED` → `TreeUpdate.focus`；`CHECKED` → `set_toggled(Toggled::True)`。
- `accesskit_macos::Adapter::new` 收 `*mut c_void`（不依赖 objc2 0.6 类型），故 accesskit 的 `objc2 0.5` 与 floem 的 `objc2 0.6` 可共存于依赖图。
- `accesskit_windows::HWND` 是 newtype：**`HWND(hwnd as *mut _)`** 构；不要当裸指针传。
- `accesskit_unix::Adapter` **自拥 session bus**（不传 `zbus::Connection`）；初始树由 `ActivationHandler::request_initial_tree` 懒提供，故 `SharedTree = Arc<Mutex<AccessTree>>` 在激活处理器与 backend 间共享。
- `accesskit_winit` 不可用（lapce winit fork）→ 直接驱动 per-OS `Adapter`，自管 `update_if_active` + `ActionHandler`。
- Cargo.toml：`accesskit = "0.24"` 放 `any(macos,linux,windows)` 组合块；`accesskit_macos/accesskit_unix/accesskit_windows` 分别进各自 target 块（**勿重复 key**）。

#### 接线再确认：`poll_actions` 每帧 drain

`WindowHandle::update_accessibility_tree()` 在 `backend.update_tree(update)` 之后遍历 `backend.poll_actions()`，逐个调用 `WindowState::handle_accessibility_action(action)`。动作经 `mpsc` 从平台 `Adapter` 的 `ActionHandler` 回传（见 4.2），落点即下方 Step 3。

### 4.3 Step 2 收尾 — 真实 action 派发 (2026-08-11)

屏读器请求的动作（经 `poll_actions` 回传的 `PendingAction`）现在**真正驱动 core 视图**，而不只是被捕获后丢弃。

- 落点：`WindowState::handle_accessibility_action(PendingAction)`（`src/window/state.rs`）。
- 节点→视图还原：`AccessNodeId(u64)` 即 floem 给视图 `ViewId` 的 `KeyData.as_ffi()`；`ViewId::from(KeyData::from_ffi(node.0))` 还原 `ViewId`，无需在中性树里携带 Rust 闭包。
- 动作映射（真实路由，跨平台）：
  - `Focus` → `view_id.request_focus()`（键盘焦点，真实生效）。
  - `Default`（激活/点击）→ `view_id.route_event(Event::Interaction(InteractionEvent::Click), RouteKind::Directed { target, phases: Phases::TARGET })`：定向派发 `Click`，触发视图的 `Click` 监听器（如 `Button::action`），与真实鼠标点击完全一致。
  - `ShowContextMenu` → 同上但 `InteractionEvent::SecondaryClick`：打开视图已注册的上下文菜单。
  - `route_event` 是 `ViewId` 的**公开**方法（经 `UpdateMessage::RouteEvent` 入队，下一帧消息循环处理，与 `request_focus` 同机制）。
- 语义动作（4.3.1）与焦点状态（4.3.2）见下。

#### 4.3.1 语义动作 per-view hook (2026-08-11)

`Increment`/`Decrement`/`Expand`/`Collapse`/`SetValue`/`ScrollIntoView` 在 floem 无通用事件，故走**每视图回调**：

- `AccessibilityAttrs` 新增 `on_action: Option<Arc<dyn Fn(AccessAction, Option<String>) + 'static>>`（`AccessibilityActionHandler` 类型别名）；因含闭包，`AccessibilityAttrs` 改为手动实现 `Clone`/`Debug`。
- `ViewId::accessibility_action(handler)`（`src/view/id.rs`）：向视图注册回调，典型用法是 slider/spinbutton 处理 `Increment`/`Decrement`/`SetValue`、disclosure 处理 `Expand`/`Collapse`。
- `handle_accessibility_action` 中这些分支：`view_id.state().borrow().accessibility.on_action.clone()` 取出 `Arc` 后释放 `RefCell` 借用，再 `handler(action, payload)` 调用（`SetValue` 的 payload 即新值字符串）。视图的响应式信号在闭包内更新，无需 core 改动。无回调则静默忽略（不 panic、不破坏状态）。

#### 4.3.2 焦点状态回传 macOS / Windows (2026-08-11)

- `AccessibilityBackend` trait 新增默认 `update_focus_state(bool)`（默认 no-op，Linux AT-SPI 走 `TreeUpdate.focus` 无需）。
- macOS 后端：`adapter.update_view_focus_state(focused)` + `events.raise()`；Windows 后端：`adapter.update_window_focus_state(focused)` + `events.raise()`。
- 接线：`WindowHandle::focused(focused)`（`src/window/handle.rs`，处理 `WindowEvent::Focused`）在派发 `FocusGained`/`FocusLost` 后调用 `backend.update_focus_state(focused)`，让 VoiceOver / Narrator 与 floem 窗口焦点同步。

#### 4.3.3 Windows `WM_GETOBJECT` 转发 (2026-08-11)

Narrator 要连上 UIA，必须把 `WM_GETOBJECT` 转给 `accesskit_windows::Adapter`。因 `accesskit_winit` 不可用（winit fork），在 `WindowsAccessibilityBackend::new` 中对 HWND **子类化**（详见 4.2 `backend_uia.rs` 描述）：`GWLP_USERDATA` 存 `Arc<SubclassData>`（含原窗口过程 + 共享 `Adapter` + `ActivationHandler`），`GWLP_WNDPROC` 换为 `subclass_proc`；`subclass_proc` 转发 `WM_GETOBJECT` 并链回原过程，`WM_NCDESTROY` 还原并释放。`Adapter` 经 `Arc<Mutex<>>` 在 backend（`update_tree`/`update_focus_state`）与 WndProc（`handle_wm_getobject`）间共享；初始树走独立 `cached_tree` Mutex，规避锁序死锁。`windows-sys`（仅 windows target）提供 `SetWindowLongPtrW`/`CallWindowProcW` 等裸 Win32 调用。

### 4.4 Step 3 — iOS VoiceOver 桥 (2026-08-12)

iOS **无 accesskit 后端**（`accesskit` 仅覆盖 macos/linux/windows），故直接走 iOS 原生 `UIAccessibility` 协议：子类化 `UIAccessibilityElement`，把中性树镜像到窗口 `UIView`（作根无障碍容器）之下，并把 VoiceOver 动作（activate / increment / decrement / scroll）经与桌面**相同的 `PendingAction` 通道**回传 core。代码全部位于新文件 `src/accessibility/backend_ios.rs`（`#[cfg(target_os = "ios")]` 门控）。

#### 为什么不用 accesskit / 不用 `NSAccessibility` 协议 trait

- iOS 上 `UIAccessibility` 相关 API 在 `objc2-ui-kit` 中以 **category traits** 暴露（`NSObjectUIAccessibility` / `NSObjectUIAccessibilityContainer`），**没有独立协议 struct**；且这些 category trait 只 `impl` 在 `UIAccessibilityElement` 精确类上，子类（`FloemA11yElement`）不会自动获得，因此属性 setter 一律用 `msg_send!` 直接发，绕过 trait。
- 动作方法（`accessibilityActivate` / `accessibilityIncrement` / `accessibilityDecrement` / `accessibilityScroll:`）在 `define_class!` 的**固有 `impl` 块**（写 `impl FloemA11yElement {`，不能加 `unsafe` 前缀）里用 `#[unsafe(method(sel))]` override；签名里**不要写 `mtm: MainThreadMarker` 参数**（否则宏会尝试把 `MainThreadMarker` 当编码参数 → `E0277`），VoiceOver 调用这些方法时不需要我们显式收 `mtm`。

#### 子类 `FloemA11yElement`（`define_class!`）

- `#[unsafe(super(UIAccessibilityElement))]` + `#[thread_kind = MainThreadOnly]` + `#[ivars = A11yIvars]` + `#[name = "FloemA11yElement"]`。
- ivars `A11yIvars { node_id: u64, actions: u32, tx: *const mpsc::Sender<PendingAction> }`：`tx` 必须是**裸指针**——`mpsc::Sender` 不是 `Encode`，不能塞进 ivar 结构；生命周期由 backend 拥有 element 保证（element 不 outlive backend）。
- `accessibilityActivate` → `AccessAction::Default`（点击/激活）、`accessibilityIncrement`→`Increment`、`accessibilityDecrement`→`Decrement`、`accessibilityScroll:`→`ScrollIntoView`；均按 `actions` 位掩码（ivars）判定是否转发，经 `send_action` 裸指针发 `PendingAction`。`Expand`/`Collapse`/`SetValue`/`ShowContextMenu`/`Select`/`Focus` 在 iOS 无对应 UIAccessibility 动作，不进默认动作（与桌面一致，走 per-view `on_action` 回调）。

#### `IosAccessibilityBackend`

- 字段：`container: Retained<UIView>`（窗口 `UIView`，根无障碍容器）、`elements: HashMap<AccessNodeId, Retained<FloemA11yElement>>`、`tree: AccessTree`（中性缓存，供 `focused_node`/`node_at_point`）、`tx/rx: mpsc`、`prev_root`/`prev_children`（拓扑缓存）。
- `new(ui_view)`：`Retained::retain(ui_view as *mut UIView)` 持有 UIView；`MainThreadMarker::new()?` 主线程要求；null 返回 `None`。
- `update_tree`：仅主线程触 OC；先 `tree.iter()` 取 `old_ids`，`tree.apply(update)` 后取 `new_ids`；差集创建/删除 element（`FloemA11yElement::new` 用 `initWithAccessibilityContainer:`）；逐节点 `update_element` 刷新 label/value/hint/traits/`isAccessibilityElement`/frame（逻辑像素经 `UIAccessibilityConvertFrameToScreenCoordinates` 转屏幕坐标）；**仅当拓扑（root 或任一节点 children）变化**才 `rebuild_containers` + `UIAccessibilityPostNotification(UIAccessibilityLayoutChangedNotification, …)`，避免每帧刷屏读器元素身份抖动。
- `rebuild_containers`：UIView 的 `accessibilityElements = [root element]`（经 `msg_send!` 发 `setAccessibilityElements:`，因 UIView 不 conform 容器 category 必须裸发），各节点 children 设为该节点 `accessibilityElements` 并反向 `setAccessibilityContainer`。
- `poll_actions` 全 drain `rx`（与桌面一致，全取比只取一个更稳）。

#### 接线（core ↔ iOS 桥）

- `mod.rs`：`#[cfg(target_os = "ios")] pub(crate) mod backend_ios;` + `native_backend_for_window(ui_view: *mut c_void) -> Option<Rc<RefCell<dyn AccessibilityBackend>>>` 返回 `IosAccessibilityBackend::new(ui_view)` 包装。
- `app/handle.rs`：`RawWindowHandle::UiKit(a) => a.ui_view.as_ptr()` 取 UIView 指针，作 `WindowHandle::new` 的 `#[cfg(target_os = "ios")] a11y_ui_view` 实参。
- `window/handle.rs`：`WindowHandle::new` 签名加 `#[cfg(target_os = "ios")] ui_view: *mut c_void`；函数体内 `if let Some(backend) = native_backend_for_window(ui_view) { window_state.set_accessibility_backend(Some(backend)); }`。
- `embedder/ios.rs`：`ios_capabilities()` 的 `accessibility: true`（原 `false`）。

#### 关键坑（objc2 0.6.4 + objc2-ui-kit 0.3.2 实战）

- `unsafe impl` 宏只接受**单一 identifier**：协议（如 `NSObjectProtocol`）必须用 `use` 引入作用域后写裸名 `unsafe impl NSObjectProtocol for T {}`；路径写法（如 `objc2_foundation::NSObjectProtocol`）直接报 `no rules expected ::`。
- `define_class!` 的固有 impl 块必须写 `impl T {`（**不能** `unsafe impl T {`），否则 `no rules expected {`；带协议才写 `unsafe impl Protocol for T {}`。
- `#[thread_kind = MainThreadOnly]` 需 `use objc2::MainThreadOnly;`（与 `MainThreadMarker` 一起从 `objc2` 导出）。
- `Retained<T>` 没有 `.as_ptr()` 实例方法（`as_ptr` 是关联函数 `Retained::as_ptr(&self)`），且 `&Retained<T>` 指针强转会拿到 **Retained 包装**而非对象。取对象指针改 `as_any(&*retained)`（`&*Retained<UIView>` = `&UIView`，再 `as *const T as *const AnyObject`）。
- `UIAccessibilityTrait*` 等是 **extern static 值**，读取需要 `unsafe`；`UIAccessibilityLayoutChangedNotification`（extern static）+ `UIAccessibilityPostNotification`（extern `C-unwind` fn）同理，调用点必须包 `unsafe {}`。
- `UIAccessibilityElement` 属性 setter 在 category trait 上、子类不可见 → 一律 `msg_send!` 直接发；`setAccessibilityContainer:` 直接定义在 `UIAccessibilityElement` 上（`unsafe fn`，子类经 Deref 可调用）。
- `setAccessibilityFrame:` 等需要 `mtm: MainThreadMarker` 尾参的方法，**不用 trait 调用**（子类无 impl），故统一改 `msg_send!` 绕过 mtm。
- `initWithAccessibilityContainer:` 是 init 族，`msg_send![super(obj, UIAccessibilityElement::class()), initWithAccessibilityContainer: container]` 按返回类型注解 `Retained<Self>` 自动消费 `Allocated<T>`。
- 整个模块 `#![allow(unsafe_op_in_unsafe_fn)]`：本模块是纯 UIKit FFI，所有函数均已 `unsafe fn`，工具链对 `unsafe fn` 内操作的 lint 噪声在此统一压制。

### 4.5 Step 4 — Android TalkBack 桥 (2026-08-12)

Android **无 accesskit 后端**（`accesskit` 仅覆盖 macos/linux/windows），且 Android 暴露无障碍的 [`AccessibilityNodeProvider`] **必须是 Java `View` 层级中的 Java 对象**——无法纯 Rust 实现。因此本步采用「**Rust 注入一个透明 Java `View` + JNI 入口**」双文件方案：

- `src/accessibility/backend_android.rs`（`#[cfg(target_os = "android")]` 门控）—— 中性树 ↔ `AccessibilityNodeInfo` 翻译 + JNI 入口（C-ABI，`#[unsafe(no_mangle)]`）。
- `src/platform/android/FloemAccessibilityView.java`（包名 `dev.floem.android`，**load-bearing**，须与 JNI 符号一致）—— 透明 `View`，`getAccessibilityNodeProvider()` 返回内部 `FloemProvider extends AccessibilityNodeProvider`，把 `createAccessibilityNodeInfo` / `findAccessibilityNodeInfosByText` / `performAction` 回调 forward 到 Rust 的 JNI 入口。

#### 为什么是「注入 Java View」而不是纯 Rust

Android 的屏读器（TalkBack）只认挂在 `View` 树上的 `AccessibilityNodeProvider`，provider 实例由 `View.getAccessibilityNodeProvider()` 返回、且必须是 Java 对象（JVM 反射调用其方法）。Rust 没有对应的接口实现机制，因此 floem 自己 ship 一个极简 Java `View`，在 Rust 侧经 `AndroidJni::with_env` 把它 `new_object` 出来并 `addView` 进 `Activity.getWindow().getDecorView()`（`inject_a11y_view()`）。该 View：**透明**（`setWillNotDraw(true)`）、**不消费触摸**（`setClickable(false)`/`setFocusable(false)`——永远不抢渲染 surface 下方的输入）、**`IMPORTANT_FOR_ACCESSIBILITY_NO`**（其本身不进树，只作 provider 宿主）；唯一职责是 vend 一个 `AccessibilityNodeProvider`，其回调 forward 到本模块的 native 入口。

#### 单例 `ANDROID_A11Y`（`static Mutex<Option<AndroidA11yInner>>`）

Android 上 floem 只有**一个窗口**，故用模块级单例：它同时被 `AccessibilityBackend` impl（core 侧 `update_tree`/`poll_actions`/`focused_node`/`node_at_point`）与 JNI 入口（无 `self` 指针）共享，tree + action channel 都在里面。`AndroidAccessibilityBackend` 是零尺寸标记，委托单例。`AndroidA11yInner { tree, tx/rx: mpsc, view: Option<GlobalRef>, root, children: HashMap, parent: HashMap, actions: Option<ActionConsts> }`。

`ActionConsts` 惰性缓存 `AccessibilityNodeInfo.ACTION_*` int 常量（`load` 用 `env.get_static_field(cls, name, "I")?.i()?` 一次取齐 click/focus/long_click/scroll_forward/scroll_backward/increment/decrement/set_text/expand/collapse/select），避免每帧查 JVM。

#### JNI 入口（4 个，`#[unsafe(no_mangle)]`）

符号遵循 JNI 自动解析约定 `Java_<fully_qualified_class_with_underscores>_<method>`，类 `dev.floem.android.FloemAccessibilityView`。每个入口先把 `JNIEnv::from_raw(ptr)`（`unsafe`）与 `JObject::from_raw(jobject)` 还原，再把任意 `jni` error 转成安全默认值（null / `false`），**绝不跨 JNI 边界 panic**（会 abort JVM）：

- `nativeRegisterView(env, this)` — `env.new_global_ref(this)` 存进单例 `view`（供 `notify_changed` 发 content-changed 事件）。`GlobalRef` 是 `Clone` 且 `Send+Sync`，可跨线程持有。
- `nativeCreateNodeInfo(env, this, virtual_view_id) -> jobject` — `virtual_view_id == HOST_VIEW_ID(-1)` 走 `build_host_info`（覆盖整窗、把树 root 作为唯一虚拟子）；否则 `create_node_info_locked` 重建该节点 `AccessibilityNodeInfo` 后 `into_raw()`。
- `nativePerformAction(env, this, virtual_view_id, action, arguments) -> jboolean` — `perform_action_locked` 把 Android action int 反查回 `AccessAction` 并经 `tx.send((id, a, payload))`；`ACTION_SET_TEXT` 的 payload 从 `arguments` Bundle 读 `android.view.accessibility.extra.DATA_TEXT`（`read_bundle_string`）。
- `nativeFindNodesByText(env, this, text, virtual_view_id) -> jobject` — `find_nodes_locked` 按 label/value/description 大小写不敏感匹配，返回 `ArrayList<AccessibilityNodeInfo>`。

#### 树 → `AccessibilityNodeInfo` 翻译（`create_node_info_locked`）

每帧 `update_tree` 收到**全量**中性树并 `tree.apply(update)`；只有**拓扑**（root 或任一节点 children）变化才 `notify_changed()`（经 `view.requestA11yRefresh()` → `post(() -> sendAccessibilityEvent(TYPE_WINDOW_CONTENT_CHANGED))`，在 UI 线程外安全触发）。**per-node info 每次按需重建**，故属性变化（label/value/bounds/state）无需显式通知——TalkBack 重新请求时自然拿到最新值。翻译细节：

- **文本**：`label` 优先、否则 `value`，经 `setText`；`description` 经 `setContentDescription`。
- **className**：`class_name_for(role)` 映射到 `android.widget.Button`/`EditText`/`CheckBox`/`SeekBar`/`ListView`/…（驱动 TalkBack 语音模式）；容器回退 `android.view.View`。
- **状态**：`setEnabled(!DISABLED)` / `setSelected(SELECTED)` / `setCheckable`+`setChecked`（role 为 CheckBox/RadioButton/Switch 且 `CHECKED`）。
- **几何**：窗口本地逻辑 px → 屏幕 px，经 `view.getLocationOnScreen` + `DisplayMetrics.density`（`view_offset_and_density`）。
- **父子**：root 的 parent 指向 `HOST_VIEW_ID`，其余按 `parent` map（`AccessTree` 只存 children，parent 在 `update_tree` 时反推）；children 逐个 `addChild(View, vid)`。
- **动作**：`add_action` 按节点 `actions` 映射 `AccessAction → ACTION_*`（`Default`→`CLICK`+`setClickable`、`Focus`→`FOCUS`+`setFocusable`、`Increment`/`Decrement`/`ScrollIntoView`→scroll_fwd+bwd+`setScrollable`、`Expand`/`Collapse`/`SetValue`→`SET_TEXT`/`Select`/`ShowContextMenu`→`LONG_CLICK`）。

#### 接线（core ↔ Android 桥）

- `mod.rs`：`#[cfg(target_os = "android")] pub(crate) mod backend_android;` + `native_backend_for_window() -> Option<Rc<RefCell<dyn AccessibilityBackend>>>` 返回 `AndroidAccessibilityBackend::new()`（`new()` 内建单例 channel 并调 `inject_a11y_view()` 注入 Java View）。
- `window/handle.rs`：ohos 块之后加 `#[cfg(target_os = "android")] if let Some(backend) = crate::accessibility::native_backend_for_window() { window_state.set_accessibility_backend(Some(backend)); }`。
- `embedder/android.rs`：`android_capabilities()` 的 `accessibility: false → true`。

#### 关键坑（jni 0.21.1 + edition 2024 实战）

- **`JNIEnv` 带生命周期参数**：返回 `JObject<'a>` 必须与 `env: &mut JNIEnv<'a>` 的 `'a` **同源**（编译器会直接建议 `env: &mut JNIEnv<'a>`）。`'a` 是 env 的 local-frame 生命周期，与 `&mut` 引用自身寿命**分离**——正因如此，返回 `JObject<'a>` 后仍能继续 `env.call_method`（不会 E0499）。若误写成 `env: &'a mut JNIEnv`（把 `'a` 绑到可变引用寿命）则 E0499；若把返回 `'a` 绑到 `this` 参数（强制 `this: &'a JObject<'a>`）则 `JValue::Object(this)` 把 env 寿命与 `this` 寿命捆绑 → E0621。
- **`call_method` / `call_static_method` / `new_object` / `new_string` / `get_string` / `get_field` / `get_static_field` 收 `&mut self`**；`new_int_array` / `get_int_array_region` 收 `&self`。所有 helper 一律写 `env: &mut JNIEnv<'a>`。
- **edition 2024 → `#[unsafe(no_mangle)]`**（原 `#[no_mangle]` 报 "unsafe attribute used without unsafe"）；iOS 后端已用 `#[unsafe(method(...))]` 佐证同套路。
- **错误转安全值**：JNI 入口一律 `match JNIEnv::from_raw(env) { Ok(e) => e, Err(_) => return null/false }`，绝不 panic 跨 JNI 边界。
- **`GlobalRef` `Clone + Send + Sync`**，经 `as_obj()` 取 `&JObject<'static>`；`notify_changed` 在 `with_env` 闭包内 `env.call_method(v.as_obj(), "requestA11yRefresh", "()V", &[])`。
- **`HOST_VIEW_ID = -1`**：Android `AccessibilityNodeProvider` 约定 host view 的虚拟 id 为 `-1`；`create_node_info_locked` 据此分派 host vs 普通节点。
- **`get_string` 收 `&JString`**（不是 `JString` 值）：`let jtext = unsafe { JString::from_raw(text) }; env.get_string(&jtext)`。
- `JValue::from(bool)` / `JValue::Int(jint)` / `JValue::Object(&JObject)` 可用；`read_bundle_string` 经 `JString::from_raw(v.as_raw() as jstring)` 取回字符串。

### 4.6 Step 4 验证与文档

`cargo check -p floem --target aarch64-linux-android` **0 error**（修复 jni 0.21 生命周期三连：E0106 elided → `JNIEnv<'a>` + 返回 `JObject<'a>` 同源；E0621 → 不绑 `this`；E0499 → 不绑 `&mut` 引用寿命）；host `cargo check -p floem` **0 error**（门控不影响）。`backend_android.rs` 本文件 0 新 warning；`src/platform/clipboard.rs:266` 的 `E0133`（`AndroidJni::init` 未包 unsafe）为 Phase 1 既有 warning，非 Step 4 引入。未做真机 TalkBack 联调（交叉编译仅验证编译与桥接结构）。

### 4.7 Step 5 — HarmonyOS (OpenHarmony) 屏读桥 (2026-08-12)

HarmonyOS **无 accesskit 后端**（`accesskit` 仅覆盖 macos/linux/windows），且 OHOS 屏读经 **Accessibility Kit**（ArkTS API）暴露，须走 **N-API**（Rust ↔ ArkTS FFI）。故本步采用与桌面/iOS/Android **同一中性协议 + 同一 `PendingAction` 通道**，仅把"树推送"与"动作回调"换成语义等价的 N-API 实现：

- `src/accessibility/backend_ohos.rs`（`#[cfg(target_env = "ohos")]` 门控）—— 中性树 ↔ JS 对象序列化 + N-API 入口（`#[napi]`）。
- ArkTS 侧（Phase 3 应用壳）注册 `register_platform_bridge({ setAccessibilityTree, on_accessibility_action })`；Rust 经 `crate::platform::ohos_bridge::call_bridge` 回调 `setAccessibilityTree`，把整棵树交给 ArkTS 注册进 OHOS Accessibility Kit。

#### 为什么是 N-API 而非 accesskit

- OHOS 内核是 Linux（`target_os = "linux"`），但**屏读栈不是 AT-SPI2**，而是 ArkTS 的 Accessibility Kit（系统服务，非 D-Bus、非 accesskit 协议）；Rust 侧无法直接驱动，必须经 N-API 把树交给 ArkTS 层注册。这是与 Android（Java `View` + JNI）同源的思路：原生平台不提供 Rust 可直接实现的"提供方"接口，故 floem 自己 vend 一个桥（ArkTS 注册器）。
- `accesskit` 0.24 的 `accesskit_unix` adapter 在 OHOS 上不可用（glib/D-Bus 不存在），故 `backend_ohos` **不依赖 accesskit 传输层**，直接序列化中性树为 JS 对象（见下 `serialize_tree`）。

#### 单例 `ACTION_TX` + `OhosAccessibilityBackend`（单窗口）

OHOS 应用默认**单窗口**（与 Android 同源），故用模块级单例共享树 + action channel：

- `ACTION_TX: OnceLock<Mutex<mpsc::Sender<PendingAction>>>` —— 后端 `new()` 建 `mpsc` 通道、`Receiver` 留给自己、`Sender` 存进单例，供 `#[napi] on_accessibility_action`（无 `self` 指针）回传动作。
- `OhosAccessibilityBackend { tree: AccessTree, action_rx: mpsc::Receiver<PendingAction> }`；`poll_actions` 全 drain `action_rx`（与桌面/iOS/Android 一致，全取比只取一个更稳）。

#### 树推送（`update_tree` → `push` → `call_bridge`）

- `update_tree(update)`：`tree.apply(update)` 同步中性缓存 → 调用 `push()`。
- `push()`：`serialize_tree(&tree)` 生成 JS 对象（结构见下）→ 经 `call_bridge(|env, obj| { let f: JsFunction = obj.get_named_property("setAccessibilityTree")?; f.call(None, &[obj.into_unknown()]) })` 交给 ArkTS。仅当**拓扑**（root 或任一节点 children）变化才发（避免每帧刷屏读器元素身份抖动；属性变化由 ArkTS 按需重读）。
- `serialize_tree` / `serialize_node`：输出 `{ root: i64, focus: i64, nodes: [...] }`，每节点 `{ id, role, label, value, description, states(bits:u64→i64), actions:[...], bounds:{x0,y0,x1,y1}, children:[...] }`，与 ArkTS 注册器约定对齐。
- `role_name` / `action_name` / `action_from_name`：做 `AccessRole`/`AccessAction` ↔ 字符串互转（ArkTS 侧据此映射 OHOS `AccessibilityRole`/`Action` 枚举）。

#### 动作回传（`#[napi] on_accessibility_action`）

- `#[napi] pub fn on_accessibility_action(node_id: i64, action: String, payload: Option<String>)` —— ArkTS 把 OHOS Accessibility Kit 收到的动作反查回中性 `AccessAction` 字符串后调用，`tx.send((AccessNodeId(node_id as u64), action, payload))` 推入单例通道；`poll_actions` 取出后走与桌面/iOS/Android **完全相同的** `WindowState::handle_accessibility_action` → `ViewId::route_event` / `accessibility_action` 回调（见 4.3）。`node_id` 用 `i64` 收（napi 2.x 无 `u64: FromNapiValue`），函数内 `as u64` 重建。

#### 接线（core ↔ OHOS 桥）

- `mod.rs`：`#[cfg(target_env = "ohos")] pub(crate) mod backend_ohos;` + `native_backend_for_window() -> Option<Rc<RefCell<dyn AccessibilityBackend>>>` 返回 `OhosAccessibilityBackend::new()`（无 view 指针）。同时把 `#[cfg(target_os = "linux")]` 的 `backend_atspi` 模块声明与 `native_backend_for_window` Linux 变体改为 `all(target_os = "linux", not(target_env = "ohos"))`，**避免 OHOS 上误编译 AT-SPI/D-Bus 后端**（见下关键坑）。
- `window/handle.rs`：`WindowHandle::new` 内加 `#[cfg(target_env = "ohos")]` 块：若 `let Some(backend) = native_backend_for_window()` 则 `window_state.set_accessibility_backend(Some(backend))`（与 android/iOS 同形）。
- `embedder/ohos.rs`：`ohos_capabilities()` 的 `accessibility: false → true`（capability cfg 翻开）。
- `Cargo.toml`：`[target.'cfg(target_env = "ohos")'.dependencies]` 含 `tgui-winit-ohos = "0.4"` + `napi = "2"` + `napi-derive = "2"`（见下）。桌面依赖块改用 `cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))`，排除 OHOS。

#### 验证（隔离 CARGO_HOME + stub sysroot，整 crate 真实编译）

- **整 crate `cargo check --target aarch64-unknown-linux-ohos`：`Finished，0 error（7 benign warning）`**。验证手段：隔离 `CARGO_HOME=/tmp/x_cargo_home`（symlink registry/git 到 ~/.cargo）+ stub sysroot（`/Users/ahs/ohos_stub_sysroot`）+ `PKG_CONFIG_ALLOW_CROSS=1`，真实 `aarch64-unknown-linux-ohos` target。这是**真实 crate 级编译**（非 probe）：含 `backend_ohos.rs` 的 N-API 屏读桥、`tgui-winit-ohos` fork 的 `CoreWindow` impl 与全部 OHOS 平台桥（clipboard/external/file/receive/a11y），均真实编译通过。剩余 7 warning 全为桌面 `translate.rs` dead-code + `private_interfaces`（OHOS 不依赖 accesskit，属预期，非 OHOS 引入）。
- **关键 gate 是 `dpi` 双源漂移（非 Phase 3 sysroot）**：`floem-winit-core`(lapce/winit@`133268de`)在 winit 工作区用 `dpi = { git path }`(git 源)，`tgui-winit-ohos` fork 声明 `dpi = "0.1.2"`(crates.io 注册源) → Cargo 不跨源 unify → 两个不同 `dpi` 实例 → `CoreWindow` trait 期望的 `dpi::PhysicalPosition`(git) ≠ fork impl 提供的 PhysicalPosition(注册) → 30 个 E0053/E0308。修复：fork `Cargo.toml` 的 `dpi` 加 `git/rev/package="dpi"` 指向 lapce/winit@`133268de`，使两实例统一，整 crate check 越过 fork。实证：check 日志 `Checking tgui-winit-ohos v0.4.11` 通过。
- 修复链（floem 自身，共 11 处）：①`mod ohos_clipboard` 文件错位（移到 `clipboard/`）②`on_shared_files` 重名（内部改名 `deliver_shared_files`）③`pub use muda` 未排除 OHOS ④`JsObject/JsString` 导入路径错（改 `napi` 根）⑤`file_action` re-export 未排除 OHOS ⑥`wasm_stubs` 未对 OHOS 声明 ⑦`PlatformBridge` 非 Send/Sync（加 unsafe impl）⑧`export_ohos_winit_app!` 误用于 lib 内（删除）⑨`BackendCapabilities::winit_default` 缺 `accessibility` 字段（补 `accessibility:true`）⑩`ShowContextMenu` OHOS 无 match 分支（补 OHOS 分支复用 in-UI 渲染器）⑪`clipboard.rs` 对 `Result` 误用 `.unwrap_or`（直接返回）。
- **隔离 probe crate**（`/tmp/ohos_a11y_probe`）仍作为早期证据保留：镜像 `backend_ohos` 全部 napi 用法（`#[napi] on_accessibility_action(node_id: i64, ...)`、`create_int64(x as i64)`、`call_bridge` + `get_named_property("setAccessibilityTree")` + `func.call(None, &[obj.into_unknown()])`），针对真实 OHOS target 编译 0 error（两阶段复现：错 `target_os` gate → 改 `target_env` 复现 `create_int64(u64)` E0308 → 加 `napi-derive`+`i64` 边界 0 error）。
- **真 `cargo build` 仍受 Phase 3 链接缺口 gate**：`cargo check` 不链接，stub sysroot 已让 fontconfig-sys 越过；真链接阶段仍缺 OHOS fontconfig sysroot 与 wgpu vulkan OHOS 链接器（`VK_OHOS_surface`），均属 Phase 3 应用壳/打包职责，独立于 Step 5 编译验证。
- host `cargo check -p floem`（macos）0 error；`cargo test -p floem --lib accessibility` 仍 15 passed（a11y 单测不触 OHOS 后端）。

#### 关键坑（napi 2.16.17 + OHOS target 实战，高复用价值）

- **cfg 铁律：`target_env = "ohos"`，绝不用 `target_os = "ohos"`**。真实 target `aarch64-unknown-linux-ohos` 的 rustc cfg 是 `target_os = "linux"` + `target_env = "ohos"`；`#[cfg(target_os = "ohos")]` 在该 target 上**恒为 false**，会让整个 OHOS 后端被静默剔除（此前所有 OHOS 文件标 "UNVERIFIED" 的根因）。已对全部 14 处 `cfg(target_os = "ohos")` → `cfg(target_env = "ohos")` 修正；顺带把桌面 `accesskit_unix`/`muda(gtk)` 块从 `cfg(any(target_os = "linux"))` 改为 `all(target_os = "linux", not(target_env = "ohos"))`。**Cargo 支持 `target_env` 于 `[target.*.dependencies]`**（probe 实证：`napi` 被 `cfg(target_env = "ohos")` 正确选中并编译）。
- **`#[napi]` 宏不在 `napi` crate 内**，在独立 `napi-derive`。napi 2.x **不 re-export** 该属性宏 → 所有含 `#[napi]` 的 OHOS 模块（`backend_ohos.rs` / `ohos_bridge.rs` / `ohos_file.rs` / `ohos_receive.rs`）必须加 `napi-derive = "2"` 依赖并 `use napi_derive::napi;`，否则 `cannot find attribute napi in this scope`。
- **`u64` 无 `FromNapiValue`**（napi 2.x 仅为 `i64` 实现；bigint.rs 明确不实现 i64/u64/i128/u128/isize/usize）。故 `#[napi]` fn 的节点 id 参数必须写 `i64`，函数内 `AccessNodeId(node_id as u64)` 重建；返回值方向的 `ToNapiValue for u64` 已实现（可序列化/返回）。
- **`Env::create_int64` 收 `i64`**（非 `u64`）。`serialize_node` 中所有 id / states bits 必须 `as i64`：`id.0 as i64`、`node.id.0 as i64`、`node.states.bits() as i64`、`c.0 as i64`（5 处）。
- **`JsFunction::call` 签名是 `call<V>(this: Option<&JsObject>, args: &[V]) where V: NapiRaw`**，绝非 tuple。正确用法：`func.call(None, &[x.into_unknown()])`（`x` 经 `.into_unknown()` 成 `Unknown`）；`func.call((x,))` 是编译错误。
- **单窗口单例**：`ACTION_TX: OnceLock<Mutex<Sender>>` 与 `OhosAccessibilityBackend` 共享 channel；`#[napi]` 入口无 `self`，靠单例取 `Sender`。与 `backend_android` 的 `ANDROID_A11Y` 同源设计。

## 5. 验证（隔离 CARGO_HOME，trust-but-verify）

| 目标 | 命令 | 结果 |
|---|---|---|
| host (macos) | `cargo check -p floem` | Finished，0 error（accesskit macOS 后端 + translate + 接线已编译；`set_accessibility_backend` 已接线） |
| host 测试 | `cargo test -p floem --lib accessibility` | **15 passed / 0 failed**（Step 0 共 10 + Step 1 bridge 2 + Step 2 translate 3） |
| iOS-sim | `cargo check -p floem --target x86_64-apple-ios` | Finished，0 error（`backend_ios.rs` iOS VoiceOver 桥已编译：子类 `FloemA11yElement` + `IosAccessibilityBackend` + `native_backend_for_window`/`WindowHandle::new(ui_view)` 接线 + `ios_capabilities().accessibility=true`；本文件 0 新 warning） |
| Windows | `cargo check -p floem --target x86_64-pc-windows-gnu` | Finished，0 error（`backend_uia.rs` 子类化/`WM_GETOBJECT` 转发 + `handle.rs` `Win32` 变体取 `hwnd` 编译通过；`windows-sys` 仅 windows target） |
| Android | `cargo check -p floem --target aarch64-linux-android` | Finished，0 error（`backend_android.rs` TalkBack 桥已编译：单例 `ANDROID_A11Y` + 4 个 `#[unsafe(no_mangle)]` JNI 入口 + `FloemAccessibilityView.java` 注入 + `native_backend_for_window`/`WindowHandle::new` 接线 + `android_capabilities().accessibility=true`；修复 jni 0.21 生命周期三连 E0106/E0621/E0499；本文件 0 新 warning） |
| HarmonyOS (整 crate) | `cargo check -p floem --target aarch64-unknown-linux-ohos`（隔离 `CARGO_HOME` + stub sysroot + `PKG_CONFIG_ALLOW_CROSS=1`） | **Finished，0 error（7 benign warning 全为桌面 translate.rs dead-code + private_interfaces，OHOS 不依赖 accesskit）**。含 `backend_ohos.rs` N-API 屏读桥 + `tgui-winit-ohos` `CoreWindow` impl + 全部 OHOS 平台桥真实编译。真 gate 是 fork 的 `dpi` 双源漂移（已修：fork Cargo.toml dpi 加 git/rev/package 指向 lapce/winit@133268de）。真 `cargo build` 仍受 Phase 3 fontconfig/wgpu-vulkan 链接缺口 gate。 |

Step 2 的 `translate.rs` 与三个 `backend_*` 均 gated 桌面 target（translate 为 `any(macos,linux,windows)`，backend 各按本平台），故 iOS/Android/wasm 不编译这些；core 侧 `bridge.rs`/`window/handle.rs`/`window/state.rs` 变更由 iOS-sim `cargo check` 0 error 验证。`update_accessibility_tree()` 在 `process_update_no_paint` 末尾每帧调用，仅当 `update.is_empty()`（根视图被隐藏）时跳过。

OHOS 后端 `backend_ohos.rs` 以 `#[cfg(target_env = "ohos")]` 门控（**非** `target_os = "ohos"` —— OHOS 的 `target_os = "linux"`）；`backend_atspi` 与桌面 `accesskit_unix`/`muda(gtk)` 依赖块均已加 `not(target_env = "ohos")` 排除，避免 OHOS 上误编译 AT-SPI/glib 栈（否则整 crate OHOS check 在 glib-sys/pkg-config 死掉）。`backend_ohos` 不依赖 accesskit 传输层，直接把中性树序列化推到 ArkTS。`update_accessibility_tree()` 每帧接线对 OHOS 同样生效（`WindowHandle::new` 内 `#[cfg(target_env = "ohos")]` 块接 `set_accessibility_backend`）。

## 6. 待做（后续 Step）

- **Step 2 — 桌面后端**：✅ macOS / Linux / Windows 三桌面平台均已接**真实 accesskit 后端**（VoiceOver / ORCA / Narrator），经 accesskit 中性传输层统一桥接；`translate.rs` 单一映射层 + 3 单测；`capabilities.accessibility` 桌面 target 已 `true`；`set_accessibility_backend` 已接线。**真实 action 派发已落地**：`Focus`→`request_focus`、`Default`→定向 `Click`、`ShowContextMenu`→定向 `SecondaryClick`（经 `ViewId::route_event`）；**语义动作 hook**（`AccessibilityAttrs.on_action` + `ViewId::accessibility_action`，覆盖 Increment/Decrement/Expand/Collapse/SetValue/ScrollIntoView）；**macOS/Windows 焦点状态回传**（`update_focus_state` → `update_view_focus_state`/`update_window_focus_state`，在 `WindowHandle::focused` 接线）；**Windows `WM_GETOBJECT` 转发**（HWND 子类化，Narrator 可连）。三项缺口全部闭合。
- **[x] Step 3 — iOS 桥 (2026-08-12)**：`UIAccessibility` 桥（VoiceOver）。子类 `UIAccessibilityElement` 把中性树镜像到窗口 `UIView`；动作（activate/increment/decrement/scroll）经与桌面相同的 `PendingAction` 通道回传 core；`capability.accessibility` iOS 翻 `true`；`native_backend_for_window(ui_view)` + `WindowHandle::new(ui_view)` 接线。`cargo check --target x86_64-apple-ios` 0 error。Android 留 Step 4。
- **[x] Step 4 — Android 桥 (2026-08-12)**：`AccessibilityNodeProvider` / `AccessibilityNodeInfo`（TalkBack）。Rust 注入透明 Java `View`（`FloemAccessibilityView.java`，包 `dev.floem.android`）+ `backend_android.rs` 4 个 JNI 入口把中性树翻译成 `AccessibilityNodeInfo`、并把 TalkBack 动作经与桌面/iOS 相同的 `PendingAction` 通道回传 core；单例 `ANDROID_A11Y` 单窗口共享 tree + action channel；拓扑变化才 `TYPE_WINDOW_CONTENT_CHANGED`；`capability.accessibility` Android 翻 `true`。`cargo check --target aarch64-linux-android` 0 error。OHOS 屏读桥见 Step 5。
- **[x] Step 5 — HarmonyOS 桥 (2026-08-12)**：N-API 屏读桥（**无 accesskit 后端**，OHOS 屏读经 Accessibility Kit/ArkTS，Rust 经 N-API 把中性树序列化推到 ArkTS 注册）。新增 `src/accessibility/backend_ohos.rs`（`#[cfg(target_env = "ohos")]`）：`ACTION_TX: OnceLock<Mutex<mpsc::Sender<PendingAction>>>` 单窗口单例 + `OhosAccessibilityBackend { tree, action_rx }`；`update_tree` → `push()` → `call_bridge`(`setAccessibilityTree`) 序列化整树（root/focus/nodes[]，每节点 id/role/label/value/description/states(bits→i64)/actions[]/bounds{x0,y0,x1,y1}/children[]）；`#[napi] on_accessibility_action(node_id: i64, action: String, payload: Option<String>)` 经单例通道把 OHOS 动作回传 core（与桌面/iOS/Android **同一 `PendingAction` → `handle_accessibility_action` → `route_event`** 路径）。接线：`mod.rs::native_backend_for_window()` + `window/handle.rs::WindowHandle::new`(`#[cfg(target_env="ohos")]` 接 backend) + `embedder/ohos.rs::ohos_capabilities().accessibility=true`。**关键 cfg 修正**：OHOS 是 `target_os="linux"` + `target_env="ohos"`，故全部 14 处 `cfg(target_os="ohos")` → `cfg(target_env="ohos")`，并把 `backend_atspi`/`accesskit_unix`/`muda(gtk)` 桌面块加 `not(target_env="ohos")` 排除（否则 OHOS 误编译 AT-SPI/glib 栈、整 crate check 死在 glib-sys）；napi 2.x 陷阱：`#[napi]` 在 `napi-derive`（须 `use napi_derive::napi`）、`u64` 无 `FromNapiValue`（id 用 `i64`）、`create_int64` 收 `i64`（5 处 `as i64`）、`JsFunction::call(None, &[x.into_unknown()])`。**验证**：隔离 probe crate 针对真实 `aarch64-unknown-linux-ohos` target + napi 2.16.17 **0 error**；整 crate OHOS `cargo check` 受 Phase 3 依赖/sysroot 缺口 gate（fontconfig/wgpu-vulkan 的 OHOS sysroot/链接器缺口；`tgui-winit-ohos` retarget 已通过 `[patch.crates-io]` 应用，非 gate），独立于本步，但本步已修掉一个真实 OHOS 编译 blocker（glib-sys，已验证整 crate check 推进到 fontconfig）。未做真机/模拟器屏读联调（交叉编译仅验证编译与桥接结构）。
- 后续细化：overlay 并入 a11y 树；a11y 树 dirty 节流（避免每帧全量重建）；`DISABLED`/`SELECTED`/`EXPANDED` 等状态从真实视图状态提取（如 `disabled` style、`checked` 等）；`value` 自动从文本视图内容合成（少样板）。

## 7. Changelog

- **[x] Step 0 (2026-08-11)** — 中性无障碍协议 + 核心模型 + 后端 trait + `MockAccessibility` + 10 单元测；`lib.rs` 注册 `pub mod accessibility`；`BackendCapabilities` 加 `with_accessibility()` builder。host `cargo check` + `cargo test`(10 pass) + iOS-sim `cargo check` 均 0 error。
  - 设计要点：镜像 embedder 子系统的「数据协议 + 适配 trait」拆分；`AccessNode` 加 `PartialEq`（测试 `Option<&AccessNode>` 比较需要）；`node_at_point` 取最内层（面积最小）命中；`AccessTree::apply` 删除时清理悬挂 child 引用并清 root/focus。
  - 非阻塞缺口：`capability.accessibility` 仍 false（无真实屏读桥）；core 尚未接线视图属性（Step 1）。

- **[x] Step 1 (2026-08-11)** — 视图属性接线 + core 产出 `AccessTreeUpdate`。
  - 声明侧：`ViewState.accessibility: Option<AccessibilityAttrs>`（`src/view/state.rs`）；`ViewId` 新增 builder `accessibility_role`/`accessibility_label`/`accessibility_value`/`accessibility_description`/`accessibility_actions`/`accessibility_skip`（`src/view/id.rs`，镜像 `set_cursor`）。
  - 存储侧：`WindowState.accessibility: AccessTree` + `accessibility_backend: Option<Rc<RefCell<dyn AccessibilityBackend>>>` + `set_accessibility_backend`（`src/window/state.rs`）。
  - 产出侧：`src/accessibility/bridge.rs` 的 `build_access_tree_update(storage, root, focused)` 自 `ViewStorage::children` 遍历 → 读 `ViewState.accessibility` + `LayoutChanged::box_window()`(窗口坐标 `Rect`) + focus（`ElementId::owning_id()`）；`AccessNodeId = ViewId.data().as_ffi()`（稳定 u64）。`hidden` 视图从树删除、children 重挂最近非隐藏祖先（再父化已修：re-parented children 正确挂入父 `children`）。
  - 发射点：`WindowHandle::process_update_no_paint` 末尾调 `update_accessibility_tree()` → `window_state.accessibility.apply(update)` + 转发 `backend.update_tree(update)`（`src/window/handle.rs`）。
  - 验证：host `cargo check` 0 error；`cargo test -p floem --lib accessibility` **12 passed**（含 bridge 2 测试：树构建含角色/标签/几何 + 隐藏再父化 + focus）。无 `#[cfg]` 门控，交叉 target 由构造保证。
  - 非阻塞缺口：`capability.accessibility` 仍 false（无真实屏读桥）；每帧全量重建无 dirty 节流；overlay 未并入 a11y 树；`DISABLED`/`SELECTED`/`EXPANDED` 等状态与 `value` 自动合成留 Step 2+。

- **[x] Step 2 (2026-08-11)** — 桌面后端桥（accesskit 转向）：macOS/Linux/Windows 三桌面平台全部接**真实 accesskit 后端**（VoiceOver / ORCA / Narrator），绕开手搓原生 API 死路。
  - 架构转向（用户确认「accesskit 适配器」）：调研发现 `atspi`/`uiautomation` 是客户端库、无法"提供"无障碍；改用 `accesskit` 0.24 作中性传输层，三平台 adapter（`accesskit_macos` 0.26 / `accesskit_unix` 0.22 / `accesskit_windows` 0.34）均依赖 `accesskit ^0.24.1`；floem 用 lapce winit fork → 直接驱动 per-OS `Adapter`，不用 `accesskit_winit`。
  - 映射层 `src/accessibility/translate.rs`（新建，单测）：`role_to_accesskit` / `action_to_accesskit` / `build_accesskit_node` / `build_tree_update`；`Select`→`None`、`CHECKED`→`set_toggled(Toggled::True)`、焦点→`TreeUpdate.focus`；含 3 单测。
  - macOS `backend_nsaccessibility.rs`：`MacOSAccessibilityBackend` 包装 `accesskit_macos::Adapter`；`new(ns_view)` 裸 NSView 指针、`update_tree`→`update_if_active`→`QueuedEvents::raise()`；`MacOSActionHandler: ActionHandler` 反向映射经 `mpsc` 回传。
  - Linux `backend_atspi.rs`：`LinuxAccessibilityBackend` 包装 `accesskit_unix::Adapter`；`Adapter::new(activation, action, deactivation)` 自拥 session bus；`SharedTree = Arc<Mutex<AccessTree>>`、`ActivationHandler::request_initial_tree` 懒提供初始树。
  - Windows `backend_uia.rs`：`WindowsAccessibilityBackend` 包装 `accesskit_windows::Adapter`；`new(hwnd)` 经 `HWND(hwnd as *mut _)` 构造；**`WM_GETOBJECT` 转发已接**：对 HWND 子类化（`SetWindowLongPtrW` 存 `Arc<SubclassData>` 于 `GWLP_USERDATA`、换 `GWLP_WNDPROC` 为 `subclass_proc`），`subclass_proc` 转发 `WM_GETOBJECT` 并链回原过程、`WM_NCDESTROY` 还原释放；`Adapter`/`ActivationHandler` 经 `Arc<Mutex<>>` 共享、初始树走独立 `cached_tree` Mutex 避死锁。`windows-sys`（仅 windows target）提供裸 Win32 调用。
  - 接线：`native_backend_for_window`（macOS `ns_view` / Windows `hwnd` / Linux 无参）→ `set_accessibility_backend`；`update_accessibility_tree()` 每帧 `update_tree` 后 drain `poll_actions()` → `WindowState::handle_accessibility_action`（真实派发已落地）。`PendingAction` 类型别名落在 `backend.rs`。
  - Capability：桌面 target（macos/linux/windows）`accessibility = true`（cfg 翻开）；移动端仍 `false`。
  - 验证：host `cargo check -p floem` 0 error；`cargo test -p floem --lib accessibility` **15 passed**（10 + bridge 2 + translate 3）；iOS-sim `cargo check --target x86_64-apple-ios` 0 error。
  - **真实 action 派发（Step 2 收尾，2026-08-11）**：`WindowState::handle_accessibility_action(PendingAction)` 把屏读器动作真正路由到 core——`AccessNodeId(u64)` 经 `KeyData::from_ffi` 还原 `ViewId`；`Focus`→`request_focus()`，`Default`→`route_event(InteractionEvent::Click, Directed{TARGET})`，`ShowContextMenu`→`route_event(InteractionEvent::SecondaryClick, …)`；**语义动作 hook 已落地**：`AccessibilityAttrs.on_action`（`Arc<dyn Fn(AccessAction, Option<String>)>`）+ `ViewId::accessibility_action(handler)`，语义动作从视图注册的回调调用（`SetValue` 带 payload 字符串）；无回调则静默忽略。host/iOS-sim `cargo check` 0 error、`cargo test` 15 passed。
  - **焦点状态回传（Step 2 收尾，2026-08-11）**：`AccessibilityBackend` trait 新增默认 `update_focus_state(bool)`；macOS 后端→`adapter.update_view_focus_state(focused)`+`raise()`、Windows 后端→`adapter.update_window_focus_state(focused)`+`raise()`；`WindowHandle::focused(focused)`（`WindowEvent::Focused`）接线，VoiceOver / Narrator 与 floem 窗口焦点同步。
  - **Windows `WM_GETOBJECT` 转发（Step 2 收尾，2026-08-11）**：见上 `backend_uia.rs`；`cargo check --target x86_64-pc-windows-gnu` 0 error。
  - 关键坑（已修）：`ActionRequest` 用 `target_node` 非 `target`；焦点树级 `TreeUpdate.focus`；`accesskit_macos::Adapter::new` 收 `*mut c_void`（objc2 0.5/0.6 共存）；`HWND` 是 newtype 须 `HWND(... as *mut _)`；`accesskit_unix` 不传 zbus Connection；per-OS adapter 是独立 crate（Cargo.toml 勿重复 target key）。

- **[x] Step 3 (2026-08-12)** — iOS VoiceOver 桥（**无 accesskit 后端**，直接子类化 `UIAccessibilityElement`，新增 `src/accessibility/backend_ios.rs`，`#[cfg(target_os = "ios")]`）。
  - 动机：iOS 无 accesskit adapter（`accesskit` 仅覆盖 macos/linux/windows）；`UIAccessibility` 在 `objc2-ui-kit` 中以 category traits（`NSObjectUIAccessibility`/`NSObjectUIAccessibilityContainer`）暴露、无独立协议 struct，且只 impl 在 `UIAccessibilityElement` 精确类 → 子类属性 setter 一律 `msg_send!` 绕过 trait + mtm。
  - 子类 `FloemA11yElement`：`#[unsafe(super(UIAccessibilityElement))]` + `#[thread_kind = MainThreadOnly]` + ivars `A11yIvars { node_id, actions: u32(位掩码), tx: *const mpsc::Sender<PendingAction>(裸指针，Sender 非 Encode) }`；`define_class!` 固有 `impl` 块（写 `impl T {`，不带 `unsafe`）override `accessibilityActivate`→`Default`/`accessibilityIncrement`→`Increment`/`accessibilityDecrement`→`Decrement`/`accessibilityScroll:`→`ScrollIntoView`，按 `actions` 位掩码经裸指针 `send_action` 发 `PendingAction`。
  - `IosAccessibilityBackend`：持有 `Retained<UIView>`(根容器) + `HashMap<AccessNodeId, Retained<FloemA11yElement>>` 增量同步 + `AccessTree` 中性缓存 + `mpsc` 通道；`update_tree` 仅主线程触 OC，按 `old/new ids` 差集创建/删除 element、逐节点 `update_element` 刷新（逻辑像素经 `UIAccessibilityConvertFrameToScreenCoordinates` 转屏幕坐标），**仅拓扑变化才 `rebuild_containers` + `UIAccessibilityPostNotification(UIAccessibilityLayoutChangedNotification)`**；`poll_actions` 全 drain。
  - 接线：`mod.rs::native_backend_for_window(ui_view)` + `app/handle.rs`(`RawWindowHandle::UiKit` 取 `ui_view.as_ptr()`) + `window/handle.rs::WindowHandle::new(ui_view)` + `embedder/ios.rs::ios_capabilities().accessibility=true`；`#![allow(unsafe_op_in_unsafe_fn)]` 压制纯 FFI 模块 lint 噪声。
  - 验证：`cargo check -p floem --target x86_64-apple-ios` **0 error、本文件 0 新 warning**（修复 `unsafe impl` 路径/固有 impl 块/`MainThreadOnly` import/`Retained::as_ptr`/`extern static` mtm 等 6 类编译坑）；host `cargo check` 0 error（门控不影响）。
  - 缺口（与桌面一致）：`Expand`/`Collapse`/`SetValue`/`ShowContextMenu`/`Select`/`Focus` 在 iOS 无对应 UIAccessibility 动作，经 per-view `on_action` 回调；未做真实设备 VoiceOver 联调（模拟器 `cargo check` 仅验证编译与桥接结构）。

- **[x] Step 4 (2026-08-12)** — Android TalkBack 桥（**无 accesskit 后端**，`AccessibilityNodeProvider` 必须是 Java `View` 层级对象 → Rust 注入透明 Java `View` + JNI 入口，新增 `src/accessibility/backend_android.rs`（`#[cfg(target_os = "android")]`）与 `src/platform/android/FloemAccessibilityView.java`（包 `dev.floem.android`，load-bearing 须与 JNI 符号一致））。
  - 动机：Android 无 accesskit adapter、且 provider 必须是 Java 对象（JVM 反射）→ 双文件方案。Java `View` 透明（`setWillNotDraw(true)`）、不消费触摸（`setClickable`/`setFocusable` false）、`IMPORTANT_FOR_ACCESSIBILITY_NO`；`getAccessibilityNodeProvider()` 返回 `FloemProvider extends AccessibilityNodeProvider`，回调 forward 到 4 个 native 入口。
  - 单例 `ANDROID_A11Y: static Mutex<Option<AndroidA11yInner>>`（Android 单窗口）：同时被 `AccessibilityBackend` impl 与 JNI 入口（无 `self`）共享 tree + `mpsc` action channel；`ActionConsts` 惰性缓存 `AccessibilityNodeInfo.ACTION_*` int 常量。`AndroidAccessibilityBackend` 零尺寸标记委托单例。
  - JNI 入口（edition 2024 → `#[unsafe(no_mangle)]`）：`nativeRegisterView`（`new_global_ref` 存 `GlobalRef`，`Clone+Send+Sync`）/`nativeCreateNodeInfo`（host `HOST_VIEW_ID=-1` 走 `build_host_info`，否则 `create_node_info_locked` 重建 node 后 `into_raw()`）/`nativePerformAction`（Android action int 反查 `AccessAction` 经 `tx.send`；`ACTION_SET_TEXT` payload 从 Bundle `android.view.accessibility.extra.DATA_TEXT` 读）/`nativeFindNodesByText`（按 label/value/description 大小写不敏感匹配，返回 `ArrayList`）。各入口 `JNIEnv::from_raw` + 错误转 null/false，绝不跨 JNI 边界 panic。
  - 翻译 `create_node_info_locked`：`obtain(View, vid)` → text(label>value)/contentDescription/className(`class_name_for(role)`)/states(setEnabled/setSelected/setCheckable+setChecked)/几何(逻辑px 经 `getLocationOnScreen`+`density` 转屏幕px)/parent(`HOST_VIEW_ID` 或 parent map)/children/addChild/actions(add_action：`Default`→CLICK+setClickable、`Focus`→FOCUS+setFocusable、ScrollIntoView→scroll_fwd+bwd+setScrollable、SetValue→SET_TEXT、ShowContextMenu→LONG_CLICK 等)。每帧全量 `tree.apply`，**仅拓扑变化**才 `notify_changed()`（`view.requestA11yRefresh()` → `post(TYPE_WINDOW_CONTENT_CHANGED)`）；per-node info 按需重建，属性变化无需显式通知。
  - 接线：`mod.rs::native_backend_for_window()`（建单例 channel + `inject_a11y_view()` 注入 Java View）+ `window/handle.rs::WindowHandle::new`（`#[cfg(target_os="android")]` 接 backend）+ `embedder/android.rs::android_capabilities().accessibility=true`；`#![cfg(target_os="android")]` 门控，iOS/桌面/HarmonyOS 不编译。
  - 验证：`cargo check -p floem --target aarch64-linux-android` **0 error**（修复 jni 0.21 生命周期三连：E0106 elided → `JNIEnv<'a>` + 返回 `JObject<'a>` 同源；E0621 → 不绑 `this`；E0499 → 不绑 `&mut` 引用寿命；`ActionConsts::load` 可变闭包 E0596）；host `cargo check -p floem` 0 error。`backend_android.rs` 本文件 0 新 warning；`src/platform/clipboard.rs:266` 的 `E0133`（`AndroidJni::init` 未包 unsafe）为 Phase 1 既有 warning，非本步引入。
  - 关键坑（jni 0.21.1 + edition 2024）：`JNIEnv<'a>` 与返回 `JObject<'a>` 必须同源（编译器直接建议 `env: &mut JNIEnv<'a>`）；`call_method`/`get_field`/`get_static_field`/`new_string`/`get_string` 收 `&mut self`、`new_int_array`/`get_int_array_region` 收 `&self`；`#[unsafe(no_mangle)]`（edition 2024）；`GlobalRef` 经 `as_obj()` 取 `&JObject`；`get_string` 收 `&JString`；`HOST_VIEW_ID=-1` 为 Android provider 约定。
  - 缺口（与桌面/iOS 一致）：未做真机 TalkBack 联调（交叉编译仅验证编译与桥接结构）；`Expand`/`Collapse`/`SetValue`/`ShowContextMenu`/`Select`/`Focus` 经 per-view `on_action` 回调（TalkBack 无对应原生动作）。

- **[x] Step 5 (2026-08-12)** — HarmonyOS (OpenHarmony) N-API 屏读桥（**无 accesskit 后端**，OHOS 屏读经 Accessibility Kit/ArkTS，Rust 经 N-API 把中性树序列化推到 ArkTS 注册；新增 `src/accessibility/backend_ohos.rs`，`#[cfg(target_env = "ohos")]`）。
  - 动机：OHOS 内核 `target_os = "linux"` 但屏读栈是 ArkTS Accessibility Kit（非 AT-SPI2、非 accesskit 协议）；`accesskit_unix` adapter 在 OHOS 不可用（glib/D-Bus 不存在）→ 不依赖 accesskit 传输层，直接把中性树序列化推到 ArkTS。与 Android（Java View + JNI）同源思路：原生平台不提供 Rust 可实现的"提供方"接口，故 floem vend 一个 N-API 桥。
  - 单例 `ACTION_TX: OnceLock<Mutex<mpsc::Sender<PendingAction>>>`（OHOS 单窗口，与 Android `ANDROID_A11Y` 同源）+ `OhosAccessibilityBackend { tree: AccessTree, action_rx: mpsc::Receiver<PendingAction> }`；`poll_actions` 全 drain。
  - 树推送：`update_tree(update)` → `tree.apply` → `push()` → `serialize_tree`（root/focus/nodes[]，每节点 id/role/label/value/description/states(bits→i64)/actions[]/bounds{x0,y0,x1,y1}/children[]）→ `call_bridge(|env, obj| { let f: JsFunction = obj.get_named_property("setAccessibilityTree")?; f.call(None, &[obj.into_unknown()]) })` 交 ArkTS 注册进 OHOS Accessibility Kit。**仅拓扑变化才发**（属性变化由 ArkTS 按需重读）。
  - 动作回传：`#[napi] on_accessibility_action(node_id: i64, action: String, payload: Option<String>)`（ArkTS 把 OHOS 动作反查回中性 `AccessAction` 字符串后回调）→ `tx.send((AccessNodeId(node_id as u64), action, payload))` 推单例通道 → `poll_actions` → 与桌面/iOS/Android **同一** `WindowState::handle_accessibility_action` → `ViewId::route_event` / `accessibility_action` 回调。`role_name`/`action_name`/`action_from_name` 做 `AccessRole`/`AccessAction` ↔ 字符串互转。
  - 接线：`mod.rs::native_backend_for_window()`（无 view 指针）+ `window/handle.rs::WindowHandle::new`（`#[cfg(target_env = "ohos")]` 块接 backend）+ `embedder/ohos.rs::ohos_capabilities().accessibility=false→true`；`backend_atspi` 模块声明与 `native_backend_for_window` Linux 变体由 `cfg(target_os = "linux")` 改为 `all(target_os = "linux", not(target_env = "ohos"))`，避免 OHOS 误编译 AT-SPI 后端。`Cargo.toml` `[target.'cfg(target_env = "ohos")'.dependencies]` 含 `tgui-winit-ohos = "0.4"` + `napi = "2"` + `napi-derive = "2"`；桌面块改 `cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))`。
  - **cfg 铁律修正（本步最重要产出之一）**：OHOS 真实 target `aarch64-unknown-linux-ohos` 的 rustc cfg 是 `target_os = "linux"` + `target_env = "ohos"`；`#[cfg(target_os = "ohos")]` 恒 false → 此前整个 OHOS 后端被静默剔除（所有 OHOS 文件标 "UNVERIFIED" 根因）。全部 14 处 `cfg(target_os = "ohos")` → `cfg(target_env = "ohos")` 修正；并把桌面 `accesskit_unix`/`muda(gtk)` 依赖块 `cfg(any(target_os = "linux"))` → `cfg(all(target_os = "linux", not(target_env = "ohos")))`（否则 OHOS 误拉 glib 栈、整 crate check 死在 glib-sys）。**Cargo 支持 `target_env` 于 `[target.*.dependencies]`**（probe 实证 `napi` 被该 gate 选中并编译）。
  - **napi 2.16.17 陷阱（高复用价值）**：`#[napi]` 宏在 `napi-derive`（napi 2.x 不 re-export → 须 `napi-derive = "2"` + `use napi_derive::napi;`）；`u64` 无 `FromNapiValue`（仅 `i64`，故 id 参数写 `i64`、内 `as u64`）；`Env::create_int64` 收 `i64`（5 处 `as i64`）；`JsFunction::call(this: Option<&JsObject>, args: &[V])` → `func.call(None, &[x.into_unknown()])`，非 tuple。
  - 验证：**整 crate `cargo check -p floem --target aarch64-unknown-linux-ohos` 已 `Finished 0 error`（7 benign warning，全为桌面 translate.rs dead-code + private_interfaces，OHOS 不依赖 accesskit）**——这是真实 crate 级编译（含 `backend_ohos.rs` N-API 屏读桥、`tgui-winit-ohos` `CoreWindow` impl、全部 OHOS 平台桥），非 probe 级。验证手段：隔离 `CARGO_HOME=/tmp/x_cargo_home` + stub sysroot(`/Users/ahs/ohos_stub_sysroot`) + `PKG_CONFIG_ALLOW_CROSS=1`。**真 gate 是 fork 的 `dpi` 双源漂移**（已修：`tgui-winit-ohos-fork` Cargo.toml 的 `dpi` 加 `git/rev/package="dpi"` 指向 lapce/winit@`133268de`，与 `floem-winit-core` 统一，消除 30 E0053/E0308）。早期**隔离 probe crate**（`/tmp/ohos_a11y_probe`）仍作 napi 表面证据保留（两阶段复现：错 `target_os` gate → `target_env` 复现 `create_int64(u64)` E0308 → 加 `napi-derive`+`i64` 边界 0 error）。**真 `cargo build` 仍受 Phase 3 链接缺口 gate**（fontconfig sysroot + wgpu vulkan `VK_OHOS_surface` 链接器），`cargo check` 不链接故已越过。host `cargo check -p floem` 0 error、host `cargo test --lib accessibility` 仍 15 passed。
  - 缺口（与桌面/iOS/Android 一致）：未做真机/模拟器屏读联调（隔离交叉编译仅验证编译与桥接结构）；`Expand`/`Collapse`/`SetValue`/`ShowContextMenu`/`Select`/`Focus` 经 per-view `on_action` 回调（OHOS Accessibility Kit 无对应原生动作时走此路径）；ArkTS 注册器（Phase 3 应用壳侧 `register_platform_bridge` + `setAccessibilityTree` 实现）待 Phase 3 落地。
