# 真实后端缺口与移植估算（Real Backend Gap & Porting Estimate）

> 范围：补齐 `floom-2` 在 **Android / iOS / HarmonyOS（OHOS）** 上的"真实窗口后端"，
> 使其能像桌面端一样打开窗口、跑事件循环、用 GPU 渲染、并把输入/系统服务接入 floom。
>
> 日期：2026-09-19 · 作者：floom-2 移植工作流 · 置信度：竞品事实为高，工作量为人周级工程判断（标注 confidence）。
>
> 参照项目：**Slint**（winit + android-activity + Skia）、**Flutter**（Engine/Embedder 分层 + 稳定 C ABI + Impeller）、**GPUI Mobile**（完整 `Platform` trait + wgpu + NDK/UIKit）。

---

## 0. 摘要（TL;DR）

- **现状**：`floom-platform-{ios,harmony}` 目前只是 ~100 行的 **headless 骨架**，把 `floom-platform::fake::HostPlatform`（内存态实现）重新导出，**没有任何窗口/渲染/事件循环**。**Android 已不再是骨架**：`floom-platform-android` 有 winit 驱动的真实 `Platform`/`Window`/`Ime`/`Clipboard`（`#[cfg(target_os="android")]`）加一个可运行入口，首帧、tap、剪贴板、软键盘已在模拟器上逐项取证（§7.2、§7.2.2）。桌面端（`floom-platform-desktop`，3332 行 winit 胶水）是另一条真实后端，已在本机 Metal 冒烟验证。
- **好消息**：floom 已经具备"真后端"所需的**全部地基**——（1）`floom-platform` 的 `Platform`/`Window`/`Surface`/`Ime`/`Clipboard` 契约（= Flutter 的 Embedder ABI）；（2）**vello/wgpu** 渲染（与 GPUI Mobile 同源）；（3）桌面端用 **raw-window-handle** 做 OS 无关的 surface 交接；（4）我们已把触摸状态机、NDK 键码表、软键盘高度滤波、safe-area/system-chrome 类型这些**纯逻辑层**从 GPUI Mobile 搬进了 `floom-gesture` / `floom-platform-mobile`。
- **真正的缺口是"OS 胶水层"**，不是 UI 逻辑：三个平台各自需要一个**真实 `Platform`/`Window`/`Ime`/`Clipboard` 实现** + 生命周期/事件循环接入 + JNI / objc2 / NAPI 桥 + 从原生窗口句柄创建 wgpu 设备。**Android 这一层已交付**（§7.2.2，含 JNI 服务桥）；iOS / Harmony 仍待做。
- **可行性排序**：Android（最高，wgpu-Vulkan 成熟 + cargo-ndk）＞ iOS（wgpu-Metal + UIKit，Apple 工具链摩擦）＞ **Harmony（路径已清晰且"硬伤"被 `gpui_ohos` 实测证伪：复用 `openharmony-ability` NAPI 桥 + wgpu `Backends::GL`(GLES/EGL) 经 `OHNativeWindow`，`raw_window_handle::Ohos` 已被 wgpu 原生支持，`SurfaceTarget` 零改造；参照 Flutter/RN/gpui-ohos 成熟实现）**。
- **工作量包络**（单工程师，到"空窗口渲染 + 基础输入"里程碑）：Android 4–6 周、iOS 4–6 周、Harmony 6–10 周；三平台完整功能对等（IME/剪贴板/safe-area/文本/深色模式）+ 各 2–3 周；CI/AVD/模拟器/真机验证另计 3–5 周。总计约 **21–38 人周**（≈ 5–9 月·人）。

---

## 1. 背景：floom-2 移动/鸿蒙现状

### 1.1 平台层全景

| 后端 crate | 源码量 | 状态 | 渲染 |
|---|---|---|---|
| `floom-platform-desktop` | `winit_backend.rs` + `winit_input.rs` + `native_appkit.rs` ≈ **3332 行** | ✅ **唯一真实后端**（winit + AppKit），本机 Metal 冒烟已验证 | vello/wgpu（via `floom-renderer-vello`） |
| `floom-platform-android` | `android.rs` 828 + `lib.rs`/`accessibility.rs` ≈ 950 行 | ✅ **真实后端**（lapce/winit fork + `AndroidDriver`/`run`/`set_android_app`）；**已在 API 34 arm64 模拟器上验证首帧 + tap**（见 §7.2） | vello/wgpu（Vulkan；模拟器走 SwiftShader） |
| `floom-platform-ios` | `lib.rs` + `accessibility.rs` ≈ 98 行 | ❌ headless 骨架，仅 `HostPlatform::new()` | 无 |
| `floom-platform-harmony` | `lib.rs` + `accessibility.rs` ≈ 94 行 | ❌ headless 骨架，仅 `HostPlatform::new()` | 无 |

三个移动/鸿蒙 crate 只是把 `floom-platform` 的**内存态 `HostPlatform`**（`mod fake` 的 in-memory 实现）重新导出——没有 winit/NDK/UIKit/ArkUI 表面胶水，没有渲染接入。

### 1.2 已经完成的"纯逻辑"层（已从 GPUI Mobile 移植，可复用）

这些 GPUI Mobile 当年从零写的部分，floom 已经就绪，是**真后端落地时直接接线即可**的组件：

- `floom-gesture/src/touch.rs` — 触摸手势分类状态机（`Idle`/`Pending`/`Scrolling`，`SCROLL_SLOP=8.0`，tap↔scroll 区分 + `VelocityTracker`→fling），9 个测试。
- `floom-platform-mobile/src/packages/keyboard/android_table.rs` — NDK `AKEYCODE_*`/`AMETA_*` → floom `Keystroke`/`Modifiers` 映射表，23 个测试。
- `floom-platform-mobile/src/packages/keyboard/soft_input/` — `SoftKeyboardState` + `KeyboardHeightFilter`（迟滞滤波）。
- `floom-platform-mobile/src/packages/{safe_area,system_chrome}` — `safe_area_insets()` / `SystemChromeStyle` 类型与 host no-op。**Android 臂仍未实现**（返回 `Insets::ZERO`，与 §7.2.2 末尾的 `InsetsChanged` 通知缺口同源）。
- `floom-platform-mobile/src/packages/clipboard` — 不再是 stub：Android 臂是**真 JNI**（`ClipboardManager`），已在设备上取到逐字节一致的回值。
- 8 个设备能力包（`device_info`/`battery`/`connectivity`/`location`/`sensors`/`local_auth`/`notifications`/`share`）——全部是**带类型的 inert stub**，等真后端落地填原生实现（`vibration` 有一个 worked example）。

### 1.3 缺口的本质

SDD（`.superpowers/sdd/progress-floom-2.md` L153-154）已官方确认：

> "floom-2 cannot run on mobile at all yet" —— mobile half 在 android/ios 仍是 headless capability-limited stub 时无法满足；OS-surface bridge 与 Phase 16 / 19.4 / 19.5 同一工作。

> **该判断对 Android 已过期（2026-09-19）**：Android 已能真实启动、渲染首帧并响应 tap（§7.2）。iOS/Harmony 仍成立。

因此这是 **Phase 17（OS provider）/ 19（跨平台硬件）的"实现缺口"，不是"测试缺口"**。旧笔记把 17/19 记为"设备门控做不了（测试）"不准确——更准确的说法：**后端尚未实现，所以既无物可测、也无从验证**。

---

## 2. floom 的"真实后端能力面"（capability surface）

### 2.1 `floom-platform` 契约（= 内嵌的 Embedder ABI）

`crates/floom-platform/src/lib.rs` 定义了后端的全部义务：

```rust
pub trait Platform {                       // 顶层平台后端
    fn id(&self) -> PlatformId;
    fn capabilities(&self) -> PlatformCapabilities;
    fn create_window(&mut self, config: WindowConfig) -> WindowId;
    fn window(&self, id: WindowId) -> Option<&dyn Window>;
    fn window_mut(&mut self, id: WindowId) -> Option<&mut dyn Window>;
    fn poll_events(&mut self, out: &mut Vec<PlatformEvent>);  // 事件循环出口
    fn request_frame(&mut self, window: WindowId);
    fn now_secs(&self) -> f64;
    fn ime(&mut self, _window: WindowId) -> Option<&mut dyn Ime> { None }
}
pub trait Window { /* create_surface / surface_target / surface_config / set_state … */ }
pub trait Clipboard { fn get_text / set_text }
pub trait Ime { fn set_enabled / set_cursor_area }
```

