<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# 插件

插件是 floom 的差异化能力之一（竞品维度 ⑤ = 5 分）。契约只有一套，缺口只有两类：**ABI 没版本号**、**桌面侧原生实现不齐**。

## 1. 契约

- 一个插件 = 一个能力 = 一个 crate，**默认不链进 SDK**（`crates/ecosystem/plugins/` 是独立 workspace，root workspace 不构建它们）。
- Rust 侧：能力 trait + 平台分派（`src/{macos,ios,ohos,windows,linux}.rs`）。
- 原生侧：C vtable `FloomNativeProviderV1`，注册函数固定形状
  `pub extern "C" fn floom_<capability>_register_provider(provider: *const FloomNativeProviderV1)`。
  原生侧从构造函数/`JNI_OnLoad`/模块初始化里注册；Rust 只通过 `extern "C"` 调用，不认识平台 API。
- 布局约定（`native/<platform>/` + `native/common/*.h` + `android/` gradle + `index.d.ts`）见各插件 README。
- 能力声明：插件自己上报能力；缺平台时必须返回"不支持 + 原因"，不允许静默 no-op。

## 2. 22 个插件矩阵

`✓` = 有原生实现；`·` = 只有 Rust（纯 std 实现或降级）；`—` = 无该平台产物；括号内为 `src/*.rs` 文件数。

| 插件 | Rust | Android | iOS | OHOS | Linux | Win/mac |
| --- | --- | --- | --- | --- | --- | --- |
| camera | 11 | ✓ | ✓ | ✓ | ✓ | ✓ |
| webview | 9 | ✓ | ✓ | — | ✓ | ✓ |
| secure-storage | 8 | ✓ | · | — | · | · |
| battery | 7 | ✓ | · | — | · | · |
| open-url | 7 | · | · | — | · | · |
| tray | 6 | — | · | · (17 行占位) | ✓ | ✓ |
| wakelock | 6 | ✓ | · | — | · | · |
| connectivity | 5 | ✓ | · | — | · | · |
| device-info | 5 | ✓ | · | — | · | · |
| local-auth | 5 | ✓ | · | — | · | · |
| location | 5 | ✓ | · | — | · | · |
| paths | 5 | · | · | — | · | · |
| prefs | 5 | ✓ | · | — | · | · |
| share | 5 | ✓ | · | — | · | · |
| file-dialog | 4 | — | · | — | ✓ (zenity→kdialog) | ✓ (NSOpenPanel / IFileDialog) |
| notifications | 4 | ✓ | · | — | · | · |
| package-info | 4 | · | · | — | · | · |
| sensors | 4 | ✓ | · | — | · | · |
| vibration | 4 | ✓ | · | — | · | · |
| background-work | 2 | ✓ | · | — | · | · |
| network-info | 3 | ✓ | · | — | · | · |
| mdns | 1 | — | · | — | · | · |

读法：

- **只有 camera 与 webview 是"全平台原生"**。其余 20 个的原生面主要在 Android（gradle/Java），桌面与 OHOS 走纯 Rust 或降级。
- `tray` 的 OHOS 是 17 行占位（`src/ohos.rs`）+ `unsupported.rs`（31 行）——这是**诚实**的写法，其它插件应照抄。
- `file-dialog` 桌面实现是真的（Linux 走 `zenity` 失败回落 `kdialog`：`floom-plugin-file-dialog/src/linux.rs:1-6`，macos.rs 153 行 / windows.rs 156 行）。注意它与平台层"没有 file-picker trait"的缺口是两件不同的事：插件能开对话框，`floom-platform-desktop` 没有抽象。
- `background-work` 只有 2 个 Rust 文件、收窄到 Android AlarmManager/WorkManager。

## 3. 缺口

| # | 缺口 | 后果 | 修法 |
| --- | --- | --- | --- |
| 1 | ~~没有插件 ABI 版本串~~ **已成立**：`crates/native/floom-native-ffi/src/abi.rs:46` `pub const ABI_VERSION: u32 = 1;`，`FloomNativeProviderV1` 首字段 `abi_version`，`provider.rs:196` 注册时不等即拒，`tests/c_vtable.rs:317` 有负例 | — | 无需再做（本行曾因 grep 参数写错而误报为缺口，2026-11-21 核实） |
| 2 | 桌面原生覆盖不齐（20 个里 18 个无桌面原生） | 桌面应用得自己写能力，插件生态在桌面是半空 | 按使用率排序补齐（secure-storage/prefs/notifications 优先） |
| 3 | OHOS 原生几乎为零（仅 camera） | [platforms.md](platforms.md) 的 OHOS 缺口在插件层再放大一次 | 与 OHOS 后端选型（winit-ohos vs N-API）一起定，不要两次改 |
| 4 | 无插件市场/索引 | 发现成本高，文档即目录 | 1.0 之后；`docs/plugins.md` 这张表就是当前索引 |
| 5 | 无 provider 契约测试套件 | 新平台臂是否被真的调用无法自动验证 | 每个插件至少一条"register → call → 校验参数"的假 provider 测试 |

## 4. 新增插件的 checklist

1. `floom new --template plugin <name>`（模板已存在：`crates/app/floom-cli/src/templates`）。
2. 能力 trait 放 Rust 侧，平台分派在 `src/<platform>.rs`；**不支持就返回原因**，不要 no-op。
3. 原生侧只暴露 C vtable，不放平台类型进 Rust 头文件。
4. `native/README.md` 写清注册点（构造函数 / `JNI_OnLoad` / 模块初始化）。
5. Android 臂要有 gradle 工程 + 清单；iOS/macOS 用 `.m`；OHOS 用 `.cpp`（与 camera 同构）。
6. `just gate plugins` 与 `just gate plugin-template` 必须绿。
7. 在本文的矩阵里加一行——这张表是唯一索引。
