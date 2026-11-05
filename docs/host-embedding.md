<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# Host Embedding — 把 Floom 作为 Native 组件的一部分（add-to-app）

> 状态：设计提案，**尚未实现**。本文回答的问题是「Floom 能不能像 Flutter
> 的 `FlutterView` / `FlutterFragment` / `FlutterViewController` 那样，作为 native
> 布局里的一个子视图存在，而不是接管整个窗口」。

## 1. 先分清两个方向

| 方向 | 含义 | Flutter 对应 | Floom 现状 |
| --- | --- | --- | --- |
| **A. Native in Floom**<br>（平台视图） | Floom 是全屏，把原生控件嵌进来 | `AndroidView` / `UiKitView` | 规划里有：Phase 16（Native ABI）+ Phase 17（WebView/Video/Camera/Map）。`crates/native/floom-native` 已作为 Phase 12.2 前置落地了 `NativeViewPool` |
| **B. Floom in Native**<br>（add-to-app） | Native 是全屏，Floom 只占其中一块 | `FlutterView` / `FlutterFragment` / `FlutterViewController` | **规划里完全没有**。本文补的就是这个 |

本文只讨论 **B**。

## 2. 今天能不能做到？——不能

当前每一条可用路径都是「Floom 拥有整个窗口/Activity」：

- 桌面：`floom::launch(...)` → winit `EventLoop` → **自己创建** `Window` →
  `GpuResources::request(window_handle)`。
- Android：`floom::Application::new_android(app)`，`app: AndroidApp` 来自
  `native-activity` glue。清单里是 `android.app.NativeActivity` +
  `android:hasCode="false"`，**Floom 就是那个 Activity**，占满全屏。
- iOS / Harmony：同理，Floom 拥有 `UIApplication` / Ability 的主窗口。

也就是说：现在想在一个 native 页面里放一块 Floom，唯一能做的是**起一个独立的
全屏 Activity/ViewController**，而不是一个子视图。这跟 Flutter 的 add-to-app 不是
一回事。

补充事实（避免误判进度）：Floom 2.0 的 `crates/platform/floom-platform-android` /
`-ios` 目前是 **headless 桩**（`lib.rs` 注释：*"headless until NDK surface glue
exists"*），即 2.0 在移动端还没有能跑起来的后端；能上真机的仍是 `src/` 下
1.x + lapce winit fork 那条路径。

## 3. 好消息：地基已经铺好 70%

`floom::embedder`（见 [`embedder.md`](./embedder.md)）定义的正是为这件事准备的
中立协议：

```
BackendCapabilities      后端声明能力，不支持的显式降级
SurfaceDescriptor        { kind, logical_size, pixel_size, scale, transparent }
SurfaceEvent             Created / Resized / Lost / Restored / Occluded / Destroyed
EmbedderEvent            Surface / Pointer / Keyboard / Ime / Focus / RedrawRequested
EmbedderCommand          RequestRedraw / SetCursor / SetImeState / ...
EmbedderRuntime / EmbedderWindow / SurfaceHandle
```

桌面路径（`floom::launch`）和 headless 测试后端（`HeadlessEmbedder`）都已经是
「生产 `EmbedderEvent`、消费 `EmbedderCommand`」的实现。**再写一个后端，让它把
事件从「宿主给的 surface」而不是「自己开的 window」里取出来，就是 add-to-app。**

## 4. 还差什么（按阻塞程度排序）

1. **原始 surface 句柄。** `SurfaceDescriptor` 只描述尺寸/缩放/种类，**没有承载
   句柄的字段**。需要引入类似 `raw_window_handle` 的
   `RawSurfaceHandle { AndroidNdk(ANativeWindow*), MetalLayer(CAMetalLayer*),
   Hwnd, NsView, Xlib/Wayland, HtmlCanvas }`，并区分「Floom 拥有 surface」和
   「宿主拥有 surface」（后者禁止 Floom 自行 resize/destroy）。
2. **反转事件循环所有权。** 现在 Floom 阻塞在 `EventLoop::run()`；add-to-app 时
   宿主才拥有主循环。需要一个外部驱动的引擎 API：
   ```rust
   let engine = FloomEngine::attach(EmbeddingSurface { handle, size, scale })?;
   engine.send(EmbedderEvent::Pointer(..));   // 宿主转发触摸
   engine.send(EmbedderEvent::Surface(SurfaceEvent::Resized(..)));
   engine.render_frame();                     // 宿主说"该画一帧了"
   engine.detach();
   ```
   这要求 `floom-scheduler` / `floom-runtime` 支持 **非阻塞、外部泵驱动** 的帧
   循环（桌面路径继续自己泵）。
3. **尺寸与 DPR 由宿主下发。** 子视图没有资格决定自己的大小：`layoutSubviews`
   / `surfaceChanged` / `onMeasure` 的结果是唯一真值，`SurfaceEvent::Resized`
   必须成为权威输入。
4. **线程亲和性。** Android 必须跑在 UI 线程、iOS 必须在 main thread，Floom 不能
   自己起渲染线程接管呈现。
5. **输入 / IME / 焦点转发。** Android `MotionEvent` → `EmbedderEvent::Pointer`，
   `InputConnection` → `EmbedderEvent::Ime`；iOS `touches*` → Pointer，
   `UITextInput` → Ime。焦点进出子视图要发 `EmbedderEvent::Focus`。
   这一层好消息是：**`crates/floom-ime`（Phase 13.3）已经是平台中立状态机，
   协议层不用重做**，只差宿主侧的桥。
6. **无障碍合并进宿主树。** Floom 的语义树不能自成孤岛：Android 上要实现
   `AccessibilityNodeProvider` 挂到 `FloomView`、iOS 上要实现
   `UIAccessibilityContainer`。`crates/core/floom-accessibility` 已经提供
   `SemanticsTree` + `SemanticsDiff`，桥接是纯增量。
7. **与方向 A 的互操作。** 一个 `FloomView` 内部再嵌原生子控件时，需要
   `NativeCompositionMode::{Overlay, Texture, Surface, Offscreen}` 的能力协商
   （Phase 16.3 的产物）。

## 5. 建议增加的 Phase（规划外）

规划 Phase 14–22 里没有这一项，建议追加：

- **23.1 `crates/floom-embed`**：`RawSurfaceHandle` / `EmbeddingSurface` /
  `FloomEngine`（attach / send / render_frame / detach），外部泵驱动的调度器接入；
  契约测试用 headless surface。
- **23.2 Android `FloomView`**：`SurfaceView`（性能好）+ `TextureView`（z-order
  与动画正确）两套，JNI glue，示例工程 = 一个普通 `Activity` 里
  `RecyclerView` + `FloomView` 混排。
- **23.3 iOS `FloomUIView`**：`UIView` + `CAMetalLayer`，ObjC 绑定。
- **23.4 桌面子视图托管**：macOS `NSView` / Windows `HWND` / Linux X11+Wayland。
- **23.5 Web canvas 挂载**。
- **23.6 a11y + IME 转发契约**与跨平台一致性测试。

## 6. 取舍

值得做，因为它是「Floom 只能整包替换你的应用」与「Floom 可以渐进式被采用」之间
的分水岭——存量 App 只可能接受后者。但要清醒：23.2/23.3 是真机工作，且依赖
23.1 的「外部驱动帧循环」这一项**核心运行时改动**，不是纯胶水。

一个务实的中间态：先只做 **23.1 + 桌面（23.4）**——桌面宿主最容易验证，能把
「外部泵 + 宿主尺寸 + 输入转发」这套语义钉死；移动端再照抄。
