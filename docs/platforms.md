<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# 平台与能力绑定

两件事必须分开看：**能编译**（`cargo build --target`）与**能跑**（有输入、有人在上机按过）。
[release-policy.md](release-policy.md) 的 targets 表是状态的**权威源**；本文件回答的是"每个平台的能力到底从哪条路进来"。

## 1. Target 矩阵（摘要，权威见 release-policy）

| target | 后端 | 状态 | 门禁账本 |
| --- | --- | --- | --- |
| `aarch64-apple-darwin` | `floom-platform-desktop`（winit）+ `floom-renderer-vello`（Metal） | **tested**（macOS 26.6.2 / M4 Pro / rustc 1.98.0） | [gates/macos.md](gates/macos.md) 8/8 pass |
| `x86_64-apple-darwin` | 同上 | **untested**（无 Intel 机器） | — |
| `x86_64-pc-windows-msvc` | `floom-platform-desktop` | device-gated（Phase 16.2），只到 `cargo build` | [gates/windows.md](gates/windows.md) 8 项全 gated |
| `x86_64-unknown-linux-gnu` | `floom-platform-desktop` | device-gated（Phase 16.2），只到 `cargo build` | [gates/linux.md](gates/linux.md) 8 项全 gated |
| `aarch64-linux-android` | `floom-platform-android`（JNI + winit-android） | device-gated（Phase 17）：启停/IME/CJK/剪贴板已在模拟器验过 | [gates/android.md](gates/android.md) 4 pass / 1 partial |
| `aarch64-apple-ios` | `floom-platform-ios`（UIKit staticlib） | 模拟器实测（2026-11-21）：宿主从占位工程换成真 Swift 宿主 + `swiftc` 链接，`floom package --platform ios` 产出可安装的 `.app`，floom 像素上屏；a11y 元素带非零 frame（`idb ui describe-all` 实测）；CJK 未过 | [gates/ios.md](gates/ios.md) 3 pass / 1 partial / 1 fail |
| OpenHarmony（手机/平板/PC 统一 OS） | `floom-platform-harmony`（N-API cdylib + 软件渲染） | device-gated（Phase 19）：启动 + 点按 pass，IME/a11y/剪贴板/文件/安全区/焦点未做 | [gates/ohos.md](gates/ohos.md) 2 pass |
| `wasm32-unknown-unknown` | `floom-platform-web` + `floom-renderer-web` | target-gated（Phase 19），headless WASM harness 未做 | — |
| no-std / bare metal | `floom-platform`（`embedded` feature） | target-gated（Phase 19） | — |

> 陈旧处：release-policy 里把 iOS/HarmonyOS 写成 `floom-platform` 的 feature，实际早已是独立 crate（`crates/platform/floom-platform-ios`、`-harmony`）。gate 文档里的 `crates/floom/src/...` 路径同样是旧的，实际是 `crates/app/floom/src/...`。

## 2. 能力绑定真相表（这张表才是"接平台"的清单）

