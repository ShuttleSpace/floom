# 交接清单 — 剩余工作与怎么测

生成于 2026-10-23。A/B 两组已全部完成（见 `docs/plugin-and-renderer-audit.md`）。
这份清单只列**还没做的**，每项写清四件事：目标、为什么卡住、需要你提供什么、怎么验证。

---

## 0. 先说一个调查中发现的新缺口

写这份清单时查了 9 个能力插件的实现体，结果比原审计表记的严重：

| 插件 | Android | iOS | 真实度 |
|---|---|---|---|
| `floom-plugin-vibration` | 真 JNI（`VibrationEffect.createOneShot`） | 真 `AudioServicesPlaySystemSound` | ✅ **真实现** |
| 其余 8 个（battery / connectivity / device-info / local-auth / location / notifications / sensors / share） | **TODO stub**，返回默认值 | **TODO stub** | ❌ |

`floom-plugin-battery/src/android.rs` 长这样：

```rust
pub fn battery_state() -> BatteryState {
    // TODO: register a `BatteryManager` receiver and read
    // `EXTRA_LEVEL` / `EXTRA_SCALE` ...
    BatteryState::default()
}
```

A7 把能力从平台层拆成独立插件，**结构上成立了**（编译隔离、按需链接、门禁能查），
但**功能上是空壳**。审计表当时只记了"拆分完成"，没记"实现是 stub"——
这是我漏掉的，现在补上。

**这意味着**：`examples/floom-capabilities` 跑起来一切都"正常"，只是因为每个调用
都返回默认值。必须真机才能发现。

---

## 1. 八 stub 插件补真实实现 🟠 需要设备

**目标**：8 个插件的 Android + iOS 两侧都返回真实数据，不再是默认值。

**为什么卡住**：这些是平台 API 调用，编译通过不代表能用。JNI 签名写错、
权限没声明、Objective-C 消息发错——全都只在运行时现形。

**需要你提供**：一台 Android 手机（`adb` 现在能连上，见第 5 节环境检查）。
iOS 侧如果你没有 Mac + 真机，可以先只做 Android，iOS 留 TODO 并写明原因。

**怎么验证**（这些命令我已在本机验证过，APK 真的能产出）：

```bash
# 0. 先在 host 上看一眼基线（不需要设备）——注意每一行都是 `—` 或 `!`
cargo run -p floom-capabilities
```

host 上会看到：

```text
- device_info            —
- battery                —
...
! clipboard (read)       clipboard is only available on iOS and Android
...
0 of 12 capabilities answered on this host
```

**这不是失败，是正确行为**——而这个 example 里有一个测试专门断言它
（`a_host_build_never_reports_a_value_it_cannot_have_measured`）。
**这正是 8 个 stub 一直没被发现的原因**：host 上断言的就是"不报告值"，
所以"平台没有"和"实现了但返回默认值"在这个测试眼里完全一样。

```bash
# 1. 插上手机，确认在线
adb devices                 # 必须看到 <serial>  device

# 2. 打 APK 并安装（已验证可产出，绝对路径是必须的：aapt2 在别处解析相对路径）
scripts/build.sh floom-capabilities android \
  --manifest $PWD/examples/floom-android/AndroidManifest.xml --install
# 最后一行应是：
# TIER=artifact PLATFORM=android CRATE=floom-capabilities ARTIFACT=...apk
```

装完在手机上打开，表格应该**填满**。逐项对照：

| 能力 | stub 的表现 | 修好后应该看到 |
|---|---|---|
| battery | `100%` / `charging: false`，永不变化 | 真实电量，插充电器后 `charging` 变 `true` |
| connectivity | 永远同一类型 | 飞行模式切换时类型跟着变 |
| device-info | 固定型号/版本 | 与「设置 → 关于手机」一致 |
| sensors | 全 0 或不更新 | 摇动手机，加速度数值变化 |
| location | `Err(...)` 或固定坐标 | 授权后真实坐标，精度随 GPS 变化 |
| notifications | 点了没反应 | 通知栏真的出现一条 |
| share | 点了没反应 | 弹出系统分享面板 |
| local-auth | 直接成功/直接失败 | 弹出指纹/人脸识别 |
| vibration | （这个已是真实现） | **应该真的震**——它是判定基准 |

`vibration` 那一行是**判定基准**：它本来就是真 JNI 实现。如果它震了而别的没反应，
就确认了 stub 诊断；如果连它都不震，说明链路（打包/安装/权限）有问题，先修那个。

