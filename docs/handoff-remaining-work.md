# 交接清单 — 剩余工作与怎么测

生成于 2026-10-23，最后更新 2026-10-24。A/B 两组已全部完成（见
`docs/plugin-and-renderer-audit.md`）。这份清单只列**还没做的**，每项写清四件事：
目标、为什么卡住、需要你提供什么、怎么验证。

---

## 0. 调查中发现的缺口（已修，留作记录）

写这份清单时查了 9 个能力插件的实现体，结果比原审计表记的严重：

| 插件 | Android | iOS | 当时真实度 |
|---|---|---|---|
| `floom-plugin-vibration` | 真 JNI（`VibrationEffect.createOneShot`） | 真 `AudioServicesPlaySystemSound` | ✅ **真实现** |
| 其余 8 个（battery / connectivity / device-info / local-auth / location / notifications / sensors / share） | **TODO stub**，返回默认值 | **TODO stub** | ❌ |

`floom-plugin-battery/src/android.rs` 当时长这样：

```rust
pub fn battery_state() -> BatteryState {
    // TODO: register a `BatteryManager` receiver and read
    // `EXTRA_LEVEL` / `EXTRA_SCALE` ...
    BatteryState::default()
}
```

A7 把能力从平台层拆成独立插件，**结构上成立了**（编译隔离、按需链接、门禁能查），
但**功能上是空壳**。审计表当时只记了"拆分完成"，没记"实现是 stub"。

**2026-10-24 已修**：八个插件的两侧全部落地真实现，详见第 1 节的落点表。

**这个缺口能潜伏的原因值得记住**：CI 只对插件跑 host 测试，而
`#[cfg(target_os = "android")]` / `#[cfg(target_os = "ios")]` 分支在 host 上
**根本不参与编译**。所以"stub 返回默认值"和"真实现"在 CI 眼里完全一样，
连"原生半边编译不过"也一样。现在 `.github/workflows/ci.yml` 加了
`ios-check` job 并扩展了 `android-check`——补上 CI 后立刻暴露了一个潜伏的真 bug：
`floom-plugin-vibration` 的 iOS 半边有两个 edition-2024 硬错误
（`extern blocks must be unsafe`），它此前从未对 iOS 编译过。

---

## 1. 八 stub 插件补真实实现 ✅ 代码完成 / 🟠 待设备确认

**目标**：8 个插件的 Android + iOS 两侧都返回真实数据，不再是默认值。

**现状（2026-10-24）**：**agent 侧已完成**。八个 crate 全部接上真实平台 API，
host / `aarch64-linux-android` / `aarch64-apple-ios` 三目标零错误，host 单测全绿。
CI 也补了两个 job（`android-check` 扩展 + 新增 `ios-check`），
因为在此之前这些 `cfg(target_os=...)` 分支**从未被 CI 编译过**——
`floom-plugin-vibration` 的 iOS 半边就带着两个 edition-2024 硬错误躺了很久。

**还没做的**：真机运行。下面那张表仍然有效，它测的是"编译通过 ≠ 能用"。

**每个调用的落点**（读代码时按这个对照，不要假设"都实现了"）：

| 调用 | Android | iOS |
|---|---|---|
| `battery_state` | 真读（`ACTION_BATTERY_CHANGED` 粘性广播） | 真读（`batteryLevel`/`batteryState`） |
| `is_low_power_mode` | 真读（`Settings.Global.battery_saver_mode`） | 真读（`isLowPowerModeEnabled`） |
| `connectivity_status` | 真读（`getActiveNetwork` + `getNetworkCapabilities`） | 真读（`SCNetworkReachability`） |
| `device_info` / `device_name` | 真读 | 真读 |
| `available_sensors` | 真读（`getDefaultSensor(type)`） | 真读（`CMMotionManager.*Available`），`Proximity`/`AmbientLight` 恒 false——iOS 无查询 API |
| `share_text` | 真发起（`ACTION_SEND`） | 真发起（`UIActivityViewController`） |
| `share_files` | **`Err`**：缺 `FileProvider`（`hasCode="false"` 的 NativeActivity 没有 androidx） | 真发起 |
| `are_notifications_enabled` | 真读（`areNotificationsEnabled()`） | 真读（信号量等 `getNotificationSettings`） |
| `request_authorization`（通知） | 真发起（`requestPermissions`） | 真发起（`requestAuthorizationWithOptions`） |
| `schedule`（即时） | 真发 | 真发 |
| `schedule`（延迟） | **`Err`**：`AlarmManager` 未接线 | 真发（`UNTimeIntervalNotificationTrigger`） |
| `available_biometric` | 真读（`KeyguardManager` + `FEATURE_FINGERPRINT`） | 真读（`canEvaluatePolicy` + `biometryType`） |
| `authenticate` | **`Err`**：结果在 `onActivityResult`，NativeActivity 不转发 | **`Err`**：reply block + 主线程弹窗会死锁 |
| `authorization_status` | 真读（`checkSelfPermission`） | 真读（`authorizationStatus`） |
| `is_location_enabled` | 真读（`isLocationEnabled` / `isProviderEnabled("gps")`） | 真读（`locationServicesEnabled`） |
| `request_authorization`（定位） | 真发起（`requestPermissions`） | 真发起（先查 `Info.plist` 的 usage description，缺了就 `Err`） |
| `current_location` | 真读（`getLastKnownLocation`，**缓存**定位 + 时间戳） | **`Err`**：委托回调在主 run loop，同步等待会死锁 |