| 能力 | macOS | Windows/Linux | Android | iOS | OHOS | Web/TUI |
| --- | --- | --- | --- | --- | --- | --- |
| 上屏 | winit + Metal/vello | winit + vello | Vulkan（模拟器经验证 `GPU adapter: Apple M4 Pro (Vulkan)`） | `floom_ios_bind_drawable` 像素指针 + Metal surface | `floom_harmony_bind_surface` + `present_frame`（软件 blit） | canvas blit / 字符网格 |
| 指针/触摸 | winit | winit | winit-android → ui-events | UIKit 侧驱动 → `floom_ios_app_touch`（设备像素，Rust 除以 scale 进逻辑点；`idb ui tap` 实测能改状态） | **无**（见 §4） | winit-web / TUI 键鼠 |
| 键盘 | winit | winit | winit-android keycodes（`Keycode::Tab → Key::Named(Tab)`，`winit-android/src/keycodes.rs:312`） | UIKit | **无** | 有 |
| IME | winit IME + `crates/platform/floom-platform-desktop/src/ime/` | 同左（Win32 组合桥未完成） | 锚点 `EditText.Editable` + `BaseInputConnection` 组合区 → `ImeCommit`/`ImePreedit` | 隐藏 `UITextField` → `floom_ios_push_ime_input`（拉丁 pass，CJK 提交成功） | **无** | 部分 |
| 剪贴板 | `floom-platform-desktop/src/clipboard.rs`：macOS `NSPasteboard`、Windows `CF_UNICODETEXT`（两者都有往返测试，Windows 在 CI runner 上真跑过含 CJK）、Linux 走 `wl-copy`/`xclip`/`xsel`（只有解析逻辑有测试，赛道里没有可用的会话） | JNI ClipboardService（双向 pass，需先获得焦点） | objc2 `UIPasteboard`（双向 pass，含系统粘贴同意） | **无** | 无 |
| 无障碍 | 语义树在进程内，**无 NSAccessibility 桥** | 无 AT-SPI / 无 UIA | `CapabilityLimitedAccessibilityAdapter`，`NATIVE_BRIDGE` 故意不置位 | 无原生适配器（frame 仍为 0） | 无 | 无 |
| 文件选择 | **无 trait**（平台层还没这个抽象） | 同左 | SAF 代码完整，结果回传需要 `FloomNativeActivity` 子类 | `ios_file.rs`/`ios_receive.rs` 有，example 未接 | 无 | — |
| 安全区 | n/a | n/a | 无公开 `WindowHandle::safe_area()` | **已接**（2026-11-22）：UIKit 的 `safeAreaInsets` 经 `floom_ios_set_safe_area_insets` 进 Rust，宿主每帧 `WidgetHost::set_content_insets` 作为根内边距；模拟器实测内容从 y=0 移到 y=62 | 无 | — |
| 声明能力位 | WINDOWING/TOUCH/IME/CLIPBOARD/A11Y 视 feature | 同左 | `android_default()` = WINDOWING\|TOUCH\|IME\|SOFTWARE_RENDER\|**WEBGPU** | `ios_default()` = WINDOWING\|TOUCH\|NATIVE_VIEW\|SOFTWARE_RENDER | `harmony_default()` = WINDOWING\|TOUCH\|NATIVE_VIEW\|SOFTWARE_RENDER | 各自 |

能力位在 `crates/platform/floom-platform/src/capabilities.rs`。**规则：只声明已有通路的能力。** 当前违反的项在 §4/§5。

## 3. 三套 ABI 与各自的导出面

1. **widget/renderer ABI**：纯 Rust trait（`floom-renderer` 的提交路径、`floom-platform` 的 windowing）。
2. **native view / 插件 provider ABI**：C vtable `FloomNativeProviderV1`，注册函数 `floom_<name>_register_provider(provider: *const FloomNativeProviderV1)`。见 [plugins.md](plugins.md)。
3. **平台宿主 ABI**（宿主拥有窗口，Rust 只画）：
   - iOS：`floom_ios_set_safe_area_insets`、`floom_ios_set_keyboard_state`、`floom_ios_push_ime_input`、`floom_ios_bind_drawable`、`floom_ios_present_frame`、`floom_ios_release_drawable`。
   - OHOS：`floom_harmony_bind_surface`、`floom_harmony_present_frame`、`floom_harmony_release_surface`。**只有三个。**
   - Android：无 `#[no_mangle]`，全部走 JNI（`Java_dev_floom_*` 入口 + `AndroidJni::with_env`）。

## 4. OHOS：已定位缺陷（按严重度）

