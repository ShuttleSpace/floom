<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# 平台插件：按使用率重排优先级（记录）

2026-10-27。这份记录只回答一件事：`docs/plugin-catalog.md` 排出的 A/B/C 批次，
和「用户实际上最需要什么」差多远。

它不改写 catalog——catalog 的取样口径（publisher）是它自己的边界，Tier A 已经全部落地。
这里给的是第二口径（使用率）下的重排，以及 catalog 没收录的缺口。

## 0. 结论先行

1. **catalog 的取样是 publisher，不是使用率。** Top 20 里 10 个不在这两个 publisher
   下（firebase 5 个 + `flutter_secure_storage` / `permission_handler` / `app_links` /
   `sqflite` / `mobile_scanner`）。其中 `flutter_secure_storage`（5.6M/月）**从未进过
   目录**，是最大的单点缺口。
2. **Tier B 的内部顺序要改。** 使用率：`image_picker` 5.60M > `webview_flutter` 3.82M
   > `video_player` 3.75M > `google_maps_flutter` 1.11M > `camera` 0.98M（另加
   `camera_android_camerax` 0.74M）。catalog 把 webview 排第一的理由是 provider ABI，
   但 `image_picker` 不需要 provider——它是系统选择器（ACTION_PICK / PHPicker），
   卡的是另一件事（见第 5 节）。
3. **最高杠杆的一块基础设施是「回调结果通道」，不是任何一个插件。** `app_links`
   （3.5M，深链接收）、`image_picker`（5.6M）、移动端 `file_dialog`、`quick_actions`
   （207k）、`mobile_scanner`（2.0M）卡的是同一个洞：native 侧通过 activity result /
   delegate 回调把结果送回来，而平台层没有这条通道。补这一条，解锁的是一组，不是一个。
4. **已有插件的平台覆盖不全，补齐比新增便宜。** `share`（share_plus 4.94M）和
   `notifications`（flutter_local_notifications 3.56M）都只有 Android + iOS 臂；
   两个上游包都是全桌面。桌面用户按同一个能力名调用会得到「不可用」。
5. **桌面三件套里两件该做成插件。** 这条原本写的是「只有一件」：当时把
   `tray_manager`（229k）和 `window_manager`（876k）一起归给平台层，理由是
   「必须有宿主」。前半对、后半错——窗口属于宿主，所以 `window_manager` 的能力
   落在 `floom-platform-desktop` 的 `Window::control`；**托盘不属于宿主**，
   `NSStatusItem`、`Shell_NotifyIconW` 的 message-only 窗口、D-Bus 上的
   `StatusNotifierItem` 都是插件自己造的对象，所以我把它做成了
   `floom-plugin-tray`（2026-10-28，三平台，见 `plugin-catalog.md` §10.1）。
   `screen_brightness`（226k）仍然是普通 L5 插件。

## 1. 方法

- pub.dev 搜索 `is:plugin`，`sort=popularity`，翻 10 页取前 100 个包名。
- 逐个读 `https://pub.dev/api/packages/<name>/score`：`downloadCount30Days` +
  `likeCount` + `publisher:*` 标签。
- 按 30 天下载排序；点赞数作为第二信号（下载含 CI，点赞更接近「人在用」）。
- 时间点：2026-10-27。

**边界，明确写下**：30 天下载包含 CI 与间接依赖的下载，`path_provider` 这类被大量包
当依赖的会被系统性放大；`likeCount` 反过来偏向长期维护的包。两者只用来排序，
不用来断言「多少应用在用」。使用率回答的是「值不值得做」，不是「谁在用」。

## 2. 数据（Top 40 + 点名项）

图例：`✅` 已有全平台 · `◐` 已有但覆盖不全 · `○` catalog 有条目未做 ·
`✗` 缺口（catalog 未收录） · `—` 不迁（厂商 SDK / app 层） · `⚠` 平台层而非插件