`surface.rs` 明确：**surface 交接用 `raw-window-handle` 类型**，不依赖 winit/wgpu/vello；后端只需填 `Surface::target`（`SurfaceTarget`）。`capabilities.rs` 的 `Capability` 枚举控制剪贴板/IME 等能力的开闭。

### 2.2 渲染接入现状：vello/wgpu + raw-window-handle

- `crates/floom-renderer-vello/Cargo.toml`：`vello`（GPU 光栅化，**re-export 自己的 wgpu**）→ 真实 GPU 呈现走 **wgpu swap-chain**。这与 GPUI Mobile 用 wgpu 完全一致（iOS=Metal，Android=Vulkan/GL）。
- 桌面 `winit_backend.rs` 的 `WinitSurface::target()` 把 winit 窗口句柄包成 `SurfaceTarget::new(...)`（`raw_window_handle`），renderer 据此建 wgpu surface。关键点：**surface 交接已 OS 无关**，后端只需在目标 OS 给出正确的 `RawWindowHandle` 变体（Android=`AndroidNdk`、iOS=`UiKit`）。**注意：OHOS 没有 winit 后端**——鸿蒙的窗口句柄来自 ArkUI `XComponent` 经 `NAPI` 下发的 `OHNativeWindow*`（Flutter/RN 均走此路径），再经 `VK_OHOS_surface` 扩展转成 Vulkan `VkSurfaceKHR` 交给 wgpu（需一段薄 shim，非完整 GPU 后端）。
- 桌面后端目前在 `target()` 里只匹配 `RawWindowHandle::AppKit`（macOS）。移植到移动端必须**枚举 `AndroidNdk` / `UiKit` 变体**（以及 OH 的句柄）。

### 2.3 "真后端"的定义

在目标 OS 上实现 `Platform` + `Window` + `Clipboard` + `Ime`，使其：
1. 创建原生窗口并产出 `RawWindowHandle`；
2. 跑原生事件循环，把 OS 输入/生命周期翻译成 `PlatformEvent` 喂给 `poll_events`；
3. 从原生句柄建 wgpu 设备/队列/swapchain，驱动 vello 每帧光栅化；
4. 通过 JNI / objc2 / NDK 把 IME、剪贴板、safe-area、system-chrome、设备能力包的原生实现接上。

---

## 3. 竞品如何补齐真实后端（参照）

### 3.1 Slint — winit + android-activity + JNI/objc + Skia

- **统一抽象**：以 **winit** 作为跨平台窗口/事件核心（`i-slint-backend-winit`，覆盖 Win/macOS/Linux/WASM/**iOS**/**Android**）。移动/鸿蒙复用桌面同一套事件映射（`LogicalPoint`、`KeyEvent` 转换、鼠标合并）。
- **Android 专用后端** `i-slint-backend-android-activity`：用 **`android-activity` crate** 管生命周期；`AndroidWindowAdapter` 桥接 `AndroidApp`；`JavaHelper`（JNI）调自带的 `SlintAndroidJavaHelper.java` 取 **safe-area insets / 主题色 / 剪贴板**；IME 通过与隐藏 Java View 同步（focus 时 `input_method_request`）；`build.rs` 用 `javac`+`d8` 把 Java 编成 `.dex` 嵌进二进制。渲染：Skia（Vulkan/OpenGL ES）。
- **iOS**：winit Apple 后端 + Skia（Metal），含 `KeyboardCurveSampler`、配色方案观察者。
- **OHOS**：平台抽象可插拔，社区/生态侧已有 OpenHarmony 支持。
- **可借鉴**：winit 作为统一事件层 + 一个小的 JNI/objc 桥处理系统服务（IME/剪贴板/safe-area），渲染选平台支持的 GPU API。

### 3.2 Flutter — Engine/Embedder 分层 + 稳定 C ABI + Impeller

- **三层**：Framework（Dart）/ Engine（C++，Skia→**Impeller**）/ **Embedder**（平台 shell）。
- **Embedder API = 稳定 C ABI**（`embedder.h`：结构体 `struct_size` 版本化、枚举稳定、函数签名稳定）。每个平台实现一个 Embedder：**Android（Java，SurfaceView/TextureView + EGL/Vulkan surface）**、**iOS（ObjC，UIKit + CAMetalLayer + Metal）**、Web（CanvasKit）、以及第三方 **OHOS（`flutter_openharmony`，华为）**。
- **渲染**：Impeller = **Metal（iOS）/ Vulkan（Android API 29+）/ GLES 回退**；`FlutterRendererType{kOpenGL,kSoftware,kMetal,kVulkan}`。
- **输入/服务**：embedder 向 engine dispatch pointer/keyboard 事件；**Platform Channels** 做原生插件通信；**Semantics 树**喂给系统无障碍。
- **可借鉴**：把"引擎（渲染器+运行时）"与"embedder（surface/事件循环/输入/插件 ABI）"彻底分离。floom 的 `floom-platform` 契约**已经是这个 embedder 契约**，架构上超前——缺的只是各平台的 embedder 实现。

### 3.3 GPUI Mobile — 完整 `Platform` trait + wgpu + NDK/UIKit

- 实现 **完整 `gpui::Platform` trait**（与 `gpui_linux` 同构）。
  - **iOS**：Metal via **wgpu**、UIKit（`UIWindow` + `CAMetalLayer`）、CoreText、GCD dispatcher。
  - **Android**：Vulkan/GL via **wgpu**、NDK（`ANativeWindow`）、cosmic-text + swash 文本、ALooper dispatcher、JNI 入口（`NativeActivity` 生命周期 + 事件循环）。
- **功能**：触摸 tap-vs-scroll 状态机、momentum/fling（匹配原生 UIScrollView/OverScroller）、硬件+软键盘（NDK 键码）、safe-area insets、深色模式、emoji（Android 13+ CBDT 回退）。
- **构建**：cargo-ndk（Android，API 26+，arm64）、XcodeGen（iOS）。
- **可借鉴**：**wgpu 作为双平台单一渲染器是最大使能器**；重活是（a）每 OS 的生命周期/事件循环、（b）JNI/objc 桥、（c）文本系统。注意：GPUI Mobile 这些"纯逻辑"部分（触摸/键码/safe-area）**floom 已经搬完**（见 §1.2），所以 floom 在 UI 层比 GPUI Mobile 命名时更靠前。

### 3.4 三者共性结论

1. **渲染器选平台支持的 GPU API**：iOS=Metal、Android=Vulkan（GLES 回退）、OH=Vulkan/GLES。用 **wgpu** 可一次性覆盖 iOS+Android（GPUI Mobile 已证），但 **OHOS 的 wgpu 后端**曾被认为"无官方 wgpu 后端"——然而 `gpui_ohos` 实测默认 `wgpu::Backends::GL`（GLES/EGL 经 `OHNativeWindow`），且 `raw_window_handle` 已含 `RawWindowHandle::Ohos`，wgpu 直接据此建 surface。**floom 走 wgpu 路线在鸿蒙上的"唯一硬伤"已被证伪**——剩 vello-on-wgpu-GL 首帧验证（必要时 `floom-renderer-skia` 的 GLES 兜底）。
2. **事件循环 + 生命周期** 是每平台的独立重活（android-activity / UIKit-CFRunLoop / OH NDK 事件循环）。
3. **系统服务（IME/剪贴板/safe-area/外观/能力）** 需要 JNI / objc2 / NDK 桥；这些 floom 已用 typed stub 预留了接口面。
4. **干净的分层**（embedder 契约与引擎分离）让"写一次引擎、每平台填 embedder"可行——floom 已具备该分层。

### 3.5 Flutter for HarmonyOS / React Native for OpenHarmony —— 鸿蒙成熟参考（用户建议优先参照）

> 用户 2026-09-19 指示：鸿蒙接入应优先参考 **Flutter for HarmonyOS** 与 **React Native for OpenHarmony（RNOH）** 的成熟代码，因为二者已在 OHOS 上跑通。经核查，二者在"如何把跨平台引擎接到鸿蒙"上给出了**可直接照搬的范式**，且**关键结论：OHOS 没有 winit 路径**。