**`Ok` 的语义已经统一写进每个函数文档**：`request_authorization` 返回 `Ok` 意思是
**"OS 已被请求"**，不是"用户已同意"；结果由调用方轮询状态读取函数。
`authenticate` 不在此列——它的 `Ok` 会声称"用户通过了认证"，那是同步签名无法诚实
表达的断言，所以两侧都保持 `Err` 并指名缺的是哪一步。

**怎么验证**（命令已在本机验证过，APK 真的能产出）：

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
注意它**只覆盖 host 侧**：它能证明"桌面不撒谎"，不能证明"手机上真能读"。
后者只有真机能证明，这就是第 1 节还没结束的原因。

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

| 能力 | stub 的表现 | 现在应该看到 |
|---|---|---|
| battery | `100%` / `charging: false`，永不变化 | 真实电量，插充电器后 `charging` 变 `true`；若系统不给读数则显示 `no reading`（`level: None`），不再伪装成 100% |
| connectivity | 永远同一类型 | 飞行模式切换时类型跟着变 |
| device-info | 固定型号/版本 | 与「设置 → 关于手机」一致 |
| sensors | 空列表 | 列出这颗设备真有的传感器种类 |
| location | `Err(...)` 或固定坐标 | 授权后真实坐标，精度随 GPS 变化 |
| notifications | 点了没反应 | 通知栏真的出现一条 |
| share | 点了没反应 | 弹出系统分享面板 |
| local-auth | 直接成功/直接失败 | 弹窗真的出现（然后如实报告结果读不到） |
| vibration | （这个一直是真实现） | **应该真的震**——它是判定基准 |

`vibration` 那一行是**判定基准**：它本来就是真 JNI 实现。如果它震了而别的没反应，
就确认了是插件实现的问题；如果连它都不震，说明链路（打包/安装/权限）有问题，
先修那个。

**反馈方式**：把手机上表格的内容原样贴回来（截图或逐行抄），
特别是 **"哪个点了有反应、哪个没反应"**，以及**任何一行显示 `!` 时那句错误原文**
——那些 `Err` 的措辞就是设计的一部分，措辞不对说明平台假设错了。

**iOS 侧**：需要 Mac + 真机（或模拟器，但 `battery`/`sensors` 在模拟器上会如实
报告"没有"，那是正确行为而不是 bug）。命令在附录。

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

## 3. A6b provider 注册表（按 kind 选 provider）✅ 已实现（2026-10-27）

**目标**：宿主说"我要一个 camera 视图"，框架按 `NativeViewKind` 选出对应的
provider。改之前宿主只能在 `WidgetHost::set_native_provider` 手动装**单个**
provider，没人管的 kind 就落到它身上。

**查找键**（`plugin-catalog.md` §8.2）：`NativeViewKind` 已经是
`Map | Video | Web | TextInput | Custom(&'static str)`，`floom-native-ffi` 已经
把它映射到 C ABI。具名变体管类型安全，`Custom` 管插件自注册，两者兼有。

**拍板的 shape：显式 kind + miss 即拒 + 无兜底默认。**
（原文列的"枚举 vs 字符串"是个假二选一；真正的选择是要不要留一个接受一切的
默认 provider，答案是不要。）