| 30 天下载 | 点赞 | 包 | publisher | Floom |
|---:|---:|---|---|---|
| 9,801,885 | 5,571 | `path_provider` | flutter.dev | ✅ `paths` |
| 8,945,286 | 8,189 | `url_launcher` | flutter.dev | ✅ `open-url` |
| 8,642,810 | 10,583 | `shared_preferences` | flutter.dev | ✅ `prefs` |
| 7,478,605 | 120 | `flutter_plugin_android_lifecycle` | flutter.dev | ⚠ 生命周期桥，非能力 |
| 5,613,726 | 4,504 | `flutter_secure_storage` | steenbakker.dev | ✗ Keychain / Keystore |
| 5,599,937 | 7,774 | `image_picker` | flutter.dev | ○ B4 |
| 5,584,079 | 4,080 | `firebase_core` | firebase.google.com | — |
| 4,942,733 | 4,032 | `share_plus` | fluttercommunity.dev | ◐ `share` 仅移动端 |
| 4,376,967 | 4,101 | `connectivity_plus` | fluttercommunity.dev | ✅ `connectivity` |
| 4,089,787 | 3,951 | `firebase_messaging` | firebase.google.com | — |
| 3,872,668 | 6,016 | `permission_handler` | baseflow.com | ⚠ 模型不同，见 §6 |
| 3,820,413 | 4,534 | `webview_flutter` | flutter.dev | ○ B1 |
| 3,751,475 | 3,724 | `video_player` | flutter.dev | ○ B3 |
| 3,563,357 | 7,357 | `flutter_local_notifications` | dexterx.dev | ◐ `notifications` 仅移动端 |
| 3,513,144 | 1,325 | `app_links` | cow-level.ovh | ✗ 深链接收（卡结果通道） |
| 3,151,940 | 5,572 | `sqflite` | tekartik.com | ✗ SQLite 绑定 |
| 2,602,990 | 648 | `wakelock_plus` | fluttercommunity.dev | ✅ `wakelock` |
| 2,460,080 | 1,314 | `firebase_analytics` | firebase.google.com | — |
| 2,447,431 | 3,608 | `google_sign_in` | flutter.dev | — catalog C4 建议不迁 |
| 2,352,232 | 1,334 | `firebase_crashlytics` | firebase.google.com | — |
| 2,039,593 | 4,301 | `firebase_auth` | firebase.google.com | — |
| 2,027,896 | 2,307 | `mobile_scanner` | steenbakker.dev | ✗ 条码（依赖相机） |
| 1,801,031 | 3,383 | `local_auth` | flutter.dev | ✅ `local-auth` |
| 1,457,926 | 896 | `record` | llfbandit.dev | ✗ 录音（流式，卡通道） |
| 1,400,190 | 2,458 | `in_app_review` | britannio.dev | ✗ 商店评分 |
| 1,352,451 | 655 | `firebase_remote_config` | firebase.google.com | — |
| 1,315,907 | 343 | `flutter_timezone` | sandromancuso.dev | ✗ 系统时区名 |
| 1,179,815 | 1,809 | `printing` | flutter.dev | ✗ 打印 / PDF |
| 1,034,340 | 2,484 | `google_mobile_ads` | google.dev | — |
| 978,388 | 2,603 | `camera` | flutter.dev | ○ B2 |
| 936,077 | 440 | `file_selector` | flutter.dev | ◐ `file-dialog`（open/save/dir 有，filters 无） |
| 876,343 | 1,126 | `window_manager` | leanflutter.dev | ⚠ 平台层（无运行时窗口 API） |
| 817,069 | 1,251 | `app_settings` | spicylemon.dev | ✗ 跳系统设置页 |
| 779,727 | 1,039 | `sensors_plus` | fluttercommunity.dev | ✅ `sensors` |
| 759,187 | 2,496 | `in_app_purchase` | flutter.dev | ○ C3 |
| 747,465 | 1,155 | `flutter_keyboard_visibility` | eopeter.dev | ◐ 有输入控制，无可见性查询 |
| 738,976 | 1,390 | `geocoding` | baseflow.com | ✗ 地理编码（平台 geocoder） |
| 736,577 | 81 | `camera_android_camerax` | flutter.dev | ○ B2（B2 的 Android 实现） |
| 688,807 | 513 | `cloud_functions` | firebase.google.com | — |
| 683,537 | 594 | `gal` | natsuk4ze.dev | ✗ 存图到相册 |
| 641,444 | 222 | `desktop_webview_window` | leanflutter.dev | ○ B1（桌面实现） |
| 501,338 | 780 | `photo_manager` | fluttercandies.com | ✗ 相册库（读） |
| 469,200 | 1,086 | `open_file` | crazecoder.dev | ✗ 用默认应用打开 |
| 423,032 | 480 | `app_tracking_transparency` | kinguin.dev | ✗ iOS ATT |
| 354,070 | 1,308 | `in_app_update` | poitch.dev | ✗ Android 应用内更新 |
| 334,674 | 171 | `app_badge_plus` | 4t.dev | ✗ 角标 |
| 316,257 | 503 | `file_saver` | info-library.dev | ◐ 存文件（save dialog 有，写字节无） |
| 296,962 | 502 | `flutter_contacts` | fluttercandies.com | ✗ 通讯录 |
| 292,244 | 2,220 | `home_widget` | abaus.dev | ✗ 桌面小组件 |
| 268,482 | 1,363 | `flutter_webrtc` | cloudwebrtc.dev | ⚠ 大件，不是能力插件 |
| 265,974 | 505 | `background_downloader` | 4t.dev | ○ C5 |
| 228,814 | 291 | `tray_manager` | leanflutter.dev | ✅ `tray` 三桌面（手机无托盘） |
| 226,384 | 170 | `screen_brightness` | aaassseee.dev | ✗ 屏幕亮度 |
| 207,053 | 1,135 | `quick_actions` | flutter.dev | ○ C1 |
| 205,649 | 964 | `map_launcher` | mohameddev.dev | ✗ 唤起地图 app |
| 203,170 | 119 | `pasteboard` | mixin.dev | ◐ 平台层已有 clipboard，无插件封装 |
| 176,189 | 132 | `volume_controller` | kurenai.dev | ✗ 音量 |
| 174,149 | 282 | `flutter_custom_tabs` | poitch.dev | ✗ 应用内浏览器标签 |
| 171,967 | 530 | `restart_app` | 4t.dev | ✗ 应用重启 |