**反馈方式**：把手机上表格的内容原样贴回来（截图或逐行抄），
特别是 **"哪个点了有反应、哪个没反应"**。这决定还要不要继续投入。

屏幕上应该看到（取决于 app 里放了什么）：

| 能力 | stub 的表现 | 修好后应该看到 |
|---|---|---|
| battery | `100%`、`charging: false`，永不变化 | 你手机的真实电量，插充电器后 `charging` 变 `true` |
| connectivity | 永远 "wifi" 或永远 "unknown" | 飞行模式切换时类型跟着变 |
| device-info | 固定型号/版本 | 与「设置 → 关于手机」一致 |
| sensors | 全 0 或不更新 | 摇动手机，加速度数值变化 |
| location | `Err("only available on...")` 或固定坐标 | 授权后返回真实坐标，精度随 GPS 变化 |
| notifications | 点了没反应 | 通知栏真的出现一条 |
| share | 点了没反应 | 弹出系统分享面板 |
| local-auth | 直接成功/直接失败 | 弹出指纹/人脸识别 |

**反馈方式**：把每一项的"实际看到的内容"原样贴回来。特别是
**"哪个能力点了有反应、哪个没反应"**——这决定了还要不要继续投入。

**agent 这侧先做**：每个插件的 Android/iOS 实现补完 + 单测绿 + `cargo check`
对 `aarch64-linux-android` 和 `aarch64-apple-ios` 都过，再把 APK 交给你。
（现在可以直接开始，不等设备。）

---

## 2. camera / webview 真能力插件 🟠 需要设备

**目标**：A6 验证了原生视图合成链路（Objective-C 插件 → C ABI → 真 `NSView`），
但用的是演示视图。camera 和 webview 才是这条链路真正要服务的能力。

**为什么卡住**：camera 要真机 + 摄像头权限；webview 要能加载真实 URL 并观察
JS 桥。两者都无法在 CI 里验。

**怎么验证**：

```bash
# macOS 侧先跑通的链路（这个现在就能跑，不需要设备）
cargo run -p floom-plugin-nativeview --example native_view
```

应该看到：真实 `NSWindow` 里出现一个由 Objective-C 插件创建的真实 `NSView`，
框架经 C ABI 完成 bind / place / destroy 全流程。窗口不显示，只看组合结果。

**camera 的验收**：app 里一个矩形区域显示摄像头实时画面，退到后台再回来画面恢复。
**webview 的验收**：加载一个页面，页面里的 JS 能调到 Rust 侧一个函数并拿到返回值。

---

## 3. A6b provider 注册表（按 kind 选 provider）🟡 不需要设备

**目标**：现在宿主在 `WidgetHost::set_native_provider` 手动安装单个 provider。
需要的是**注册表**：宿主说"我要一个 camera 视图"，框架按 `NativeViewKind`
选出对应的 provider。

**为什么卡住**：纯设计决策 + 实现，不卡设备。我之前没做是因为它在等一个真用例——
没有 camera/webview 插件时，注册表只有一个条目，做了也验证不了价值。

**需要你决策**：注册表的查找键是什么？
- `NativeViewKind` 枚举（编译期封闭，加能力要改 scene crate）
- 字符串 kind（插件可自注册，但失去类型安全）

我倾向后者 + 一个 `NativeViewKind::Custom(String)` 兜底，理由是插件本来就要能独立于
框架发布。但这是你的调用。

---

## 4. C4 — OPEN_URL / FILE_DIALOG 重新引入 🟠 需要决策

**目标**：A8 把这两个 capability 位**删掉了**（没有实现的位不该存在）。
现在要么补实现，要么明确说"我们不支持"。

**为什么卡住**：产品决策。这两个是常用能力，删掉会影响宿主迁移；
但加回来就要有真实实现，否则就是退回 A8 修掉的问题。

**需要你决策**：
- (a) 补实现（作为能力插件，`floom-plugin-open-url` / `floom-plugin-file-dialog`），
  桌面端用 `open` crate / `rfd`，移动端用 intent / `UIDocumentPicker`
- (b) 明确不支持，在文档里写明"宿主自己调平台 API"
- (c) 只在桌面端做，移动端留空

**注意**：如果选 (a)，做完必须真的能打开一个 URL / 真的弹出文件选择器——
这两件事在 macOS 上我能验（有桌面环境），Android 需要设备。

---

### 5. 环境检查（已验证）

```bash
# Android 工具链——已确认齐备
echo $ANDROID_HOME          # /Users/humphrey/Library/Android/sdk ✓
echo $ANDROID_NDK_HOME      # .../ndk/27.1.12297006 ✓
which cargo-ndk adb         # 都有 ✓
```

