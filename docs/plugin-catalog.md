# 平台插件迁移目录（Flutter → Floom）

对 pub.dev 两个官方发布者（`flutter.dev`、`fluttercommunity.dev`）名下**全部插件包**
的逐条盘点，结论是「迁什么、不迁什么、按什么顺序迁」。

数据采集于 2026-10-25：用 `is:plugin` 过滤逐页抓取，两个发布者共 **90 个插件包**，
去掉 federated（一能力多平台分包）的重复后是 **35 个不同能力**。其中 6 个已有
对应插件，**29 个是候选**。

这份目录只做判断和排序，不写代码。每个条目写清四件事：对应哪个 Flutter 包、
Floom 侧叫什么、为什么这个优先级、以及它会不会撞上已有 crate。

---

## 0. 先读这一节：三条硬约束

迁移不是「Flutter 有所以我们也得有」。三条约束来自仓库自己的教义
（`docs/platform-and-plugins.md`），任何条目违反一条就不进目录。

**约束一：一个能力一个 crate，按需链接。**
SDK 不携带任何能力插件。app 想要电池就连 `floom-plugin-battery`，不想要就不链接。
这条是把 13 个包从 `floom-platform-mobile` 拆成 9 个插件的全部理由，新增插件必须
继续成立。

**约束二：host-inert，且诚实地 inert。**
在桌面/web 上必须仍然编译，并且返回 `Err` / `false` / `Option::None` /
`Insets::ZERO` 这种「不声称测到过」的值，而不是一个像测到的默认值。
`floom-plugin-battery` 的 `BatteryState::level` 从 `f32` 改成 `Option<f32>` 就是
为了这个：读失败和电量满必须能被调用方区分。

**约束三：没有实现的位不存在。**
`Capability` 位集里每个位背后都必须有实现——A8 为此删掉了 `OPEN_URL`、
`FILE_DIALOG`、`DRM_KMS` 三个无实现的位。所以「加一个能力」意味着**先有实现再加位**，
顺序反了就是把 A8 修掉的问题请回来。

### 排序依据

优先级由四件事决定，按权重排：

1. **host 上能不能验证**。能在 macOS 本机跑通桌面臂的排前面——这条直接把 Tier A
   顶到最前，因为 8 个全部有真实桌面实现（`dirs` / `open` / `rfd` / 读
   `Info.plist`）。
2. **是否撞已有 crate**。`floom-persistence` 已经有了通用 KV + snapshot，
   `shared_preferences` 迁过来必须说清和它的分工，否则是重复造轮子。
3. **是否需要 provider ABI / 真机**。需要 native view 的（camera/webview）全部
   排在后面，它们是 handoff 第 2 节的内容，卡设备。
4. **是否是厂商 SDK 绑定**。Google Sign-In / Google Maps / ArcGIS 这类绑定单一
   厂商服务的，作为插件会把 SDK 许可和体积问题带进每个人的构建，排在最后并标注。

---

## 1. 已覆盖：6 个，不要再造

| Flutter 包 | Floom 已有 | 备注 |
|---|---|---|
| `battery_plus` | `floom-plugin-battery` | 两侧真实现（`ACTION_BATTERY_CHANGED` / `batteryLevel`） |
| `connectivity_plus` | `floom-plugin-connectivity` | `SCNetworkReachability` / `ConnectivityManager` |
| `device_info_plus` | `floom-plugin-device-info` | `Build` / `UIDevice` |
| `sensors_plus` | `floom-plugin-sensors` | iOS 的 `Proximity` / `AmbientLight` 恒 false——平台无查询 API，如实报告 |
| `share_plus` | `floom-plugin-share` | Android `share_files` 缺 `FileProvider` 返回精确 `Err` |
| `local_auth` | `floom-plugin-local-auth` | 两侧 `authenticate` 都返回 `Err`：同步签名无法诚实断言「已认证」 |

这 6 个是 2026-10-24 从 TODO 桩补成真实现的那批（audit 的 A7b 行）。
**它们的形态就是新插件的参照物**，尤其是 `Err` 的措辞——措辞本身是设计的一部分。

另有一个不算能力插件但已存在的：`floom-plugin-nativeview`（provider ABI 的先行者，
原生半边是 Objective-C）。它是 Tier B 的全部基础。

---

## 2. Tier A — 查询类能力插件（8 个，先做这批）