- **共同底座：ArkUI `XComponent` + `OHNativeWindow` + `NAPI`**。两个项目都不走 winit，而是：
  1. ArkTS 侧放一个 `XComponent({ id, type: 'surface', libraryname })` 作为渲染承载；
  2. 对应的 NAPI 模块（`libraryname`）在 `Init` 里经 `OH_NativeXComponent_GetNativeXComponent(env, context)` 取 `OH_NativeXComponent*`，注册 `OH_NativeXComponent_Callback`（`OnSurfaceCreated/Changed/Destroyed`、`DispatchTouchEvent`、`OnKeyEvent`、`OnMouseEvent`、`OnFocusEvent`、`OnUIInputEvent`、`OnFrameCallback`、`OnSurfaceShow/Hide`）；
  3. `OnSurfaceCreated(OH_NativeXComponent*, void* window)` 的 `window` 就是 `OHNativeWindow*` —— 这是鸿蒙的"原生窗口句柄"，等价于 Android 的 `ANativeWindow`、iOS 的 `CAMetalLayer` 承载的 layer。
- **Flutter-OH（Embedder 模型，floom 应直接对齐）**：
  - 三层 `ArkTS Embedding ↔ NAPI ↔ libflutter.so(C++)`；渲染通路 `XComponent Surface → FlutterNapi.xComponentAttach → PlatformViewOHOS → OHOSSurface → OHNativeWindow → Vulkan(VK_OHOS_surface)/GLES swapchain → RenderService`。
  - **GPU surface 做法**：从 `OHNativeWindow` 经 `VK_OHOS_surface` 扩展转成 `VkSurfaceKHR` 再建 `VkSwapchainKHR`。Impeller 在鸿蒙 Next 上即走 **Vulkan**（ArkGraphics 支持 Vulkan 后端）。**这证明 wgpu 在 OHOS 上并非"无后端"——缺的只是 `VK_OHOS_surface` 的 surface 创建 shim，而非整套 GPU 后端**。
  - VSync 经 `OH_NativeVSync`（`VsyncWaiterOHOS::AwaitVSync`）；事件/生命周期经 XComponent 回调 + `UIAbility.onForeground/onBackground → LifecycleChannel`。
  - 系统服务经 **MethodChannel / EventChannel**（NDK 侧 NAPI 桥），等价于 Android 的 JNI Platform Channel。
- **RNOH（Fabric 模型，floom 不需要但值得知道）**：RN 新架构走 `JSI(C++ 共享内存) + NAPI(C++↔ArkTS)`，`Fabric` 把 RN 组件经 **ArkUI C-API（`ArkUINode`）直接画原生控件**（不走声明式范式，用 `ContentSlot`+`NodeContent` 挂载），即"原生控件渲染"。**floom 不采用此模型**——floom 保留自己的 scene + vello，因此应跟随 Flutter 的 XComponent+GPU surface 模型，而非 RN 的 ArkUI 节点映射模型。RNOH 的 `TurboModule`（系统能力桥，分 `cxxTurboModule` 纯 C++ / `ArkTSTurboModule` 经 NAPI 调系统）对应 floom 的 `floom-platform-mobile` 能力包接线方式。
- **对 floom 的 actionable 结论**：
  1. **弃用 `crates/vendor/floom-winit-ohos` / winit 假设**；鸿蒙后端 = 直接 NDK/NAPI embedder（类 Flutter 的 `OHOSShellHolder`），不依赖 winit。
  2. **渲染（已证伪"需自写 shim"）**：实测 `ohos-rs/gpui-template`（`gpui_ohos`）**默认 `wgpu::Backends::GL`**——即 **GLES/EGL 经 `OHNativeWindow`**，而非 Vulkan/`VK_OHOS_surface`；`raw_window_handle` 已提供 `RawWindowHandle::Ohos` + `DisplayHandle::ohos()`，**wgpu 直接据此建 surface，floom 的 `SurfaceTarget` 零改造即可消费**。故 OHOS 唯一"硬伤"已被证伪：无需自写 surface shim。Vulkan/`VK_OHOS_surface` 仅作备选研究项。
  3. **事件/生命周期**：NAPI 回调 → Rust `floom-platform-harmony` 的 `translate`（与 `android.rs` 同构，touch 走 `DispatchTouchEvent`/`OnUIInputEvent`）；`OH_NativeVSync` 取代 winit 的 redraw 调度。
  4. **系统服务桥 = NAPI**（Android 是 JNI）：已 stub 的 `soft_input`/`safe_area`/`system_chrome`/`device_info`/`clipboard` 在 OHOS 侧改用 NAPI 实现（参照 Flutter MethodChannel / RN TurboModule）。
  5. **打包**：产物是 HAP（非 APK），结构 = ArkTS 入口（`EntryAbility`）+ C++ 共享库（`libentry.so`，承载 NAPI 桥）+ Rust `staticlib` 链入；需 OHOS SDK（API 12+）+ DevEco/hvigor + `hdc`。
  6. **可复用的现成 NAPI 桥（比手搓 XComponent 更优）**：`ohos-rs/gpui-template` 用的不是手搓 `OH_NativeXComponent_*` 回调，而是 **`openharmony-ability` + `openharmony-ability-derive`（`@ohos-rs/ability` npm）** 现成桥：ArkTS 侧 `EntryAbility extends NativeAbility`（`moduleName` 指向 Rust cdylib），Rust 侧 `#[openharmony_ability_derive::ability] pub fn openharmony_app(app: OpenHarmonyApp)` 收到 `OpenHarmonyApp`（含 `native_window()`→`raw_window_handle()`、`content_rect()`、SurfaceCreate/输入事件派发）。floom 应直接依赖这套 crate **复用 NAPI 桥**，只写 `floom-platform-harmony`（类比 `gpui_ohos`）对接 floom 的 `Platform` trait；构建用 `ohrs build`（ohos-rs CLI）出 HAP。

---

### 3.6 嵌入式 surface / native↔floom 互相嵌套（移植 Flutter 的 SurfaceView/TextureView + PlatformView）

用户建议：把 Flutter 的 SurfaceView/TextureView 思路移植过来，最终实现 native 与 floom view 互相嵌套（例如 Android Compose 组件中嵌入一个 floom view）。

**Flutter 已成熟的两条嵌入路径（核查来源见 §9）**：
- **native 嵌入 Flutter**＝`FlutterView`：它本身就是一个 `android.view.View`，继承 `SurfaceView`（`RenderMode.surface`，性能最佳，但**不能**在 z 序上夹在其他 View 之间、也**不能**被变换/动画）或 `TextureView`（`RenderMode.texture`，可被变换/裁剪/z 序，但多一次 app 合成、有延迟）。`attachToFlutterEngine(engine)` 把引擎绑到该 View 的 `Surface` 上；可被加进任意 ViewGroup，包括 Compose 的 `AndroidView`。这正是"SurfaceView/TextureView 拆分"存在的原因——为了在滚动/变换的原生容器里嵌 Flutter。
- **Flutter 嵌入 native**＝`PlatformView`（`AndroidView` widget + `PlatformViewFactory` 返回真实 `android.view.View`）：两种合成模式——Hybrid Composition（原生 View 真在层级里，Flutter 拆成 bg/overlay 两层 `ImageReader`，SurfaceFlinger 合成；保真度最高但线程合并掉 FPS）与 Texture Layer Hybrid Composition（原生 View 画进 texture 再上传 Impeller；Flutter 性能好、变换可用，但快速滚动会卡、含 SurfaceView 时失效）；另有 HCPP（API 34+ / Vulkan / Impeller，SurfaceFlinger 合成两个原生 surface）。
- 多视图：`FlutterView` 可在引擎间 attach/detach；一个引擎＝一个 Dart isolate；`FlutterEngineGroup` 跨引擎共享资源。

**floom 的 L4 模型已有对应接缝（不需要大改类型）**：
- `SurfaceTarget { surface, size, scale, window_handle: RawWindowHandle, display_handle: RawDisplayHandle }`（`crates/floom-platform/src/surface.rs`）用的就是 `raw-window-handle`（OS 中立、ABI 安全、不引 winit）。渲染器正是由这对句柄建 swapchain——**这正是把 floom surface 绑到"外部提供的原生 Surface"所需的原语**（SurfaceView/TextureView 的 `Surface` → `ANativeWindow*` → `RawWindowHandle::AndroidNdk`）。
- `Window::create_surface(config) -> SurfaceId`（`window.rs`）——一个 Window 可挂多个 surface，子 surface / 内嵌 surface 已被建模。
- `floom_types::NativeViewId` **已存在**（`crates/floom-types/src/ids.rs:117`）——"floom 嵌入 native"的身份类型已预置，只差平台 + widget 落实。
- `PlatformEvent` 以 `WindowId` 为键（`event.rs`）——因此内嵌的 floom 窗口（方向 A）只需收到以该 `WindowId` 为键的 `PointerMoved/Key/Scroll/WindowResized`，由原生宿主注入即可。