1. **没有输入入口**。`floom-platform-harmony` 只导出上屏三件套，没有 touch/key/IME/剪贴板/安全区导出；ArkTS 侧无处可送事件。而 `harmony_default()` 声明了 `TOUCH`。
2. **有输入能力的那一层被弃用**：`crates/vendor/floom-winit-ohos/`（3,049 行，含 touch `.notify_touch`、mouse `.notify_mouse`、键盘、`request_ime_update`）**没有任何 crate 依赖它**，只有 workspace 声明（`Cargo.toml:184`）与一句注释。当前在用的是无输入的 N-API 上屏层。
3. ~~**缓冲队列泄漏**~~：`RequestBuffer` 之后的所有早退路径（handle 空 / `size<=0` / `fd<0` / `mmap` 失败 / 拷贝失败）现在都 `OH_NativeWindow_NativeWindowAbortBuffer` 归还缓冲；只有成功拷贝才 `Flush`。
4. ~~**脏帧上屏**~~：拷贝失败改为 abort，不再先把半帧 `Flush` 给合成器。
5. ~~**状态非成组**~~：三个 atomic 换成一个 `static Mutex<Option<Bound>>`（window 地址 + 宽高同一把锁），且锁**跨整个 blit 持有** —— `release_surface` 会等正在进行的帧画完，window 不可能在帧中途被释放。
6. ~~**尺寸必须精确相等**~~：`present_frame` 改用**帧自己的**宽高（每次 present 都 `SET_BUFFER_GEOMETRY`），bind 只剩"有没有窗口"这一个作用。旋转/分屏竞态不再丢帧。
7. ~~**`stride` 单位可疑**~~：`stride_bytes(reported, width)` 先按字节读；小于一行时按像素读（×4）；仍不够则退回行宽，不再把帧拒掉。语义有单元测试，**设备上仍是待验项**（真正读数只有真机才给）。

**决定（2026-11-21）**：走 `floom-winit-ohos`（A 方案）。理由是"谁拥有事件循环"这个选择只能有一个答案——ArkTS 驱动 vsync 时输入要从 ArkUI 手工转进 Rust，每加一个事件类型就要在 C++ 与 Rust 两侧各写一遍；把循环交给 Rust 后，surface/触控/键盘都由 winit 抽象统一，并且 wgpu 能拿到 `native_window` 直接上屏（当前 CPU `mmap` blit 的替代路径）。代价是要写 ArkTS/C++ 胶水，这部分只在设备上验证。

落地形态（`crates/app/floom/src/platform/ohos_winit.rs`，feature `harmony,harmony-winit`）：

- 应用入口唯一：`floom::ohos_app!(build)` 导出 `ohos_winit_runtime_*`（surface 生命周期、touch、mouse、key、frame、focus、visibility），C++ 桥先调 `ohos_winit_runtime_new()` 拿运行时指针，其余事件原样转发。
- 树在 worker 线程上构建（floom 的树从诞生起就持有 `Rc`，只能生在它要活着的线程），因此工厂是闭包 + 一次性 `Handoff`，不是 `fn` 指针。
- 帧循环由 Rust 自己决定：`about_to_wait` 只在 `platform.wants_frame()` 为真时 `request_redraw()`，静态界面零帧。
- `floom::run` 在这条路径上**返回错误**（事件循环归 ArkUI 所有，从这里再起一个没人泵的循环只会画出黑窗），错误信息直接指向 `ohos_app!`。
- 上屏仍走 `floom_platform_harmony::present_into_window` → `native::present`（`OH_NativeWindow_*` + `mmap`），§4 的 5 个上屏缺陷（第 3–7 条）因此尚未修，它们是 `S1.2` 的内容。

旧路径 `floom-platform-harmony` 的 N-API 三件套保留（`floom_harmony_bind_surface`/`present_frame`/`release_surface`），`tests/floom-ohos-demo` 的 C++ 桥目前仍调它们；切桥是 `S1.1b`，届时旧三件套标 deprecated。

## 5. iOS：已定位缺陷