共同特征：同步查询或一次性动作、host-inert 容易写、**桌面臂在本机 macOS 上就能验证**、
不需要 provider ABI、不卡设备。全部落在 L5，与现有 9 个插件同层。

### A1. `floom-plugin-paths` ← `path_provider`（+ `xdg_directories`）

最该先做的一个：几乎每个 app 都要往磁盘写点东西，而现在没有任何 crate 回答
「我的应用支持目录在哪」。

- Android `Context.getFilesDir` / `getCacheDir` / `getExternalFilesDir`
- iOS `NSFileManager URLsForDirectory:inDomains:`（`NSApplicationSupportDirectory` 等）
- 桌面：`dirs` crate（Linux 走 XDG，macOS `~/Library/Application Support`，
  Windows `%APPDATA%`）
- host-inert 语义：桌面**不是** host——桌面是这些目录的真正宿主。只有 web/tui
  这类目标才拒绝。

**注意分工**：`floom-persistence` 提供的是 `KeyValueStore` trait + `FileStore` /
`MemoryStore` 后端，它**不知道**目录在哪，要调用方给路径。`floom-plugin-paths`
回答的正是「路径是什么」。两者是上下游，不是重复。

### A2. `floom-plugin-open-url` ← `url_launcher`

**这一项直接结掉 handoff 第 4 节的 C4 决策。** A8 删掉 `OPEN_URL` 位的理由是
「没有实现的位不该存在」；有了这个插件就有了实现，位可以加回来。

- macOS `NSWorkspace.sharedWorkspace openURL:`（objc2-app-kit 已在 lock 里）
- Windows `ShellExecuteW`
- Linux `xdg-open`（或 `gio open`）
- Android `Intent.ACTION_VIEW` + `startActivity`
- iOS `UIApplication.sharedApplication openURL:options:completionHandler:`
- web：`window.open`（wasm 目标）

桌面臂的验收标准很硬：**真的打开一个 URL**。macOS 本机可验，这是它排第二的原因。

### A3. `floom-plugin-file-dialog` ← `file_selector`

C4 的另一半。`file_selector` 在 Flutter 侧是 federated 包（6 个分包），
说明每个平台的选取器都真的不一样。

- 桌面：`rfd` crate（异步文件对话框，跨三平台）
- Android：`ACTION_OPEN_DOCUMENT` / `ACTION_CREATE_DOCUMENT`（SAF，不需要存储权限）
- iOS：`UIDocumentPickerViewController`

**与 `image_picker` 的分工要写进文档**：`file_selector` 是「任取文件」，
`image_picker` 是「取图片/视频，含拍照」。后者依赖前者加相机。

### A4. `floom-plugin-package-info` ← `package_info_plus`

版本号、build 号、包名/identifier。

- Android `PackageManager getPackageInfo`（`versionName` / `versionCode`）
- iOS `CFBundleShortVersionString` / `CFBundleVersion`（读 `Info.plist`）
- 桌面：构建期由 `build.rs` 从 `CARGO_PKG_VERSION` 写进来，运行期读常量
- **host 语义**：桌面返回的是 crate 版本而不是「安装包版本」，这个区别必须在文档里
  写明，否则调用方会以为拿到的是安装器版本。

### A5. `floom-plugin-prefs` ← `shared_preferences`

OS 支持的键值存储（`SharedPreferences` / `NSUserDefaults`），
区别于 `floom-persistence` 的文件后端。

**分工（必须写进 crate 文档，否则一定被重复造）**：

| | `floom-persistence` | `floom-plugin-prefs` |
|---|---|---|
| 存哪 | 调用方给的 `KeyValueStore`（文件/内存） | OS 的偏好存储 |
| 同步/异步 | 同步 | 两侧都是同步 API |
| 快照/恢复 | 有 `RestorationSnapshot` | 无，是偏好不是会话 |
| 谁该用 | 会话状态、可恢复 UI | 用户设置、开关 |

Flutter 侧这两个也是不同包（`shared_preferences` vs 应用自管的 JSON），
分工一致。

### A6. `floom-plugin-wakelock` ← `wakelock_plus`

保持屏幕常亮。

- Android `WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON`（不需要权限）
- iOS `UIApplication.isIdleTimerDisabled`
- Windows `SetThreadExecutionState`；macOS `IOPMAssertionCreateWithName`
  （这两个让桌面臂**不是**空实现，值得做）
- Linux：无通用 API，如实 `Err`