- **新增** `crates/floom-native/src/registry.rs`：`NativeProviderRegistry`。
- `register(provider, kinds)` 申报**确切**的 kind；一个 provider 可认领多个
  kind（OS host 拥有自己的视图），一个 kind 只能有一个 provider，冲突是注册期
  `Err(KindRegistered(kind))`。
- 没人认领的 kind 在 bind 时 `Err(NoProvider(kind))`，消息点名是哪个 kind。
- `WidgetHost::set_native_provider` **已删除**，改成
  `register_native_provider(provider, kinds) -> Result<(), NativeError>`。
- 生命周期仍是每个 provider 一个 `NativeViewController`，bound/frozen/mode
  不变量原样下放；焦点改为**注册表级独占**（focus 一个会先 blur 另一个
  provider 上拿焦点的那一个）。
- `reconcile_clipping` 改成**按 participant 的 kind** 取 capabilities，
  不再问一个全局 `capabilities()`。

**为什么"无兜底"是必要的**：desktop 桥的默认工厂是
`|_, mtm| NSView::new(mtm)`，对任何 kind 都造一个不画东西的 `NSView`。改之前
一个 `Custom("camera")` 请求会"成功"并拿到空白矩形 —— 正是 `WidgetHost` 文档
里点名的最坏结果（空白矩形与"从没被请求过"无法区分）。申报 kind 把那句警告
变成了结构。

**验证**：`floom-native` 66 个测试、`floom-app` 全部 target、
`floom-ui-composites` 全绿；`cargo check --workspace --all-targets` 干净；
clippy/rustfmt 干净。`floom-probe` 只申报 `Video`（它演示挂载的唯一 kind），
其余 kind 现在是拒绝而不是画空。

**留下的口子**：desktop 桥的 kind 申报还靠调用方传 `kinds`，不是由
`NativeViewHost` 实现自身推导（`MacNativeViewHost::with_factory` 的工厂没告诉
任何人它能造什么）。理想状态是 `create_view` 对它画不出来的 kind 返回错误，
让"能画哪些 kind"成为实现的性质。需要真实工厂存在（webview plugin），届时一并做。


---

## 4. C4 — OPEN_URL / FILE_DIALOG ✅ 已拍板（2026-10-25）

**目标**：A8 把这两个 capability 位**删掉了**（没有实现的位不该存在）。
现在要么补实现，要么明确说"我们不支持"。

**结论：实现补了（选项 a），位不加回来。**

- 实现：`floom-plugin-open-url`（A2）、`floom-plugin-file-dialog`（A3）两个能力插件，
  按依赖 opt-in；桌面上是五臂/三臂真实现（不是 `open` crate / `rfd`，理由见
  catalogue 的 as-built），手机上是精确拒绝。
- 位：`OPEN_URL` / `FILE_DIALOG` **保持删除**。理由与取舍写在
  `docs/plugin-catalog.md` §8.3 —— 一句话：位由平台适配器回答，而适配器看不见
  插件（实测 `floom-platform*` 里 `floom-plugin-*` 依赖为 0，且不能有），
  加回去只会得到一个"本层无人实现"的位，退回 A8 修掉的问题。
- 补的真缺口：`floom-plugin-file-dialog::SUPPORTED` —— `pick_file` 不能当探针
  （支持的目标上会弹模态框），宿主需要不调就知道该不该显示按钮。它和臂由同一张
  target 表生成，不可能脱节。

**仍需设备的部分**：Android 上真的打开一个 URL、真的弹出 picker —— 代码已写完、
三目标已编译，但需要一台设备按 gallery 的按钮确认。

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

- ~~**B7 的 vello GPU 帧缓冲测试**~~ ✅ 已完成（2026-10-27）。`VelloGpuPresenter::render_offscreen`
  把 scene 走**同一条 `encode` 路径**光栅化进离屏纹理再回读成 RGBA；`tests/gpu_pixels.rs`
  有 4 个测试，断言的是**墨迹在哪**（空场景/实心边界、裁剪边缘、阴影在矩形之外），
  不是计数器。`#[ignore]`，因为它要真 adapter；门禁新增的 `gpu` 检查在任何机器上
  **都编译**这条路径（这正是 `gpu` feature 从未被 CI 编译过的补丁），并在有 adapter
  时报「4 pixel tests ran and passed」、没有时报「skipped — no GPU adapter on this
  machine」——**跳过从不计成通过**。`FLOOM_REQUIRE_GPU=1` 让缺 adapter 变红。