**打包已验证通过**（不需要设备，我刚刚跑过）：

```bash
scripts/build.sh floom-capabilities android \
  --manifest $PWD/examples/floom-android/AndroidManifest.xml
# → TIER=artifact PLATFORM=android CRATE=floom-capabilities ARTIFACT=...apk ✓
```

所以**环境不是阻塞项，只差一台设备**：

```bash
adb devices                 # 当前输出空列表
```

插上手机（USB 调试已开）后应出现 `<serial>  device`。
若显示 `unauthorized`，在手机上确认弹窗；若为空，换线/换口/开 USB 调试。

---

## 6. 不需要设备的（我可以直接做）

如果你不想碰设备，这几项现在就能推进，都有 CI 保护：

- **B7 的 vello GPU 帧缓冲测试**：给 vello 加 framebuffer 读回对比。需要 GPU，
  但**你的 M4 Pro 有**——我可以写一个 `#[ignore]` 的测试，你本地
  `cargo test --features gpu -- --ignored` 跑一次，CI 里跳过。
- **A6b 注册表**（等你拍板第 3 节的键）
- **8 个 stub 插件的实现代码**（编译 + 单测我能验，真机行为你验）

---

## 附：怎么跑测试（完整版，均已验证）

```bash
cd /Users/humphrey/Project/Rust/floem

# 全量测试（当前 100% 绿）
cargo test --workspace

# 架构门禁（平台→插件禁止边、层级约束）
./scripts/check-architecture-deps.sh

# 单个后端
cargo test -p floom-renderer-vello --features gpu --lib   # 47 项 ✓
cargo test -p floom-renderer --test degradation           # 12 项 ✓
cargo test -p floom-renderer-web --test web_pixels        # 6 项 ✓
cargo test -p floom-renderer-tui --test tui_pixels        # 5 项 ✓

# 能力 demo，host 基线（不需要设备）✓ 已验证
cargo run -p floom-capabilities

# 原生视图插件（macOS，不需要设备）✓ 已验证跑通
cargo run -p floom-plugin-nativeview --example native_view
# 输出：window: <NSWindow: 0x...> / bound in mode Overlay / placed at 10,15 100x50
#       the native half did all of it: one view created, placed, and destroyed.

# Android APK（不需要设备也能出包）✓ 已验证 TIER=artifact
scripts/build.sh floom-capabilities android \
  --manifest $PWD/examples/floom-android/AndroidManifest.xml --install

# golden 更新（改了渲染行为后）
UPDATE_SNAPSHOTS=1 cargo test -p floom-renderer-web --test web_pixels

# 知识图谱（改完代码后）
graphify update .
```

### ⚠️ 已知破损：`floom-probe` 编译不过（新发现）

```bash
cargo run -p floom-probe --features real
# error[E0432]: unresolved imports floom_widgets::BottomSheet, Dialog,
#   NavDestination, NavigationDrawer, Popup, Select
```

**原因**：这些 widget 在 `ecosystem/floom-ui`（**另一个 workspace**）的
`floom-ui-composites` 里，主 workspace 的 `floom-widgets` 没有它们。
`floom-probe` 在主 workspace，引用不到。

**为什么一直没被发现**：它的 `[[bin]]` 有 `required-features = ["real"]`，
而 `real` 不在 default features 里——所以 `cargo test --workspace`
（不带 `--features real`）**静默跳过了它**。feature 门把破损掩住了。

**这是第 2 个"测试全绿但东西是坏的"案例**，和第 1 个（8 个 stub 插件）同一个模式：
**验证的边界没覆盖到出问题的地方**。

**修法**（二选一，需要你拍板）：
- (a) 把 probe 依赖改成 ecosystem workspace 的对应 crate——但那是跨 workspace 依赖，
      可能不现实；
- (b) 把 probe 移进 ecosystem workspace，或删掉它，在文档里注明设备探针已由
      其他方式覆盖；
- (c) 给 CI 加一条 `cargo check -p floom-probe --features real`，
      先让它**红着**暴露出来，再决定修法。

---

## 优先级建议

1. **第 1 节（8 个 stub 插件）** —— 影响最大，且你现在就有 Android 工具链，
   只差一台设备。实现代码我可以先写。
2. **第 4 节（C4 决策）** —— 一次拍板，阻塞别人。
3. **第 3 节（A6b）** —— 等第 2 节有了第二个插件才有意义。
4. **第 2 节（camera/webview）** —— 最重，放最后。