### A7. `floom-plugin-network-info` ← `network_info_plus`

Wi-Fi 的 SSID / BSSID / IP / 网关。

- Android `WifiManager.getConnectionInfo`（**API 27+ 读 SSID 需要定位权限**，
  且只能拿到本 app 的——文档要写清这个限制）
- iOS `CNCopyCurrentNetworkInfo`（**iOS 12+ 需要 entitlement，且系统已弃用**——
  这一条很可能让 iOS 臂变成精确 `Err`，那本身就是有价值的结论）
- 桌面：枚举接口在三个平台各不相同且都繁琐，**建议 host 臂直接精确 `Err`**
  而不是给一个空列表

### A8. `floom-plugin-background-work` ← `android_alarm_manager_plus` + `workmanager`

定时/延迟的后台任务。

**这一项顺带解掉一个现存的精确 `Err`**：`floom-plugin-notifications` 的 Android
延迟通知现在返回 `Err`，理由是「`AlarmManager` 未接线」。接线的地方就是这个插件——
通知插件之后可以改为「延迟调度委托给 background-work」。

- Android `AlarmManager.setExactAndAllowWhileIdle` / `WorkManager`
- iOS `BGTaskScheduler`（后台刷新，系统决定何时跑——**`Ok` 只能声称「已提交」**）
- 桌面：无等价物，精确 `Err`

> **一个刻意的合并**：Flutter 侧 `android_alarm_manager_plus` 和 `workmanager` 是
> 两个包，但它们在我们这里的形态是同一个能力（「让一段代码稍后跑」），
> 分成两个 crate 会各拿到半个 API。合并为一个，文档里点名这两个来源。

---

## 3. Tier B — 原生视图能力插件（5 个，需要 provider ABI）

共同特征：能力本身是一个**视图**（框架不绘制它），必须实现第二个契约——
provider ABI（`crates/floom-native-ffi/include/floom_native_provider.h`），
并且要按 handoff 第 3 节的 A6b 决策把 provider 注册表做出来。
**全部卡真机**，所以排在 Tier A 之后。

这 5 个也是 handoff 第 2 节列的「camera / webview 真能力插件」的完整清单。

### B1. `floom-plugin-webview` ← `webview_flutter`

**建议作为 Tier B 的第一个**：验收标准最明确（「页面里的 JS 能调到 Rust 侧一个
函数并拿到返回值」），且 Android 有 `WebView`、iOS/macOS 有 `WKWebView`，
不需要相机权限这类容易卡住的东西。

- Android `android.webkit.WebView`（注意：`hasCode="false"` 的 NativeActivity 示例
  没有 androidx，是否要为此引入 `floom-platform-android` 侧的依赖是待决策项）
- iOS / macOS `WKWebView` + `WKUserContentController` 做 JS 桥
- JS→Rust 的方向走 provider 自己的回调表，不经过 `floom_<name>_arkts_call`

### B2. `floom-plugin-camera` ← `camera`

Flutter 侧是 6 个 federated 包，每个平台的相机 API 都不同。

- Android CameraX（`camera_android_camerax` 就是干这个的独立包）
- iOS AVFoundation
- 帧的交付：**外部纹理**，不要每帧拷贝——这是文档里已经写明的原则

`docs/platform-and-plugins.md` 已经点名 camera 是「第一个真正要 provider ABI 的能力」，
现在 nativeview 已经把链路打通，camera 是顺理成章的下一步。

### B3. `floom-plugin-video-player` ← `video_player`

Android ExoPlayer / iOS AVPlayer + `AVPlayerLayer`。
比 camera 简单（不需要权限），但需要音视频解码链路，验收要真机放一个视频。

### B4. `floom-plugin-image-picker` ← `image_picker`

「选图片/视频 + 拍照」。**依赖 A3（file-dialog）和 B2（camera）**，
所以必须排在两者之后。Flutter 侧 7 个 federated 包，也是这个能力平台差异大的证据。

### B5. `floom-plugin-maps` ← `google_maps_flutter` / `apple_maps_flutter` / `platform_maps_flutter`

**排最后且标注风险**：三个 Flutter 包说明连 Flutter 社区自己都没统一。
Google Maps 要 API key + 厂商 SDK；Apple Maps 只有 iOS。
建议做法是先做 `apple_maps_flutter` 对应物（`MKMapView`，无 key 无依赖），
Google Maps 版作为可选的第二个 crate，而不是一个 crate 里塞两家。
`platform_maps_flutter`（跨平台统一层）在我们这里的对应物是
「宿主选 provider」——那正是 A6b 注册表要解决的问题。