**两个方向 + 互相嵌套**：
- **方向 A — native 嵌入 floom（用户的 Compose 例子）**：一个 `FloomView`（自定义 `SurfaceView`/`TextureView` 子类，类比 `FlutterView`）经 JNI 把它的 `Surface` 交给 floom（类比 `FlutterJNI#nativeSurfaceCreated(long, Surface)`），floom 引擎绑定并渲染；Compose 用 `AndroidView { factory = { FloomView(...) } }` 承载。Compose 的 `PointerInputChange`/`KeyEvent` 转发进 floom 事件队列（以内嵌 `WindowId` 为键）。
- **方向 B — floom 嵌入 native**：一个 `NativeView` widget（L6/7）持 `NativeViewId`；Android 上平台在该 widget 布局 rect 处实例化真实 `android.view.View`；两种合成模式——Hybrid（原生 View 在真层级，floom 抠透明洞）与 Texture（原生 View → `SurfaceTexture` → floom texture 节点）。
- **互相嵌套**＝两者皆有 + 深层嵌套时的坐标/事件对账（最难，最后做）。

**floom 需新增的扩展（具体）**：
1. `Platform`/`AndroidPlatform` 需要 `create_embedded_window(parent_raw_handle: RawWindowHandle, rect, config) -> WindowId`（或把 `WindowConfig` 扩成 `Option<ParentSurface>`）。Android 上即从宿主 SurfaceView/TextureView 的 `ANativeWindow*` 建窗口。当前 `create_window(config)` 无父 surface 参数、永远建 winit 顶层窗口。
2. 外部事件注入：`AndroidPlatform` 需 `push_event(PlatformEvent)` 入口，供 FFI 层（Compose 宿主）把 pointer/key/scroll/resize 注进内嵌窗口（当前 `poll_events` 只排 winit 来源）。
3. 方向 B：`create_native_view(NativeViewId, rect)` + `NativeView` widget + Hybrid/Texture 合成。
4. 由外部 `ANativeWindow*` 建 wgpu swapchain：wgpu 有 Android surface 支持（`VK_KHR_android_surface` / 其 Android 后端），比 OHOS 更可行——方向 A 渲染在"Android 上能渲一个 floom surface"之后即可落地。

**层卫生（关键）**：
- 绑定胶水（`FloomView` Kotlin 类、JNI/FFI seam、Compose `AndroidView` factory、事件转发）落在**应用/FFI 层（L7+）**，**不是 L4**。L4（`floom-platform-android`）只暴露 Rust 表面："用提供的 `ANativeWindow*` + rect 建 Window"与"以 `WindowId` 注入 `PlatformEvent`"。这正映射 Flutter 的拆分：`FlutterJNI`/引擎（native lib）vs `FlutterView`/`FlutterActivity`/`FlutterFragment`（应用 classpath 里的 Android embedding 类）。
- `floom-platform`（L4）必须保持句柄化（`RawWindowHandle`），绝不可 winit 专属——`SurfaceTarget` 已合规。
- OHOS 类比：同模型经 `XComponent`/`OHNativeWindow*`+NAPI（无 SurfaceView/TextureView 概念；"可变换"的考量映射为"XComponent 是否处在变换/滚动的 ArkUI 容器里"→ 是则需 texture 式路径）。

**推荐阶段（在现有 Android PoC 骨架之上的新增 track）**：
- P0：Android 上渲出一个 floom surface（ANativeWindow* + wgpu swapchain + vello）——即现有 §7.2 PoC。
- P1（方向 A，用户的例子）：扩 `create_window`/`AndroidPlatform` 绑定外部 `ANativeWindow*`、加 `push_event`、写 `FloomView` JNI + Compose `AndroidView` 胶水 → "Compose 嵌入 floom view"。
- P2（方向 B）：`NativeView` widget + `create_native_view` + Hybrid/Texture。
- P3（互相嵌套）：嵌套组合 + 坐标/事件对账。


## 4. 逐平台缺口与移植方案

### 4.1 Android（最高可行性）

- **现状（2026-09-19 更新）**：`android.rs` 已是**真实 `Platform` 实现**（`AndroidPlatform`/`AndroidWindow`/`AndroidDriver`/`run`/`set_android_app`，走 lapce/winit fork 的 android 后端），并已在 **API 34 arm64 模拟器**上验证首帧与 tap（§7.2）；剩余为 JNI 服务回值（`android-activity` / `jni` / `ndk` 尚未加入依赖）。Rust 目标 `aarch64-linux-android` 已装；AVD 镜像 `system-images;android-34;google_apis;arm64-v8a` 已装（AVD 名 `floom2api34`）。
- **要做的事**：
  1. 新增 `android.rs` 实现 `Platform`/`Window`/`Ime`/`Clipboard`（`#[cfg(target_os="android")]`），用 **`android-activity`**（`NativeActivity`/`GameActivity`）接管生命周期与事件循环（参照 Slint `android-activity` 后端、GPUI Mobile `jni_entry.rs`）。
  2. 从 `ANativeWindow` 取 `RawWindowHandle::AndroidNdk`，在 `winit_backend` 风格的 `target()` 里枚举该变体，建 **wgpu Vulkan** surface（GPUI Mobile 证实 Vulkan 优先、GL ES 3.0 回退，API 26+）。
  3. **JNI 桥**（`jni` crate）：接入已写好的 `soft_input`（show/hide + 高度）、`safe_area_insets`（取 `WindowInsets`）、`system_chrome`（状态栏/导航栏着色）、`clipboard`、`device_info`。可仿 Slint 用 `build.rs` + `javac`+`d8` 嵌 Java helper。
  4. 输入：`android_table.rs` 的 NDK 键码映射直接复用；触摸走 `floom-gesture::TouchRecognizer`（已就绪）；`poll_events` 把 `MotionEvent`/`KeyEvent` 翻成 `PlatformEvent`。
  5. 文本：floom 文本系统需能从 `/system/fonts` + assets 加载字体（参照 GPUI Mobile 的 CBDT emoji 回退处理 COLR v1）。
- **关键决定**：lapce/winit fork（`Cargo.toml` 里 `git="https://github.com/lapce/winit"`）是否带 Android 后端？若带，可直接复用桌面 `winit_backend`；若不带，走 `android-activity` 直连（Slint 路线）——**这是 Step-0 必须核查的**。
- **风险**：① lapce/winit fork 移动端支持未知；② wgpu-Vulkan 在模拟器（SwiftShader/ANGLE）上的可用性；③ JNI 线程模型（不可每帧 JNI hop，参照 Zdroid 的 AChoreographer vsync 设计）。

#### 4.1.1 嵌入式 surface / 互相嵌套（移植 Flutter SurfaceView/TextureView + PlatformView）

要点与分层约束见 §3.6。Android 在此场景下是**最成熟**的目标：wgpu 已有 Android surface 支持（`VK_KHR_android_surface`），`ANativeWindow*` 直出 `RawWindowHandle::AndroidNdk`，`SurfaceTarget` 可零改造消费；`create_embedded_window` + `push_event` 两个扩展即可让 Compose 通过 `AndroidView` 嵌一个 `FloomView`。方向 B（floom 嵌 Android View）需 `NativeViewId`（已存在）+ Hybrid/Texture 合成，工作量更大，放 P2。


### 4.2 iOS

- **现状**：`floom-platform-ios` 仅 `HostPlatform::new()`；Rust **iOS 目标未装**（`rustup target add aarch64-apple-ios`/`aarch64-apple-ios-sim`）；Xcode 已装（`/Applications/Xcode.app`，xcrun 72）。
- **要做的事**：
  1. `ios.rs` 实现 `Platform`/`Window`/`Ime`/`Clipboard`，用 **`UIApplicationMain` + `UIWindow` + `CAMetalLayer`**（GPUI Mobile `window.rs`）；CFRunLoop 作为事件循环。
  2. 从 `CAMetalLayer` 取 `RawWindowHandle::UiKit`，建 **wgpu Metal** surface。
  3. **objc2 桥**：接入 `soft_input`（`keyboardWillShow`/`WillHide` + 高度/`UIResponder` 输入）、`safe_area_insets`（`safeAreaInsets`）、`system_chrome`、`clipboard`、`device_info`。
  4. 输入/文本：触摸状态机 + NDK 键码（已就绪，iOS 侧键码另映射）、CoreText 或复用 floom 文本。