## 3. 缺口分级

### S 级：高使用率 + 本机可验

- **`floom-plugin-secure-storage`**（5.61M）—— Keychain（Apple）、
  `EncryptedSharedPreferences`/Keystore（Android）、`libsecret`（Linux）、DPAPI（Windows）。
  macOS keychain 可以在普通 host test 里做真实往返（和 `prefs` 的 `NSUserDefaults`
  同一类），所以第一批就该有它。和 `prefs` 的分工必须写清：prefs 是明文偏好，
  secure-storage 是「不信宿主文件系统」的那一层。
- **补 `share` 的桌面臂**（share_plus 4.94M）—— macOS `NSSharingServicePicker`、
  Windows `DataTransferManager`、Linux 无统一 API（点名 xdg-desktop-portal）。
  已有插件的覆盖补齐，成本远低于新 crate。
- **补 `notifications` 的桌面臂**（flutter_local_notifications 3.56M）—— macOS
  `UNUserNotificationCenter`，Linux `org.freedesktop.Notifications`，Windows `ToastNotificationManager`。

### A 级：单平台几十行，使用率 0.2–1.4M

`app_settings`（817k）、`geocoding`（739k）、`in_app_review`（1.40M）、
`app_tracking_transparency`（423k）、`in_app_update`（354k）、
`app_badge_plus`（335k）、`screen_brightness`（226k）、`map_launcher`（206k）、
`volume_controller`（176k）。

共同点：一个平台 API 一件事，无状态机、无回调通道。缺点也一致：多数**只能在真机上验**，
host 上只能编译加惰性臂——按本仓库的规矩，它们要进「验证矩阵」时得明说这一列是空的。

### B 级：需要新基础设施

- **回调结果通道**（见 §5）→ `app_links` 3.51M、`image_picker` 5.60M、移动 `file_dialog`、
  `quick_actions`、`mobile_scanner` 2.03M、`record` 1.46M（流式是同一通道的连续版本）。
- **相机管线** → `camera` 978k + `camerax` 737k、`mobile_scanner`、`gal` 684k、
  `photo_manager` 501k 都建立在它上面。