---

## 4. Tier C — OS 集成与后台（6 个，视需要取）

### C1. `floom-plugin-quick-actions` ← `quick_actions`

主屏长按快捷菜单（iOS Quick Actions / Android App Shortcuts）。
- iOS `UIApplicationShortcutItem`（`application:performActionForShortcutItem:`）
- Android `ShortcutManager` + manifest 声明
- **注意**：shortcut 的点击会带一个 URL 进 app，需要 `floom-plugin-open-url`
  的反向（深链接收）配合，后者是窗口层的事——文档要写清这个边界。

### C2. `floom-plugin-intent` ← `android_intent_plus`

通用 Android Intent 发起。**只有 Android 臂**，host-inert 是平凡的。
放最后是因为它没有跨平台意义，且容易被滥用成「什么都往 intent 里塞」。
如果要做，API 必须收得很窄（指定 action + extras），而不是把 Intent 整个暴露出来。

### C3. `floom-plugin-billing` ← `in_app_purchase`

StoreKit / Play Billing。**风险最高的一项**：商品 ID、收据验证、订阅状态机
都是服务端的事，插件只能做到「把购买流程跑通并把凭据交出来」。
Flutter 侧拆成 `in_app_purchase` + `in_app_purchase_storekit` 两个包正是因为
两端差异大到无法共用实现。建议：先只做「查询商品 + 发起购买 + 返回凭据」，
不做任何服务端假设。

### C4. `floom-plugin-google-sign-in` ← `google_sign_in`

厂商身份绑定，和 C3 一样的问题：OAuth client ID、scopes、令牌刷新都是服务端配置。
**建议不做**，理由写进目录：一个框架插件不应该要求所有使用者都去 Google Cloud
建项目。app 层自己接 `oauth2` crate 更合适。

### C5. `floom-plugin-transfer` ← `flutter_downloader` + `flutter_uploader`

后台下载/上传，带进度和断点。

- Android `DownloadManager` / WorkManager
- iOS `URLSession` 后台配置（`URLSessionConfiguration.background(withIdentifier:)`）
- **与 A8 的关系**：后台传输的后台执行正是 A8 的能力，两者有依赖——
  先 A8 后 C5

### C6. `floom-plugin-mdns` ← `multicast_dns`

mDNS/Bonjour 服务发现（「局域网里有没有人提供 _http._tcp」）。
- 这是个**纯 Dart 包**（不在 `is:plugin` 列表里），但它对应的是真实平台能力
- Rust 侧有 `mdns-sd` / `libmdns` 可用，不需要自己实现协议
- 排最后：用例窄（局域网设备发现），但成本低、不卡设备

---

## 5. 不迁：以及它们本来该去哪

不迁不等于不重要。这一节的价值是**防止下次再讨论一遍**。

### 5.1 纯 Dart UI / 工具库 —— 不是平台能力

`rxdart`、`equatable`、`animations`、`material_ui`、`cupertino_ui`、
`cupertino_icons`、`font_awesome_flutter`、`vector_math`、`listen`、`redux`、
`sealed_unions`、`backdrop`、`sticky_headers`、`infinite_listview`、
`draggable_scrollbar`、`two_dimensional_scrollables`、`after_layout`、
`interval_tree`、`rx_command`、`state_persistence`、`firestore_helpers`、
`flutter_lints`、`flutter_launcher_icons`、`import_sorter`、`minibar`、
`mustache_template`、`path_parsing`、`rfw`。

其中几个在 Floom 侧**有对应物但属于别的层**，记下来避免误迁：

| Flutter 包 | Floom 侧对应物 | 层 |
|---|---|---|
| `vector_math` | `floom-types` 的几何/矩阵类型 | L0 |
| `animations` / `after_layout` | `floom-animation` / 布局回调 | L5 |
| `redux` / `state_persistence` / `listen` | `floom-signal` + `floom-persistence` | L2 / L5 |
| `rfw`（远程 widget） | `floom-dsl` + `floom-ui-ir` | L8 |
| `flutter_svg` / `vector_graphics` / `path_parsing` | 渲染器 + `floom-text` 的矢量路径 | L4 / L3 |
| `google_fonts` / `flutter_blurhash` | 资源加载 / 图像解码（widget 层） | L6 |
| `flutter_shaders` | `floom-renderer` 的 shader 支持 | L4 |