- **风险**：① Apple 工具链摩擦（需 Xcode、provisioning、XcodeGen/生成 xcodeproj）；② Rust iOS 目标 + wgpu-metal 交叉编译；③ 模拟器 Metal 可用性（需真实设备验证更可靠）。

### 4.4 Terminal (TUI)

前三个平台是"把 OS 表面接进来"，终端是唯一一个**输出本身就不同构**的目标：终端不是像素网格，而是**字符网格**。因此这里的关键设计决定不是"用哪个 GPU API"，而是"要不要把画面降采样成彩色方块"。

- **被否的方案**：先软件光栅化、再按半块（`▀`/`▄`）降采样成 ANSI（即 `chafa`/`viu` 的路子）。它能自动吃下所有节点类型（rect/path/image/filter），但**文字必然糊**——一个 14px 的字在本机常见 120×40 网格下只占约 1 个单元格。能跑不等于能用。
- **采用的方案**：原生字符网格。`Rect` 填充背景色；`Text` 节点把**它当初被排版的那些字符**写进字形所在的单元格。
- **这带来一个 IR 改动**（为什么必须由 shared IR 承担）：shaped text 是**有损**的。`GlyphRun` 只带字形 id、位置与字体 blob，连字/回退字体让"字形 id → 字符"是多对多映射，任何下游都无法还原。所以 `ResourceKind::GlyphRun` 现在额外保留源字符串 `text: Arc<str>`（生产注入点唯一：`crates/floom-app/src/lib.rs` 的 `intern_glyphs`，那里本来就有 `text`）。同一份字符串将来也是文本提取、无障碍树、命中测试要用的东西——TUI 只是第一个消费者。
- **诚实放弃的部分**：字符网格表达不了的东西（矢量 path、图片、滤镜、自定义 shader、渐变）**跳过而非近似**。近似（例如拿渐变的第一个色标当纯色）是对"场景到底要了什么"的沉默说谎。跳过量由 `PaintStats::skipped` 计数暴露，调用方能自己看见画面少了什么。
- **已交付（T1 切片）**：`crates/floom-renderer-tui` —— `TuiGrid`（字符网格 + 单元 => 列的映射 + 直 alpha 合成）、ANSI truecolor 编码（同色相邻不重复发转义）、`TuiPresenter`（`Presenter`，输出到可注入的 writer，便于测试逐字节断言）、以及 `present_diff`（首帧清屏归位、之后只发差异单元格的 cursor home，是终端性能的关键）。覆盖 stripe：单元测试（grid/ansi/presenter）21 项 + `crates/floom-app/tests/tui_backend.rs` 三条端到端（真实 `WidgetHost` + `Text` widget 的字符落到网格、重复帧稳定性、列宽改变映射随之改变）。变异战役 7 杀 7（`scripts/mutate-t1-tui.sh`）。
- **已交付（T2 切片）**：`crates/floom-platform-tui` —— 终端尺寸探测（unix `TIOCGWINSZ` ioctl，回落 `COLUMNS`/`LINES`，再回落 80×24）、`InputParser`（状态机把字节翻成 `PlatformEvent`：ASCII 与 UTF-8 **经由 `ImeCommit` 提交**，因此 CJK 等非 Latin 输入不丢；箭头/Home/End/Delete/PageUp/PageDown 经 `KeyCode`，Ctrl/Alt 和 CSI `;mod` 修饰正确还原）、OSC 52 剪贴板（`set_text` 写 base64 序列，`get_text` 返回 `None`，因终端回读需要同步应答协议）。`TuiPlatform` 复用 `HostPlatform` 承担窗口/事件簿记（与 embedded 同构），`ime()` 返回 `None`（终端无原生输入法表面）。覆盖 stripe：平台 crate 单元测试 16 项（`scripts/mutate-t2-tui.sh` 见下）+ `crates/floom-app/tests/tui_platform.rs` 三条端到端（`AppRunner` 驱动真实 widget 树：建窗→首帧渲染标签、注入 Latin 文本被分发进树、注入 CJK `你` 也经 IME 路径被提交）。
- **已交付（T3 切片 — 真实运行闭环，2026-09-28）**：把 T1/T2 的 `TuiPresenter` + `TuiPlatform` 接到一个真正的终端事件循环。`crates/floom-platform-tui/src/run.rs` 实现 `run::<TuiApp>`：`TerminalGuard` 进入时切 raw mode + 交替屏（仅 tty；管道/CI 检测后原样留空，故 headless 也能跑）、安装 `SIGWINCH` 处理 resize、`stdin` 字节流经独立线程喂入、Ctrl-C(0x03)/Ctrl-D(0x04) 作为退出信号（不进 widget 树）、`Drop` 自动恢复终端。`floom2::run_tui(build)`（`#[cfg(feature = "tui")]`）是门面单调用入口，`examples/hello` 是可运行示例。验证：`cargo run -p floom-platform-tui --example hello` 以管道（headless）运行，0 错将 `Hello from floom2 TUI!` 渲染进字符网格；宿主 `cargo check -p floom2 --features tui` 0 错；`floom-renderer-tui` **21** + `floom-platform-tui` **22** 单测全绿。TUI 自此从"测试里能跑"升级为"`cargo run` 能用"——原 §4.4「仍缺」段已关闭。

### 4.3 Harmony（路径已清晰，硬伤降级）

- **现状**：`floom-platform-harmony` 仅 `HostPlatform::new()`；本机 **`hdc`/`deveco` 全无**，鸿蒙验证在本机**不可能**；**原假设的 `crates/vendor/floom-winit-ohos` 路径不成立——OHOS 没有 winit 后端**（Flutter/RN 均走 XComponent+NAPI 直连，已核查）。
- **要做的事**（参照 Flutter-OH / RNOH 成熟实现，不走 winit）：
  1. `harmony.rs` 实现 `Platform`/`Window`/`Ime`/`Clipboard`，复用 **`openharmony-ability` + `openharmony-ability-derive`** 现成 NAPI 桥（ArkTS `EntryAbility extends NativeAbility` + Rust `#[ability] fn openharmony_app(app: OpenHarmonyApp)`），而非手搓 `OH_NativeXComponent_*` 回调；`OpenHarmonyApp.native_window()`→`raw_window_handle()` 提供 `OHNativeWindow*`、`content_rect()` 提供安全区/布局，`Event::SurfaceCreate` 触发延迟建 renderer。底层仍是 ArkUI `XComponent`+NDK（由该 crate 封装）。
  2. **渲染（"硬伤"已被证伪，无需 shim）**：实测 `gpui_ohos` 默认 `wgpu::Backends::GL`（**GLES/EGL 经 `OHNativeWindow`**），`raw_window_handle` 已含 `RawWindowHandle::Ohos`+`DisplayHandle::ohos()`，wgpu 直接据此建 surface。**floom 的 `SurfaceTarget`（本就吃 `RawWindowHandle`）零改造即可消费**，vello 走 wgpu-GL 即可首帧（需验证 vello 对 wgpu-GL 后端的特性支持，必要时用 `floom-renderer-skia` 的 GLES 兜底）。**Vulkan/`VK_OHOS_surface` 仅作备选研究项**。工作量从"月级 GPU 后端"降为"0——确认 vello 在 wgpu-GL 上的首帧 + 验证目标设备 GLES 可用"。
  3. **系统服务桥 = NAPI**（Android 的 JNI 等价）：接入已 stub 的 `soft_input`/`safe_area`/`system_chrome`/`device_info`/`clipboard`（参照 Flutter MethodChannel / RN TurboModule）。
  4. 输入：`floom-gesture::TouchRecognizer` + 键码映射（OHOS 键码另建表，可参照 RN/Flutter 的 OHOS 键码映射）直接复用逻辑层。
- **风险（更新）**：① 本机无工具链（需购置 DevEco/OH SDK/`hdc`，外部成本）；② NAPI 与 Rust 事件循环的对接（但 `openharmony-ability` 已封装大部分，繁琐度显著下降）；③ 目标设备 GLES/EGL 可用性（比 Vulkan 更普遍，但仍须真机/模拟器实测）；④ 原"wgpu 无 OHOS 后端"风险经 `gpui_ohos` 实测已**证伪**——wgpu GL 后端 + `raw_window_handle::Ohos` 直接可用，无需 surface shim；剩 vello-on-wgpu-GL 的首帧验证。
- **参照仓库**：`flutter/flutter` 的 OHOS 适配（`flutter_openharmony`，华为）、`linhandev/ohos_react_native`（RNOH）、OpenHarmony `interface_sdk_c` 的 XComponent/NativeWindow 文档。