- **SQLite 绑定**（`sqflite` 3.15M）—— 需要判断：`floom-persistence` 已经覆盖应用侧
  持久化，SQLite 是另一件事（原生库 + 平台路径 + 迁移），还是定位成 persistence 的一个后端？
- **打印 / PDF**（`printing` 1.18M）—— PDF 生成是纯 Rust 的事，平台打印对话框是另一件；
  两者不要捆成一个 crate。

## 4. 已有插件的真实覆盖（核对结果）

| 插件 | 臂 | 上游包的平台 |
|---|---|---|
| `share` | Android + iOS | share_plus：全平台 |
| `notifications` | Android + iOS | flutter_local_notifications：全平台 |
| `file-dialog` | macOS + Linux + Windows；手机精确拒绝 | file_selector：全平台 |
| `clipboard`（平台层，非插件） | Android + iOS + macOS | pasteboard：全平台 |
| `vibration` | Android + iOS（桌面无 API，天然为空） | vibration：移动为主 |

结论：**`share` 和 `notifications` 的桌面缺口是「上游有、我们没有」，不是「平台没有」**。
这类缺口在使用率口径下比新插件更显眼，因为用户按同一个名字调用。

## 5. 最高杠杆：回调结果通道

catalog 的 Tier B 把「native view provider ABI」当成 Tier B 的门槛，这个判断只对了一半。
使用率数据里卡住的插件分成两类：

1. **需要一块原生视图**（webview、video、maps）——确实要 provider ABI。
2. **需要一次回调**（image_picker、app_links、file_dialog 手机端、quick_actions、
   mobile_scanner、record）——要的不是视图，是**结果回来的路**：
   Android 的 `onActivityResult`、iOS 的 delegate 回调。

`file_dialog` 的手机臂当初精确拒绝，理由正是「平台层没有这条通道」。这条通道补上，
解锁的是 6 个能力、合计约 1,300 万次月下载（去重前的上界，但量级说明问题）。
所以下一批真正该排在第一的，不是某个插件，而是这条通道本身。

## 6. 需要拍板的点

- **`permission_handler`（3.87M）不照搬。** 它是 Flutter 的聚合器：一个 API 查所有权限。
  我们的模型是每个能力自带授权（`location::request_authorization`、
  `notifications::request_authorization`、`local_auth`）。照搬会把权限模型从能力里
  抽出来，和现有设计相反。但有一个真缺口值得单独看：**查询**某能力当前授权状态
  （camera / photos / mic 在 Tier B 里都要用），这应该是每个能力自己的 `status()`，
  不是一个中央 `permission_handler`。
- **`flutter_secure_storage` 归 L5 还是平台层？** Keychain/Keystore 是平台 API，
  但能力语义、迁移、命名空间都是插件的。倾向前者：和 `prefs` 并列。
- **`sqflite` 是否要**：取决于 `floom-persistence` 的定位，不取决于使用率。
- **`in_app_purchase`（C3）与 `in_app_review`（A 级）不同档**：前者是服务端状态机
  （catalog 已判风险最高），后者只是一个系统评分弹窗。不要因为同属「商店」就一起排。

## 7. 重排后的建议批次

```
批次 4'（真机 + provider）：B4 image-picker → B1 webview → B3 video-player
                          → B5 maps → B2 camera
        （image_picker 提前：使用率最高，且系统选择器的实现面比 webview 小）
批次 5'（本机可验）
        secure-storage（macOS keychain 真往返）
        share 桌面臂 + notifications 桌面臂（补覆盖，不是新 crate）
        app_settings / in_app_review / geocoding / screen_brightness / map_launcher
批次 6'（需要新基础设施，另开）
        回调结果通道 → app_links + 移动 file_dialog + quick_actions + mobile_scanner
        camera 管线 → gal / photo_manager
        桌面宿主 → window_manager 等价（窗口运行时 API）、tray_manager 等价（托盘）
        仍不迁：firebase 全家、google_mobile_ads、google_sign_in（C4 理由不变）
```

## 8. 这份记录不做什么

- 不重写 catalog 的 Tier 划分；Tier A 已完成，Tier B/C 的条目仍然有效，只是顺序变了。
- 不用下载量断言「多少应用在用」；它只用来判断「值不值得做」。
- 不含 web 平台判断（`floom` 的 web 后端是另一条线）。