1. ~~**CJK 渲染成豆腐**（`gates/ios.md` item 2 partial）：`parley`/`fontique` 在 iOS 找不到汉字字体，首帧 `你好世界` 即 tofu。~~
   **已修（2026-11-21，待真机）**：根因不是"iOS 没有汉字字体"，而是 fontique 的 Apple 后端只扫
   `NSSearchPathForDirectoriesInDomains(LibraryDirectory)`（`/Library/Fonts` + `~/Library/Fonts`），
   **从不扫 `/System/Library/Fonts`** —— 而 iPhone/iPad 的汉字覆盖（`PingFang.ttc`、`Songti.ttc`）全在后者。
   macOS 上这个洞被"开发者机器 /Library/Fonts 里常有一个 CJK 字体"掩盖了（本机是 `Arial Unicode.ttf`，
   实测 `PingFang SC` 在 bare `FontContext` 里查不到，`Arial Unicode MS` 能查到）。
   修法：`crates/widgets/floom-text/src/system_fonts.rs` 扫那两个系统目录，按文件名前缀挑 **一个** CJK 面注册进
   `FontContext`；`TextShaper::new()` 与编辑器线程的 `FONT_CONTEXT` 初始化时各调一次（`thread_local!`
   的初始化表达式里）。**只注册一个**字族（`PingFang` 优先，其余按 `CJK_STEMS` 顺序取第一个存在的），
   **且只在非 macOS 的 Apple 平台做**：注册一份字体就是把它的字节拷进内存（`Blob`），而系统里
   `Songti.ttc` 单个文件 67 MB —— 第一版把整个 CJK 清单都读进来，macOS 的 `TextShaper::new()`
   因此吃到 242 MiB（见 `docs/performance.md` 的回归记录），现在是 9.1 MiB。
   单测：`the_system_directory_holds_a_cjk_face_the_backend_cannot_see`（该文件确实在、注册后 family 数增加）、
   `only_the_platforms_without_system_fallback_register_a_face`（macOS 一个字节都不读）、
   `han_text_shapes_to_real_glyphs`（`你好世界` 无 `.notdef`）、`registering_is_safe_on_every_target`。
   真机门槛仍是 `gates/ios.md` item 2。
2. **a11y 零 frame**（item 3 fail）：`idb ui describe-all` 有 label，但每个节点 frame 为 0，VoiceOver 无法命中。根因有两层，第一层已修（2026-11-21）：`SemanticsNode` 原本**没有几何字段**，适配器拿到节点也不知道它在哪——现在带 `bounds`（由布局填，不是 widget 自报），并且 `Platform::accessibility` 这道门把每帧的 `SemanticsDiff` 交给适配器（`crates/app/floom-app/src/runner.rs` 的 `deliver_semantics`）。第二层已修（2026-11-21）：`crates/platform/floom-platform-ios/src/accessibility.rs` 把语义树展平成 `#[repr(C)] A11yNode` 记录（label / 角色码 / 点在容器坐标系的 frame / enabled / selected / focused，按 y 再按 x 排序 = 阅读顺序），经 `floom_ios_set_a11y_sink` 交给 Swift 宿主；宿主把它们变成 `UIAccessibilityElement` 并赋给 `view.accessibilityElements`。模拟器实测（`idb ui describe-all`）：`StaticText | Count: 0 | {0,0,402,14}`、`Button | Increment | {0,22,78,22}` —— frame 不再是 0。
动作回传（2026-11-22）：`WidgetHost::activate(ElementId)` 把激活当作**元素中心的点击**派发——屏幕阅读器要够到的就是那个 click handler，另开一条"无障碍激活"通路只会让两者说法不一致。`UIAccessibilityElement` 的子类带着下标回调 `floom_ios_a11y_activate(index)`，适配器把下标换成 `ElementId` 再交给 `ios_host` 注册的处理器（它才拿得到 widget 树）。能力位现在含 `ACTION_INVOKE`；`SetValue`/`ScrollIntoView` 仍如实报 `false`。
**证据分级**（按"CI 有效性"规则）：Rust 两条腿有测试（`floom-app` 的 `activating_a_semantics_node_runs_what_a_click_would` 证明回调真的跑了一次；`floom-platform-ios` 的 `the_host_activates_the_node_behind_the_index_it_was_given` 证明下标→id→处理器）；UIKit 那一腿（`accessibilityActivate`）**没有机器验证**——`idb` 只能注入触摸，不能触发 VoiceOver 的双击，所以这一条要么在真机上手动验，要么等 VoiceOver 自动化。
3. 能力位的 `TOUCH` 由 UIKit 侧兑现：`floom_ios_app_touch` 收设备像素、除以 `displayScale` 后进逻辑点空间（`crates/app/floom/src/platform.rs` 的 `ios_host::touch`）。漏掉这次除法时命中测试全部落空——这是首次上机发现的问题。
4. **safe-area 曾经只是"读得到、没人用"**（2026-11-22 修）：`floom-platform-ios` 的 `safe_area_insets()` 一直在，但全仓没有消费方，于是内容画到状态栏下面。现在 `WidgetHost::set_content_insets(Insets)` 把它当作每趟布局的根内边距（每个方向取 `max`，widget 自己的 padding 是更高的下限），宿主在 `start` 与每帧 `pump` 各同步一次。单测 `crates/app/floom-app/tests/insets.rs`（2 个）。与它一起修的两个连带缺陷：布局移动后已发布的语义 bounds 不会刷新（`WidgetHost::semantics_moved` 现在会触发重建），以及事件处理器拿不到 host —— 新增 `WidgetHost::on_frame`（`crates/app/floom-app/tests/frame_callbacks.rs`），`floom create` 生成的示例因此能真的显示点击次数。