- ~~**A6b 注册表**~~ ✅ 已完成（2026-10-27），见第 3 节。
- ~~**8 个 stub 插件的实现代码**~~ ✅ 已完成（2026-10-24），只剩真机确认。


---

## 附：怎么跑测试（完整版，均已验证）

```bash
cd /Users/humphrey/Project/Rust/floem

# 全量测试（`floom-probe` 需要显式 --features real，见下方）
cargo test --workspace
cargo check -p floom-probe --features real   # CI 的 probe-check job 跑这条

# 八个能力插件的三目标编译矩阵（CI 已自动跑前两条）
cargo test \
  -p floom-plugin-battery -p floom-plugin-connectivity -p floom-plugin-device-info \
  -p floom-plugin-local-auth -p floom-plugin-location -p floom-plugin-notifications \
  -p floom-plugin-sensors -p floom-plugin-share -p floom-plugin-vibration
cargo check \
  -p floom-plugin-battery -p floom-plugin-connectivity -p floom-plugin-device-info \
  -p floom-plugin-local-auth -p floom-plugin-location -p floom-plugin-notifications \
  -p floom-plugin-sensors -p floom-plugin-share -p floom-plugin-vibration \
  --target aarch64-apple-ios
# Android 侧：CI 用 --target aarch64-linux-android，本机用 cargo ndk
cargo ndk -t arm64-v8a check \
  -p floom-plugin-battery -p floom-plugin-connectivity -p floom-plugin-device-info \
  -p floom-plugin-local-auth -p floom-plugin-location -p floom-plugin-notifications \
  -p floom-plugin-sensors -p floom-plugin-share -p floom-plugin-vibration

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

### ✅ 已修：`floom-probe` 编译不过（选 C 落地，同时修掉根因）

```bash
cargo check -p floom-probe --features real   # Finished，通过
```

**原来的症状**：`error[E0432]: unresolved imports floom_widgets::BottomSheet,
Dialog, NavDestination, NavigationDrawer, Popup, Select`。

**为什么一直没被发现**：它的 `[[bin]]` 有 `required-features = ["real"]`，
而 `real` 不在 default features 里——所以 `cargo test --workspace`
（不带 `--features real`）**静默跳过了它**。feature 门把破损掩住了。

**这是第 2 个"测试全绿但东西是坏的"案例**。第 1 个（8 个 stub 插件）已修，
但**两个是同一个模式**：**验证的边界没覆盖到出问题的地方**。
stub 插件靠"host 测试 + `cfg(target_os)` 分支从不编译"藏住，probe 靠
`required-features` 藏住。

**实际有两层问题，都不是"组件缺失"**：

1. **import 指错了 crate**。那六个组件在 `ecosystem/floom-ui` 的
   `floom-ui-composites` 里，不在 `floom-widgets`。`floom-widgets` 也不能
   再导出它们——设计系统 crate 依赖 `floom-widgets`，反过来导出会闭合依赖环。
   这是**跨 workspace 的 path 依赖**，`examples/floom-demo` 早就这么依赖
   `floom-material` 等了，有先例，不是新架构。

2. **`NavigationBar` / `NavigationRail` / `NavigationDrawer` 漏了 `WidgetAny`
   注册**。这才是真正的缺陷：三个都有 `semantics_hint`，但都没写进
   `floom-ui-composites/src/any.rs` 的 `impl_widget_any_labelled!` 列表。
   `any.rs` 的模块注释自己就写着这条规则——"加了 widget 却忘了这个文件，
   会在挂载点变成编译错误"。改 import 之后报错就变成了
   `NavigationDrawer is not a Floom 2.0 Widget`，正是这条规则在起作用。

**修法**（已提交）：
- `examples/floom-probe/Cargo.toml` 加 `floom-ui-composites` path 依赖；
- `main.rs` 改成从 `floom_ui_composites` import 那六个；
- `any.rs` 补上三个导航组件的 `WidgetAny`；
- CI 加独立 job `probe-check`（macOS），跑
  `cargo check -p floom-probe --features real`。

**两个决定值得记住**：

- **检查放在 macOS，不放 Ubuntu**。probe 的大部分内容——native view provider
  安装、`NSView::frame` 读回——都在 `#[cfg(target_os = "macos")]` 里。
  Linux 检查会跳过正好最会腐坏的那几条分支，绿着看它烂掉。