---

## 5. 工作量估算（人周，单工程师）

| 工作模块 | Android | iOS | Harmony | 备注 / confidence |
|---|---:|---:|---:|---|
| Step-0 核查（lapce/winit fork 移动端、目标/AVD） | 0.5 | 0.5 | 1.0 | 高 |
| 生命周期 + 事件循环接入 | 1.5 | 1.5 | 2.5 | 中（android-activity/UIKit/OH NDK） |
| 窗口 + wgpu surface（句柄变体枚举） | 1.0 | 1.0 | 2.0 | 中；OH 含"选渲染路线"决策 |
| 输入翻译（触摸/键码/滚动） | 0.5 | 0.5 | 0.5 | **高**（逻辑已就绪，仅接线） |
| IME 桥（JNI/objc2/NAPI） | 1.0 | 1.0 | 1.5 | 中 |
| 剪贴板 / safe-area / system-chrome 原生实现 | 0.5 | 0.5 | 1.0 | **高**（类型已就绪） |
| 文本系统移动端硬化（字体/emoji 回退） | 0.5 | 0.5 | 0.5 | 中 |
| 8 个设备能力包原生实现 | 1.0 | 1.0 | 1.5 | 中（类型已就绪） |
| **空窗口渲染 + 基础输入里程碑小计** | **≈4–6** | **≈4–6** | **≈6–10** | 中 |
| 完整功能对等（深色/无障碍/多窗口等） | +2–3 | +2–3 | +3–4 | 低 |
| CI / AVD / 模拟器 / 真机验证 | +1.5 | +1.5 | +1.0（本机不可验） | 中 |
| **单平台总计** | **≈7–10** | **≈7–10** | **≈9–14** | 低–中 |
| **三平台合计包络** | | | **≈ 23–34 人周** | 未含 OH 工具链购置 |

> 说明：估算为工程判断，非实测。Android/iOS 因 wgpu + 成熟工具链置信度中；Harmony 路径已清晰（XComponent+NAPI 直连、wgpu 仅需 `VK_OHOS_surface` shim，参照 Flutter/RN 成熟实现），但因本机零工具链、需先购置 DevEco/OH SDK（外部成本）置信度中–低。
> 复用项（触摸状态机、NDK 键码、soft_input/safe_area/system_chrome 类型、8 包类型）已从估算中扣除其"从零写"成本。

---

## 6. 风险登记

| # | 风险 | 影响 | 缓解 |
|---|---|---|---|
| R1 | **lapce/winit fork 不含 Android/iOS 后端** | 无法复用桌面 `winit_backend`，需改走 android-activity / 自接 UIKit | Step-0 核查；若缺失则直接依赖上游 winit（含 android-activity/iOS）或扩展 fork |
| R2 | **wgpu 在 OHOS 的 surface 创建缺失（已证伪）** | 经 `gpui_ohos` 实测：wgpu 默认 `Backends::GL`（GLES/EGL），`raw_window_handle` 已提供 `RawWindowHandle::Ohos`+`DisplayHandle::ohos()`，wgpu 据此直接建 surface；floom 的 `SurfaceTarget` 零改造即可消费。**无需自写 surface shim，无需 Vulkan**。 | 确认 vello 在 wgpu-GL 后端的首帧；目标设备 GLES 可用性真机验证；Vulkan/`VK_OHOS_surface` 仅作备选 |
| R3 | 模拟器 GPU 后端不完整（Android SwiftShader / iOS 模拟器 Metal） | 冒烟可能在模拟器失败，需真机 | 关键里程碑用真机验证；AVD 装含 Play 的 x86_64 镜像 |
| R4 | JNI/objc2/NAPI 每帧跨语言 hop 成本 | 滚动/输入掉帧 | 仿 Zdroid：AChoreographer vsync、批量事件、避免每帧桥 |
| R5 | 本机工具链缺失（iOS 目标、Android AVD、OH SDK） | 本地无法验证 Android/iOS/OH | 先补齐工具链（见 §7.3）；OH 需购置 DevEco |
| R6 | Apple 签名/provisioning 摩擦 | iOS 真机部署慢 | 早期用模拟器 + XcodeGen 生成工程 |
| R7 | 嵌入式 surface / 互相嵌套的事件注入与 Hybrid 合成复杂度 | 方向 A 需外部事件注入路径；方向 B 需 Hybrid/Texture 合成与坐标对账，易事件错位/掉帧 | 先做方向 A（仅注入、成本低）；Hybrid 参考 Flutter PlatformView 的 bg/overlay ImageReader 分层；纹理路径先覆盖静态/弱变换容器 |

---

## 7. 推荐路径与 PoC

### 7.1 推荐顺序

**Android → iOS → Harmony**。理由：Android wgpu-Vulkan 最成熟、cargo-ndk 工具链轻、且 GPUI Mobile 已证同构路径；iOS 次之（Metal+wgpu，但 Apple 工具链摩擦）；Harmony 最后（仍走 XComponent+NAPI 直连、wgpu 仅需 `VK_OHOS_surface` shim，但本机零工具链、需先购置 DevEco/OH SDK）。

### 7.2 Android PoC 定义（DoD）

目标：证明**整条链**端到端可行，而非追求功能完整。

- [x] `cargo check --target aarch64-linux-android -p floom-platform-android` 通过（真实 `Platform` 实现接入）。**已达成**（0 错 0 警）。
- [x] 应用层链路能为 Android 编译：`cargo check -p floom-app --target aarch64-linux-android` 通过（app runner + vello renderer + L4 后端）。
- [x] 可运行入口已落地：`examples/floom2-android`（`cdylib` + `android_main` → `set_android_app` → `AndroidRunner`/`run`），交叉编译 `--features real` 0 错 0 警（登记于 `docs/2.0/EXAMPLES.md`）。
- [x] 在 Pixel 模拟器（含 Play 的 x86_64 或 arm64 镜像）打开**一个空 floom 窗口**，vello/wgpu-Vulkan 成功呈现首帧。**已达成（2026-09-19）**：`system-images;android-34;google_apis;arm64-v8a` + Pixel 6 profile 建 AVD `floom2api34`，无头启动（`-gpu swiftshader_indirect`）后 logcat 出 `GPU adapter: SwiftShader Device (LLVM 10.0.0) (Vulkan)` 与 `Displayed com.example.floom2/android.app.NativeActivity: +4s709ms`，screencap 可见按钮。
- [x] 派发一次 tap → `poll_events` 收到 `PlatformEvent::Pointer`（`floom-gesture::TouchRecognizer` 判为 tap）。**已达成（2026-09-19）**：`adb shell input tap` 落在按钮**可见位置**的中心 → `PointerButton src=Touch{finger_id:0} mapped=Primary` → `on_click` → 计数 +1（logcat `pointer -> button click, counter = 2`）；对照组（同一 x、按钮外）不触发。
- [x] **Clipboard / Ime 经 JNI 生效**（证明系统服务桥通）。**已达成（2026-09-19）**：`impl Clipboard` / `impl Ime` 不再是 inert stub，两者都在模拟器上取到真实回值 —— 见 §7.2.2。
- [ ] `safe_area_insets` 经 JNI 返回非默认值。**未达成**：`safe_area/android.rs` 仍返回 `Insets::ZERO`。它与"键盘出现没人通知应用"是同一个 `WindowInsets` 通知缺口（§7.2.2 末尾），宜一同解决。
- [x] 架构门 `bash scripts/check-architecture-deps.sh` 仍 PASS（L4 不依赖更高层）。

**当前结论**：编译级、入口级、**设备级首帧、tap、剪贴板、软键盘四类均已在模拟器上收口**；剩余仅 `safe_area`（与 `InsetsChanged` 通知缺口同源）与 IME **文字回传**（`InputConnection`）。

### 7.2.1 设备运行暴露并已修复的两个真实缺陷

首帧与 tap 不是"跑起来就完事"——设备级验证第一次跑就抓到两个**只在 scale≠1 的真实设备上才会现形**的缺陷，桌面 1× 测试全部绿着：