## 6. Android：已定位缺陷与教训

- a11y：`crates/platform/floom-platform-android/src/accessibility.rs` 是 `CapabilityLimitedAccessibilityAdapter`，`perform_action` 恒 `false`（能力位也如实不置 `NATIVE_BRIDGE`）。
- 文件选择：SAF 代码完整，结果回传需要 `FloomNativeActivity` 子类 + Gradle/AGP 集成，未做。
- **已修复但必须记住的坑**：吞掉 JNI `Err` 不 `exception_clear()` 是**延迟进程中止**——异常挂在线程上，几秒后任意一次无关 JNI 调用即 `No pending exception expected: java.lang.ClassNotFoundException: dev.floom.android.FloomAccessibilityView`。修复在 `AndroidJni::with_env` 出口清异常；`Java_dev_floom_*`（Java 调进来）故意保留异常给调用方。
- 声明了 `WEBGPU`：vello/wgpu 在 Android 上确实跑起来了，但这条与 OHOS 的 `SOFTWARE_RENDER` 不对称，写 capability 时要解释清楚原因。

## 7. 桌面（mac, Windows, Linux）

- 剪贴板只有 macOS 真实现；Linux/Windows 是**进程内占位**——复制粘贴出不了应用。这是三个桌面 target 里最容易踩的"看起来能用"。
- 无障碍：语义树保留、无 OS 桥（mac 无 NSAccessibility、Linux 无 AT-SPI、Windows 无 UIA）。
- 文件选择：**平台层根本没有 file-picker trait**，所以"每个桌面 target 都缺"不是 Linux/Windows 的特别问题。
- Windows/Linux 的 native-view 合成（Phase 16.2）未完成，`cargo test` 在 CI 上只跑到 build。

## 8. 门禁账本

[gates/](gates/) 每平台 8 项（启停与交互 / IME 拉丁+CJK / a11y label+Default action / 剪贴板双向 / 文件选择 / 安全区 / 焦点与键盘导航 / CI 编译），值只有 `pass`/`fail`/`n/a`，每行带逐行证据（logcat、hilog、`idb`、`simctl` 原始输出）。**tag 1.0 时不允许任何 `fail`**；`SKIP ≠ PASS`，跳过的必须打印原因。