### 5.2 工具链 / 代码生成 / 测试 —— 不是运行时能力

`pigeon`、`standard_message_codec`、`plugin_platform_interface`、
`flutter_plugin_tools`、`espresso`、`devtools_*`、`web_benchmarks`、
`flutter_plugin_android_lifecycle`。

其中两个值得一句说明：

- `pigeon`（类型安全的平台通道代码生成）对应我们的是
  `floom create --plugin` 的脚手架 + C ABI 头——**我们不需要代码生成器，
  因为插件是链接期依赖而不是运行时通道**。这正是
  `crates/floom-platform/src/plugins.rs` 模块文档论述的差异。
- `flutter_plugin_android_lifecycle` 是「让插件拿到 Android 生命周期」的辅助包。
  我们的对应物是 `floom-platform-mobile` 的 JNI 桥（`with_env` / `activity`），
  9 个插件的 Android 臂都在用它。**不需要新 crate。**

### 5.3 federated 分包本身 —— 不是能力

`*_android` / `*_ios` / `*_windows` / `*_macos` / `*_linux` / `*_web` /
`*_foundation` / `*_darwin` / `*_platform_interface` 这些是 Flutter 的
federated plugin 结构（一个能力按平台拆包）。Floom 的对应结构是
`src/android.rs` + `src/ios.rs` 在同一个 crate 里按 `cfg` 分派——
**一个能力一个 crate，平台是实现细节**。所以这些分包全部折叠掉，不是 29 个候选。

### 5.4 厂商 SDK 绑定 —— 迁了一个就是绑了所有人

`google_sign_in`（C4）、`google_maps_flutter`（B5 的后半）、`arcgis_map_sdk`、
`apple_maps_flutter`（只有 iOS，见 B5）、`wear`（Wear OS 配套）。

`wear` 单独说一句：Flutter 侧它是「给 Wear OS 做配套表盘/app」。我们对应的故事是
`floom-platform-harmony`（OHOS）和手表侧 ArkTS 应用——那是另一条线，
不是这个插件目录要解决的。

---

## 6. 每个新 crate 要动的 8 个地方

漏一个的后果不同，但都会在某个 gate 上炸。按「漏了会怎样」排：

| # | 位置 | 漏了的后果 |
|---|---|---|
| 1 | 根 `Cargo.toml` 的 `members` | `cargo test --workspace` 不覆盖它 |
| 2 | 根 `Cargo.toml` 的 `default-members` | 裸 `cargo build` 不构建它 |
| 3 | `scripts/gate.sh` 的 `FLOOM2_PACKAGES` | **clippy 完全不跑它**——`provider()` 的裸指针签名就是这么溜过去的 |
| 4 | `scripts/gate.sh` 的 `MOBILE_CAPABILITY_PLUGINS`（仅移动插件） | `android` / `ios` check 不跑它，原生半边编译不过也看不见 |
| 5 | `scripts/check-architecture-deps.sh` 的 `LAYERS`（L5 组） | `arch` check **直接失败**：`UNREGISTERED 2.0 library crate(s)` |
| 6 | `docs/2.0/ARCHITECTURE.md` 的 layer 表 | `docs_parity.rs::the_layer_table_matches_the_architecture_gate` 失败 |
| 7 | `.github/workflows/ci.yml` 的 `FLOOM2_PACKAGES` + `android-check` + `ios-check` | CI 三个 job 不跑它（同 #3 #4） |
| 8 | `CHANGELOG.md` 的 `Unreleased` | 无自动失败，但发布政策要求记录 |

**再加 example 接线**（如果它要出现在 `floom-capabilities` 的画廊里）：
`Cargo.toml` 加依赖、`probe.rs` 加 `use`、写一个 `pub fn <name>() -> Row`、
插进 `all()` 的 `vec!`、按需改 `GROUPS`（定长数组，长度变了要改 `; 6]`）；
有副作用按钮的还要改 `actions()` 和 `ui.rs` 的 `run_action()` match。

---

## 7. 验证矩阵（每个插件的验收标准）

沿用 A7b 建立的三目标矩阵，**每个格子都必须说明它能不能答**，而不是假装都答了：