1. **presenter 从不消费 scale factor**：场景按 logical 单位构建，target 是 device 像素，但 vello 的根变换只有 `translate(-damage origin)`、软件后端也只按缓冲尺寸换算。后果是**应用按 1× 画进一个 ×2.625 的靶面**（UI 缩在角落），而命中测试仍把指针**除**以 scale 变成 logical —— 于是**点击永远落在"实际绘制位置 / scale"处**：看着点在按钮上，事件却打在别处。修法是把 scale 并入唯一的那一个根变换（`crates/floom-renderer-vello::scene_transform`）并在软件后端逐几何换算；由 `a_logical_scene_is_painted_at_the_scale_factor`（软件后端像素断言）与 `the_scene_transform_scales_then_translates`（纯算术，无需 GPU）钉住。
2. **glyph run 里存的是 offset 而不是位置**：parley 的 `Glyph.x` 是"相对笔位的偏移 + advance"，而 `floom-text::Glyph` 是"已定位字形"，两个渲染器都按后者绘制 —— 于是**整串字形叠在 run 原点上**，文字糊成一团；而**行宽是 advance 之和、依旧正确**，所以布局留下的位置分毫不差，只有屏幕看得出来。修法是在 shaper 拷贝时累加笔位（`crates/floom-text/src/layout.rs`），由 `glyph_positions_advance_across_the_run` 钉住。

> 设备侧备注：`adb shell input tap` 用的是 **display 坐标**，而平台的事件坐标是**窗口相对**的（本模拟器 status bar 占 128 device px）。因此设备验证必须"先截屏量出控件的可见 rect，再点它的中心"，不能拿逻辑/窗口坐标反推。已实测：按钮绘制于 device y 230..335，tap 在 250/283/310 命中、229 与 336 均落空 —— 命中区与绘制区边界一致。

### 7.2.2 JNI 服务桥：从 inert stub 到设备回值

三处改动把 `Clipboard` / `Ime` 从 stub 变成真实现，且都**只在设备上可核实**：

1. **桥没接线**：`floom-platform-mobile::jni` 早已写好 `set_java_vm`/`set_activity`，但**没有任何代码调用它们** —— 于是 12 个能力包全是死的。现由 `floom-platform-android::run` 在启动时从 `AndroidApp::vm_as_ptr`/`activity_as_ptr` 交接（setter 收 `*mut c_void`，后端不必依赖 `jni`）。
2. **Clipboard**：Android 无 NDK 剪贴板 API，只能走 JNI 的 `ClipboardManager`。实测 `write=true`；`read_back=None` 是 **Android 10+ 的焦点限制**，系统日志给出确证原因：`Denying clipboard access to com.example.floom2, application is not in focus`（首次读发生在 `ActivityTaskManager` 报 `Displayed` 之前约 30ms）。焦点稳定后再读 → `Some("floom2-android clipboard probe 9f3c")`，**逐字节一致**，证明值确实存在于系统剪贴板而非仅进程内。
3. **Ime**：winit 的 `request_ime_update` 走 NDK `ANativeActivity_showSoftInput`，对 `NativeActivity` **不可能生效** —— 请求走完全链后被框架拒绝：`Ignoring showSoftInput() as view=…NativeContentView is not served` / `onFailed at PHASE_CLIENT_VIEW_SERVED`。改为 Java 侧通行做法：向 activity 挂一个 1×1、alpha=0 的 `EditText` 锚点并让 `InputMethodManager` 为**它**显示输入法（`floom-platform-mobile` 的 `keyboard::soft_input::android`）。实测 `mServedView=android.widget.EditText{… 0,0-1,1}`、`mInputShown=true`、`ImeTracker: onShown`，截屏可见键盘。

**仍未收口、且已定位到确切位置的一处**：**键盘出现时没有任何事件通知应用**。`android-activity` 会抛 `MainEvent::InsetsChanged`，而 winit 的 Android 后端把它丢掉了（`winit-android/src/event_loop.rs`：`TODO: handle Android InsetsChanged notification`）；窗口虽声明 `adjustResize` 却**并未真的 resize**，所以也不产生窗口事件。后果有二：应用**无法避开被键盘遮住的区域**（`soft_keyboard_state()` 读得出 336.4 logical px 的高度，但要有人先把它唤醒）；`safe_area_insets` 同样缺这条通知。修法是在该 fork 里转发 `InsetsChanged`（或由锚点注册 `OnApplyWindowInsetsListener` 回调 Rust）。

**另一处已知边界**：`Ime` 目前只驱动键盘的**显隐**，**不接收键入的字符** —— 把文字送回应用需要 `InputConnection`（`commitText`/`getExtractedText`），锚点的文本从未被读取。

**CI 缺口（本次一并补上）**：上述 JNI 代码位于 `#[cfg(target_os = "android")]` 之下，而 CI 从不跨编译 Android，`floom-platform-mobile` 连包列表都没进 —— 所以该模块**从未被编译过**，其中 5 个硬错误一直藏着（`OnceLock<jobject>` 不是 `Sync`、全局引用被当局部引用释放等）。现已在 `ci.yml` 增加 `android-check` 作业（`cargo check --package=floom-platform-mobile --package=floom-platform-android --target aarch64-linux-android`，无需 NDK），并把 `floom-platform-mobile` 补进 `FLOOM2_PACKAGES`。已做变异证伪：在 Android 专属代码里注入一个类型错误 → 该作业变红，而 host 构建**依旧全绿**。

### 7.3 前置核查清单（在写代码前）

1. `git grep` / 读 lapce/winit fork 的 `crates/floom/src/platform/` 确认是否含 `android` / `ios` 后端（R1）。
2. `rustup target add aarch64-apple-ios aarch64-apple-ios-sim`（iOS）。
3. `sdkmanager` 装一个 Android AVD 系统镜像（本机 `emulator` 在、无镜像）。
4. 确认 `wgpu` 版本对 Android-Vulkan / iOS-Metal 的特性开关（workspace 内 wgpu 来自 vello 的 re-export，需对齐版本）。

---

### 7.4 Harmony PoC 定义（DoD，参照 Flutter/RN 成熟实现）

目标：证明鸿蒙"`openharmony-ability` NAPI 桥 + `OHNativeWindow` + wgpu-GL(GLES/EGL)"整条链端到端可行（不走 winit，不手搓 XComponent）。

- [ ] `cargo check --target aarch64-unknown-linux-ohos -p floom-platform-harmony` 通过（真实 `Platform` 实现接入；`#[cfg(target_os="linux")]` + OHOS target 判定或专门 `cfg`）。
- [ ] ArkTS `EntryAbility extends NativeAbility`（`@ohos-rs/ability`）+ Rust `#[openharmony_ability_derive::ability] fn openharmony_app(app)`；`OpenHarmonyApp.native_window()` 暴露 `OHNativeWindow*`，`Event::SurfaceCreate` 触发延迟建 renderer（参照 `gpui_ohos::window.rs`）。
- [ ] 用 `raw_window_handle::RawWindowHandle::Ohos` + `DisplayHandle::ohos()` 经 **wgpu `Backends::GL`（GLES/EGL）** 从 `OHNativeWindow` 建 surface，vello（wgpu-GL）成功呈现首帧（DevEco 模拟器或真机）。
- [ ] 派发一次 touch → NAPI 回调 → Rust `translate` → `poll_events` 收到 `PlatformEvent::Pointer`。
- [ ] `soft_input` / `safe_area_insets` 经 NAPI 返回非默认值（证明系统服务桥通）。
- [ ] 架构门 `bash scripts/check-architecture-deps.sh` 仍 PASS（L4 不依赖更高层）。
- [ ] **工具链前置**：装 OHOS SDK（API 12+）+ DevEco/hvigor + `hdc`（本机当前全无，须先购置）。

---

### 7.5 嵌入式 surface / 互相嵌套 设计（Android 优先，移植 Flutter）

目标：先实现"native 嵌入 floom"（Compose `AndroidView` 嵌 `FloomView`），再实现"floom 嵌入 native"，最后互相嵌套。

- [ ] **P0 前置**：§7.2 PoC 达成——Android 上能渲出一个 floom surface（ANativeWindow* + wgpu-Vulkan + vello 首帧）。
- [ ] **P1 — native 嵌入 floom**：`Platform`/`AndroidPlatform` 新增 `create_embedded_window(parent: RawWindowHandle, rect, config)`（从宿主 SurfaceView/TextureView 的 `ANativeWindow*` 建窗口）；新增 `push_event(PlatformEvent)`（FFI 层注入 pointer/key/scroll/resize，键为内嵌 `WindowId`）；写 `FloomView`（Kotlin，`SurfaceView`/`TextureView` 子类，类比 `FlutterView`）+ JNI seam（类比 `FlutterJNI#nativeSurfaceCreated`）+ Compose `AndroidView` factory；事件转发（`PointerInputChange`/`KeyEvent`→`PlatformEvent`）。
- [ ] **P2 — floom 嵌入 native**：新增 `NativeView` widget（L6/7，持 `NativeViewId`）；`Platform::create_native_view(NativeViewId, rect)`（Android 实例化真实 `android.view.View`）；Hybrid（抠洞）/Texture（`SurfaceTexture`→texture 节点）两种合成。
- [ ] **P3 — 互相嵌套**：嵌套组合 + 坐标/事件对账（最难点）。
- [ ] 架构门 `bash scripts/check-architecture-deps.sh` 仍 PASS（L4 不依赖更高层；绑定胶水在 L7+）。
- [ ] **设计约束**：L4 保持 `RawWindowHandle` 句柄化、不引 winit；胶水层（Kotlin/JNI/Compose）在 L7+。