- **`probe-check` 不挂 `needs`**。`build` 有 `needs: [fmt, clippy]`，而这两个
  job 在本分支上本来就是红的（见下方"预先存在的红灯"），任何挂在它们后面的
  检查都永远不会执行——那正好又是这个 job 要消灭的同一个毛病。

---

### ✅ 已修：预先存在的红灯（曾让 `build` 永不执行）

当时的状态：

```bash
cargo fmt --all --check      # 2831 处差异
cargo clippy --workspace     # 多处 error（path.rs map_or、native-ffi 裸指针等）
```

`build` 的 `needs: [fmt, clippy]` 因此**从未跑过**，`cargo test --workspace`
那一步同样没跑。两处都已清掉，分成两次提交（`f8093273` 纯格式化 732 个文件，
`997914ec` clippy 修复），所以后者可以当作它实际的改动来读。

**但根因不在 fmt 和 clippy，而在这里：CI 根本没有执行过。** 详见下一节。

---

### ⚠️ CI 从未执行过：origin 是 Gitee，CI 全在 `.github/workflows/`

这是一个必须先知道的事实，否则上面所有"CI 会跑"的句子都会被误读：

- `origin` 是 `https://gitee.com/floem/floem.git`；
- 全部 CI 定义在 `.github/workflows/ci.yml`；
- 仓库里没有 Gitee Go / 任何 Gitee 侧的流水线配置。

也就是说，`.github/workflows/` 里的每一个 job —— 包括 `fmt`、`clippy`、
`build`、`probe-check` —— **在这个仓库的托管方式下不会被执行**。这是为什么
上面那些红灯能存在很久而没人发现，也是为什么"CI 绿"这个结论此前无从谈起。

**缓解措施：本地门禁 `scripts/gate.sh`（`just gate`）。** 它按 job 独立执行、
每条都单独报告、失败数即退出码：

```bash
just gate            # 全部
just gate --list     # 列出有哪些检查
just gate fmt        # 只跑一条
```

设计要点（都是踩出来的）：

- **每条检查独立运行、独立报告**。跳过必须打印理由，不能静默变绿。
- **clippy 用 `--all-features`**。默认 feature 只编译占位实现，
  `floom-platform-desktop` 的整个 winit 后端在 feature 后面。第一次
  `--all-features` 就找出 7 个此前从未被编译过的错误。
- **clippy 范围必须等于 CI 的包列表**。门禁此前截断在 `floom-native-ffi`，
  漏掉全部 9 个插件、`floom-app`、`floom-widgets` 和另外 20 个 crate ——
  于是门禁能报 `PASS clippy`，而三分之一的 track 从未被 lint 过。
- **`android`/`ios`/`windows` 用 `cargo check --target`**，与 CI 一致；`check` 不链接，
  所以不需要 NDK，也不该用 `cargo ndk` 把真检查降级成跳过。
- **交叉目标的 std 要在每个 rustc 里找，不能只问 PATH 上那个**。这台机器
  PATH 上的 `rustc` 是 Homebrew 的，它的 `lib/rustlib` 里**没有任何**交叉目标
  的 std；而三个设备目标的 std 都装在 rustup 的 `1.98-x86_64-apple-darwin`
  里。旧探针只问 PATH 上那个，于是把**已安装**的目标报成缺失，跳过理由写着
  "no … std in this toolchain (`rustup target add …`)" —— 理由本身是假的，
  照它做也不会好。这正是这个门禁存在的理由（"不能失败的门禁比没有更糟"），
  所以探针现在先问 PATH 上的 `rustc`，不行就遍历 `~/.rustup/toolchains/*`，
  找到就把那个 toolchain 的 `bin` 临时前置；真的一个都没有才跳过，并写明
  要找的是哪个 toolchain。
- **`fmt` 要跑每一个 workspace**。仓库里有三个 workspace：根、`crates/ecosystem/floom-ui`、
  `crates/ecosystem/floom-editor`。后两个是独立 workspace，根目录的
  `cargo fmt --all` 一个都到不了；`crates/vendor` 永不格式化。
- **卫星 workspace 是推导出来的，不是手写的**：`crates/ecosystem/` 下任何
  含 `[workspace]` 的 `Cargo.toml`。手写列表正是漏掉第三个的原因。