| 目标 | 命令 | 能证明什么 |
|---|---|---|
| host | `cargo test -p floom-plugin-<name>` | host-inert：不编译失败、不返回假值 |
| Android | `cargo check -p ... --target aarch64-linux-android` | 原生半边**编译得过**（CI 加 android-check 之前，8 个插件的原生半边从未被编译过） |
| iOS | `cargo check -p ... --target aarch64-apple-ios` | 同上；`vibration` 的 iOS 半边就带着两个 edition-2024 硬错误躺了很久 |
| 真机 | 设备上手 | 「编译通过 ≠ 能用」 |

**host 测试要断言什么**（这是 A7b 留下的关键教训）：断言 inert 值，
例如 `level: None` 而不是 `100%`、`Err` 而不是 `Ok(false)`。
`floom-capabilities` 的 `a_host_build_never_reports_a_value_it_cannot_have_measured`
就是干这个的——**没有它，stub 和真实现在 CI 眼里完全一样**。

三类落点，每个调用必须归入其一并在文档里写明：

1. **真读** — 同步 API 存在，返回实测值
2. **真发起** — 动作本身就是答案，`Ok` 只声称「OS 已被请求」
3. **精确拒绝** — 同步签名答不了，`Err` 指名缺的是哪一步（不是「暂不支持」）

---

## 8. 三个未决问题（本目录不拍板）

记录在案，各自单独开议题。**本目录的排序不依赖它们的答案**，所以可以先动工。

### 8.1 新插件走哪套形状（教义与代码已分叉）

`docs/platform-and-plugins.md` 第 51-54 行要求每个插件自带 Kotlin/Swift 原生半边 +
`extern "C"` 四函数 ABI，Rust 永不直接碰平台对象。但现有 9 个能力插件全部是
`src/android.rs` 直接 JNI、`src/ios.rs` 直接 objc2，**没有 `native/` 目录**。
`floom create --plugin` 脚手架生成的是第三种形状（C 半边 + `build.rs` + `cc`）。

三种形状并存，迁移目录**按现有 9 个插件的形状（Shape A）写**，
因为代码已经是那个形状，且 `floom-platform-mobile` 的 JNI 桥就是为它存在的。
统一形状是独立的工程决策，不阻塞任何条目。

### 8.2 A6b provider 注册表的查找键

Tier B 的全部 5 个插件都需要它。选项（handoff 第 3 节）：
`NativeViewKind` 枚举（编译期封闭）vs 字符串 kind + `Custom(String)` 兜底。
作者倾向后者（插件要能独立于框架发布），但这是宿主的调用。
**Tier A 的 8 个不依赖它**，所以 A 可以先做完。

### 8.3 C4 的最终形态

A8 删掉了 `OPEN_URL` / `FILE_DIALOG` 两个 `Capability` 位。本目录的 A2/A3 就是
C4 选项 (a) 的落地。但要先确认：位加回来时，是加在 `floom-platform` 的
`Capability` 位集（插件 → 平台的向上依赖，被架构门禁禁止），还是宿主自己查
「这个插件注册了吗」。**后者更符合分层，但意味着 capability 位不再是查询能力的
唯一入口**——这个取舍要拍。

---

## 9. 建议的执行批次

```
批次 1（无需设备，全部本机可验）
  A1 paths      → 最通用，且不撞任何现有 crate
  A2 open-url   → 结掉 C4 的一半，桌面臂真能打开 URL
  A4 package-info → 最小，适合验证「新增 crate 的 8 个地方」流程本身
  （A4 第一个做：它最短，用来把第 6 节的清单走通一遍）

批次 2（无需设备）
  A3 file-dialog → C4 另一半
  A5 prefs       → 要先写清与 floom-persistence 的分工
  A6 wakelock    → 桌面臂非空（Windows/macOS 真有 API）

批次 3（无需设备，但依赖批次 1）
  A7 network-info → iOS 臂很可能是精确 Err，那本身就是结论
  A8 background-work → 顺带解掉 notifications 的延迟 Err

批次 4（需要 A6b 决策 + 真机）
  B1 webview → B2 camera → B3 video-player → B4 image-picker → B5 maps

批次 5（视产品需要）
  C1 quick-actions / C5 transfer / C6 mdns
  C2 intent / C3 billing 视需要
  C4 google-sign-in 建议不做
```

每个批次做完跑一遍第 7 节的矩阵，并在 `docs/plugin-and-renderer-audit.md`
里加一行记录——A7b 的教训是**只记「拆分完成」不记「实现是 stub」**，
所以新条目必须同时记结构和真实度。