## 8. 与已交付工作的衔接

本估算**不重复**以下已完成项（已在 `floom-gesture` / `floom-platform-mobile` 落地、测试全绿、架构门 PASS）：

- 触摸手势分类状态机（`TouchRecognizer` + `to_floom()` 适配器，L5）。
- NDK 键码映射表（`android_table.rs`，23 测试）。
- 软键盘高度迟滞滤波（`KeyboardHeightFilter`）+ `safe_area_insets` / `SystemChromeStyle` 类型。
- 8 个设备能力包的**类型与 host no-op**（原生实现待本估算的 §4 接线）。

即：UI 逻辑层已就绪，**本估算专攻 OS 胶水层**。

**本估算自身已交付（编译级）**：

- `crates/floom-platform-android`：接入 lapce/winit fork 的真实 `Platform` 实现（`AndroidPlatform` / `AndroidWindow` / `AndroidDriver` / `run` / `set_android_app`），key/modifier 映射在 L4 本地实现（不依赖 winit）；host 与 `aarch64-linux-android` 双零警告，架构门 PASS。
- `examples/floom2-android`：把 L4 后端 + `AppRunner<VelloGpuPresenter>` 接成可被 `native-activity` 加载的 `cdylib`（`android_main`），含 APK 模板（`AndroidManifest.xml` / `scripts/build-apk.sh`）。
- 上方两项均在 `docs/2.0/EXAMPLES.md` 登记，由 `crates/floom2/tests/docs_parity.rs` 与 `examples/` 目录、架构门 `LAYERS` 对账。

设备级余项（首帧 / tap / JNI 回值）见 §7.2，被 AVD 镜像门控。

---

## 9. 参考来源

- Slint 后端矩阵与 android-activity 后端：`docs.slint.dev` / `deepwiki.com/slint-ui/slint/9.3-mobile-and-web-platforms`、`5.2-winit-backend`。
- Flutter Engine/Embedder 分层与 Impeller：`deepwiki.com/flutter/flutter/4.4-engine-shell-embedder-api-and-dart-ui`；渲染器 `FlutterRendererType{kOpenGL,kSoftware,kMetal,kVulkan}`。
- GPUI Mobile（完整 `Platform` trait + wgpu + NDK/UIKit）：`crates.io/crates/gpui-mobile`、`github.com/itsbalamurali/gpui-mobile`。
- Zdroid（Zed on Android，Vulkan via wgpu，AChoreographer vsync，无每帧 JNI hop）：`github.com/Dylanmurzello/zed-android-port`。
- Flutter for HarmonyOS（OHOS Embedder 模型：XComponent+OHNativeWindow+VK_OHOS_surface+Vulkan，NAPI MethodChannel 桥）：`openharmonycrossplatform.csdn.net` 架构文档、`gitcode.com/CPF-Flutter/flutter_samples/ohos/docs/02_architecture`。
- React Native for OpenHarmony / RNOH（Fabric→ArkUI C-API 原生控件渲染 + JSI/NAPI 桥 + TurboModule）：`github.com/linhandev/ohos_react_native`、`deepwiki.com/linhandev/ohos_react_native`。
- **`ohos-rs/gpui-template` + `ohos-rs/zed`（`gpui_ohos`）—— 与 floom 同技术栈（Rust UI + wgpu）的鸿蒙实测参考（最关键）**：`github.com/ohos-rs/gpui-template`（最小可用 GPUI+OHOS 模板，含 `ohos/` ArkTS 工程、`EntryAbility.ets`、`module.json5`、HAP 构建）；`gpui_ohos` crate（`crates/gpui_ohos/src/ohos/{window,platform,wgpu_context,dispatcher,display,keyboard}.rs`）证明：① 复用 `openharmony-ability` NAPI 桥；② 默认 `wgpu::Backends::GL`（GLES/EGL）经 `OHNativeWindow` 建 surface；③ `OpenHarmonyApp.native_window()`→`RawWindowHandle::Ohos`+`DisplayHandle::ohos()`；④ renderer 延迟到 `Event::SurfaceCreate` 建；⑤ `content_rect()` 给安全区。floom 的 `floom-platform-harmony` 应直接镜像该 crate，而非从零设计。
- OpenHarmony XComponent / NativeWindow / Vulkan Surface 开发指南（`OH_NativeXComponent_Callback`、`OHNativeWindow`、`VK_OHOS_surface`）：`developer.huawei.com/consumer/cn/doc/harmonyos-references/vulkan-guidelines`、`deepwiki.com/openharmony/interface_sdk_c/2.1.6-xcomponent-and-native-rendering`。
- floom 内部：`crates/floom-platform/src/{lib,window,surface,capabilities,event,keymap}.rs`、`crates/floom-platform-desktop/src/{lib,winit_backend,winit_input,native_appkit}.rs`、`crates/floom-renderer-vello/Cargo.toml`、`docs/2.0/ARCHITECTURE.md`、`.superpowers/sdd/progress-floom-2.md`（L153-154）、本机工具链实测（2026-09-19）。
- Flutter 嵌入既有 Android 应用 / `FlutterView`（`SurfaceView`/`TextureView` 两 RenderMode、`attachToFlutterEngine`）：`docs.flutter.dev/javadoc/io/flutter/embedding/android/FlutterView.html`、`github.com/flutter/flutter/wiki/Experimental-Add-Flutter-View`（add-to-app 的 `FlutterView` 集成）。
- Flutter Platform Views（Hybrid Composition / TLHC / HCPP、native View 嵌入 Flutter）：`docs.flutter.dev/platform-integration/android/platform-views`、`flutter.googlesource.com/mirrors/flutter/+/.../docs/platforms/android/Android-Platform-Views.md`。
- Flutter add-to-app 三种承载（`FlutterActivity`/`FlutterFragment`/`FlutterView`）：`docs.shorebird.dev/flutter-concepts/add-to-app`。

---

## 附录：能力对照矩阵

| 能力 | floom 现状 | Slint | Flutter | GPUI Mobile | floom 缺口动作 |
|---|---|---|---|---|---|
| 窗口/事件循环 | 仅桌面 winit | winit（含移动） | 各平台 Embedder | 自写 NDK/UIKit | 移动端实现 `Platform`+事件循环 |
| GPU 渲染 | vello/wgpu（桌面） | Skia（Vk/GLES/Metal） | Impeller（Vk/Metal/GLES） | wgpu（Metal/Vk/GL） | 移动端从原生句柄建 wgpu surface |
| 触摸 tap/scroll | `floom-gesture` ✅ | winit 事件 | engine 手势 | 自写状态机 ✅ | 仅接线 |
| 键码映射 | NDK 表 ✅ | winit | 引擎 | NDK 表 ✅ | 仅接线（iOS 另映射） |
| IME/软键盘 | typed stub | JNI/objc 桥 | Embedder | 自写 | JNI/objc2 桥 |
| safe-area | typed stub | JNI 取 insets | Embedder | 自写 ✅ | JNI/objc2 取 insets |
| 系统栏外观 | typed stub | — | Embedder | — | JNI/objc2 设着色 |
| 剪贴板 | trait 定义 | JNI/objc | Platform Channel | 自写 | JNI/objc2 桥 |
| 文本/emoji | floom 文本 | 各平台 | engine | cosmic-text+CBDT | 移动端字体/回退硬化 |
| OHOS 后端 | headless stub | 社区(`flutter_openharmony`) | `flutter_openharmony`(成熟) | 无（但 `gpui_ohos` 证 wgpu-GL 可行） | **复用 `openharmony-ability` NAPI 桥 + wgpu `Backends::GL`(GLES/EGL)；`raw_window_handle::Ohos` 已被 wgpu 支持，`SurfaceTarget` 零改造；无 winit、无 shim** |