- **门禁关掉 debuginfo**（`CARGO_PROFILE_DEV_DEBUG=0`、`CARGO_PROFILE_TEST_DEBUG=0`、
  `CARGO_INCREMENTAL=0`，写在脚本里而不是 `[profile]` 里，免得影响普通
  `cargo test`）。这是 `ecosystem` 能否跑得动的关键，见下。

**四类包名错误**（同一种病：`--package=` 里写了根 workspace 解析不出来的名字，
一跑就 `did not match any packages`）：

| 名字 | 问题 |
|---|---|
| `floom-renderer-skia` | 已重命名为 `floom-renderer-skia-abi` |
| `floom-runtime` | 不存在 |
| `floom-theme` | 只在 ecosystem workspace 里（`crates/ecosystem/floom-ui/crates/floom-theme`），根 workspace 永远解析不到 |
| 漏掉 `floom-renderer-tui`、`floom-renderer-web` | CI 与门禁都漏了 |

现在门禁与 CI 的包列表用脚本做过差集比对，两侧都是 58 个，不再靠眼睛核对。

**第 11 条检查：`ecosystem`。** 根 `Cargo.toml` 的 `exclude` 里有
`crates/ecosystem`，所以根目录的 `cargo test --workspace` 永远到不了那两个卫星
workspace。这不是理论缺口：四个设计系统的像素 golden 就住在 `floom-ui` 里，
`d131b5af` 把 `PixelSnapshot::of` 挪到 `ReferencePixels` trait 后面、又教会
vello 画投影，于是那四个 gallery 同时**编译不过**和**哈希不同**，而门禁和 CI
都只报绿。`floom-editor` 的 25 个 paint 级测试是同一处境：已提交、手跑能过、
没有任何东西在跑。现在两边都按推导出的卫星列表循环跑整 workspace。

**实测数字**（这台机器，`CARGO_PROFILE_*_DEBUG=0`）：

| workspace | 成员 | 测试 | 冷 | 热 | target |
|---|---|---|---|---|---|
| `crates/ecosystem/floom-ui` | 306 | 961 个 test binary，3602 passed / 0 failed / 16 ignored | 273 s | 68 s | 3.2 GB |
| `crates/ecosystem/floom-editor` | 4 | 25 passed | 126 s | — | 1.1 GB |

开完整 debuginfo 时同一条 `floom-ui` 测试跑过 **>109 分钟**、target 约 **46 GB**，
第一次尝试还因为磁盘写满被静默杀掉、日志里一个 `test result` 都没有。所以
debuginfo 是这条检查的成本大头，不是成员数。

**当前状态（2026-10-25 19:51）**：`just gate` **12 项全绿**
（`12 passed, 0 skipped, 0 failed`）。

此前（19:30）`fmt` 曾经红过一阵，红在**尚未提交的 `HostedNode` 重构**上
（`floom-app/src/lib.rs`、`floom-devtools`、`floom-widgets/src/surface.rs` 等 9 个文件），
不是门禁本身的问题——重构一提交（`a1d9d851`），`fmt` 立即转绿，**没有被 allowlist 或
豁免绕过**。同一提交里也带上了两处依赖它才能编译的测试（`devtools_demo.rs` 的
三参 `tree_rows`、`hosted_tree.rs` 四处 fixture 的 `style`/`layout` 字段）。

门禁此前报过两个真缺陷，都已修（见 `plugin-catalog.md` A5 的 as-built）：
`floom-plugin-prefs` 的测试名在本机 1 µs 时钟下会撞，导致两个测试共用一个 domain、
互相抹掉——这是 `test` 间歇性红的真因；以及 Apple 臂把 `""` 折成 `None`，与另外两臂
不一致。两处都留了会翻红的测试。


---

## 优先级建议

1. **第 1 节的剩余部分（真机确认）** —— 代码已写完、三目标已编译、CI 已加护栏，
   只差一台设备把表格填满。你现在就有 Android 工具链。
2. **第 3 节（A6b）** —— 等有了第二个 native-view 插件才有意义；查找键的取舍在
   `plugin-catalog.md` §8.2。
3. **第 2 节（camera/webview）** —— 最重，放最后。

~~第 4 节（C4 决策）~~ —— ✅ 2026-10-25 已拍板，见本节。
