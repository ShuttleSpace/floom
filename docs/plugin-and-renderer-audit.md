# 插件化架构与渲染后端审计规划

审计日期：2026-10-22。范围：需求 1（SDK 只内置最基本平台能力，其余走原生语言插件 + FFI）与需求 2（渲染后端缺口对齐）。

本文是 `docs/backend-readiness-audit.md` 的续篇：前者审计"平台后端是否生产级"，本文审计"插件机制是否成立"与"渲染后端是否对齐"。两者的结论可以合并成一张总计划表。

---

# 第一部分 · 插件化架构审计（需求 1）

## 1.1 目标与现状的差距

要求的形态：

> SDK 只内置最基本的平台能力（窗口、剪贴板、IME）；camera / video / webview / bluetooth 等，
> 像 Flutter 插件那样用原生语言（Java / Swift / C++ / C# / JS / ETS）开发，
> 通过 FFI 与 Rust 运行时交互，保证 SDK 精简。

这个形态在本仓库里**已有相当完整的机制骨架**，但**链路在最后两步断开**。这是关键判断——不是"什么都没有"，而是"最后两步没接上"。

## 1.2 已经具备的机制（有代码证据）

| 机制 | 位置 | 状态 |
|---|---|---|
| 插件注册表 | `crates/floom-platform/src/plugins.rs` | ✅ 完整，8 个测试。函数指针注册（非字符串查找）、幂等 enable、失败不重试 |
| 原生视图 ABI | `crates/floom-native/src/abi.rs` | ✅ 完整，`NativeViewProvider` 9 方法 + 能力协商 + 21 个测试 |
| 合成控制器 | `crates/floom-native/src/controller.rs` | ✅ 生命周期不变量由控制器强制，后端无法私自破坏 |
| 槽位池 | `crates/floom-native/src/pool.rs` | ✅ generation 复位的回收语义 |
| CLI 插件脚手架 | `crates/floom-cli/src/main.rs:295` `create_plugin` | ✅ 生成 `Cargo.toml` + `build.rs` + `src/lib.rs` + `floom.h` + 各平台原生半边 + `example/` + README |
| C ABI 头模板 | `crates/floom-cli/src/templates.rs:504` `plugin_header_rs` | ✅ 4 个符号：`version` / `enable` / `disable` / `capability`，带 ABI version 宏 |
| Kotlin 插件半边 | `crates/floom-cli/src/native_project.rs:874` | ⚠️ 有代码，但契约方向与 C ABI 头相反（见 G3） |
| Swift 插件半边 | `crates/floom-cli/src/native_project.rs:981` | ⚠️ 有代码，但没有导出 C ABI 符号（见 G4） |
| 桌面 CMake 半边 | `crates/floom-cli/src/native_project.rs:1050` | ✅ GTK / AppKit / Win32 三套 |
| 可运行示例 | `examples/floom-native-plugin/src/main.rs` | ✅ 协商 + 生命周期 + 池化三段断言 |

**结论：机制是够用的，缺的是接线与验证。**

## 1.3 缺口清单（按严重度）

### 已修复（2026-10-22，批次 1）

| # | 缺口 | 状态 | 证据 |
|---|---|---|---|
| G1 | `build.rs` 是空操作，原生代码从不被编译或链接 | **已修** | `templates.rs::plugin_build_rs` 现在按目标平台探测 `native/<platform>/`，用 `cc` 编译 `.c` 并发出 `cargo:rustc-cfg=floom_plugin_native`。生成工程的 `target/debug/build/*/output` 里有该 cfg；`nm` 显示 `_floom_sample_enable` 等 4 个符号已在二进制里 |
| G2 | Rust 半边不绑定也不调用 C ABI | **已修** | `templates.rs::plugin_lib_rs` 现在有真正的 `unsafe extern "C"` 块，`version()`/`enable()`/`disable()`/`capability()` 全部走原生。生成工程的测试 `the_version_comes_from_the_native_half` 断言版本号来自原生半边（原生返回 `"sample 0.1.0"`，crate 版本是 `"0.1.0"`）——这是唯一能证明调用真的跨过边界的断言 |
| G3 | Kotlin 半边与 C ABI 头方向相反 | **已修** | `native_project.rs::android_plugin_kotlin` 去掉了 `external fun`，改为 `@JvmStatic` 普通方法，由 `native_project.rs::android_plugin_c_shim`（JNI C shim）调用。方向统一为"原生实现、Rust 调用" |
| G4 | Swift 半边没有实现 C ABI | **已修** | `native_project.rs::ios_plugin_swift` 增加 `@_cdecl("floom_<name>_enable")` 等 4 个导出 |
| G8 | 头文件路径不一致 | **已修** | 统一到 `native/common/floom_<name>.h`；`build.rs` 监视 `native/` 整目录；C 源 `#include "floom_<name>.h"` |

### 仍然开放

| # | 缺口 | 影响 |
|---|---|---|
| G5 | `NativeViewProvider` 是纯 Rust trait，没有 C ABI 外壳 | Swift/Java/C++ 插件**无法参与原生合成**（camera/video/webview 的关键路径）。这是需求 1 最核心的一环 |
| G6 | 仓库内零个真实插件 | 机制已在仓库内通过端到端验证（生成 → 编译 → 测试 → 符号核对），但**仍没有一个真实能力**跑在上面 |
| G7 | `packages/` 住在平台适配器里，违反教义第 5 条 | `floom-platform-mobile/src/packages/` 已有 13 个能力包（3 真 8 stub + 2 全 stub），本应拆成插件 |
| G9 | `Capability` 位里没有 camera/video/webview/bluetooth | 这本身是对的；但要等 G7 完成后，"SDK 精简"才是结构性事实而非口号 |

### 附带修掉的既有问题

- **插件 manifest 缺 `[workspace]`**：在 checkout 内生成插件会让 cargo 向上找到外层 workspace 并拒绝构建。
- **生成的示例工程不编译**：`example/Cargo.toml` 依赖 `floom` 时没有开 `desktop` + `renderer-gpu`，而示例调用的 `run_desktop` 需要这两个 feature。
- **示例 manifest 的注释缩进错乱**：模板里那几行漏了续行转义，生成的 TOML 注释第一行没缩进、其余行有 9 空格。
- **插件的桌面半边被生成为应用 CMake**（`add_executable` + `main.c`）：插件应是库；且桌面半边已由 `build.rs` 用 `cc` 编译，再来一个 CMake 会造成重复符号。已改为只写 `native/desktop/README.md` 说明这件事。

### 新增的固化验证（`crates/floom-cli/src/main.rs` 的 `mod tests`，28 项）

| 测试 | 守住什么 |
|---|---|
| `the_header_and_every_native_half_agree_on_the_symbol_names` | 头、C 半边、Android shim、Swift 半边**四者符号名一致**——这是"两份契约"缺陷（G3/G4）的自动拦截 |
| `the_version_comes_from_the_native_half`（生成工程内） | 证明调用真的跨过了边界，而非返回常量 |
| `the_c_abi_header_is_the_one_the_native_half_includes` | C 源按 `build.rs` 给的 include 路径引用头文件 |
| `the_rust_half_calls_the_abi_behind_the_flag_build_rs_emits` | `extern` 块与 `build.rs` 发出的 cfg 一致 |
| `the_kotlin_half_is_called_by_rather_than_calling_out` | Kotlin 不声明 `external fun`（只看声明，不看散文） |
| `the_swift_half_exports_the_c_abi` | Swift 用 `@_cdecl` 导出 |
| `the_generated_plugin_is_its_own_workspace` | 插件 manifest 有 `[workspace]` 且 `cc` 是 build-dependency |
| `the_native_half_lives_where_build_rs_looks_for_it` | 平台半边落在 `native/<platform>/`，不是根目录 |


## 1.4 结论

| 维度 | 判定 |
|---|---|
| 教义（为什么这样切） | ✅ 清晰、正确，有成本论证 |
| 注册表机制 | ✅ 完整可用 |
| C ABI 契约 | ✅ **已定稿**：头文件是唯一事实源，原生实现、Rust 调用 |
| 原生合成 ABI | ⚠️ 语义完整，但 **Rust-only**（G5） |
| 跨语言 FFI 边界 | ✅ **桌面已打通并验证**；Android/iOS 半边代码已就位但**未在设备上验证** |
| 真实插件验证 | ⚠️ 机制已端到端验证，但**仍无真实能力插件**（G6） |
| SDK 精简的结构性落实 | ❌ **未落实**（G7、G9） |

**一句话**：契约已定稿、桌面链路已打通并验证；剩下的是**原生合成路径的跨语言化**（G5）、
**一个真实能力插件**（G6）、以及**把能力包搬出平台适配器**（G7）。

---

# 第二部分 · 渲染后端缺口审计（需求 2）

## 2.1 审计基准

场景词汇表 = `crates/floom-scene/src/node.rs` 的 `enum SceneNode`（12 个变体）：
`GroupPush/GroupPop`、`Rect{bounds,fill,radius,shadow}`、`Path{path,fill,stroke}`、
`ClipPush/ClipPop`、`FilterPush/FilterPop`、`BackdropFilter{rect,filter}`、
`Shader{bounds,shader}`、`Image{bounds,image,alpha}`、`Text{origin,size,glyphs,color}`。

原则（`docs/renderer-backend-support.md` 引用的 `floom-types::Shadow`）：**降级必须被计数，不得静默**。

## 2.2 覆盖矩阵

| 场景节点 | software | vello (GPU) | skia | web | tui |
|---|---|---|---|---|---|
| `Rect` 填充 | ✅ 绘制 | ✅ 绘制 | ⚠️ 仅记录 | ✅ | ✅ 仅纯色 |
| `Rect` 圆角 | ✅ | ✅ | ⚠️ 仅记录 | ✅ | ❌ 静默忽略 |
| `Rect` 阴影 | ✅ **honor spread** | ⏭️ 计数 `shadows_skipped` | ⚠️ 仅记录 | ✅ | ❌ **静默忽略** |
| `Path` 填充/描边 | ✅ | ✅ | ⚠️ 仅记录 | ✅ | ⏭️ 计数 `skipped` |
| `Image` | ✅ | ✅ | ⚠️ 仅记录 | ✅ | ⏭️ 计数 |
| `Text` | ✅ | ✅ | ⚠️ 仅记录 | ✅ | ✅ |
| `ClipPush/Pop` | ✅ | ✅ | ⚠️ 仅记录 | ✅ | ✅ |
| `GroupPush/Pop` | ✅ | ✅ | ⚠️ 仅记录 | ✅ | ⏭️ 计数 |
| `FilterPush/Pop` | ✅ `filters_drawn` | ⏭️ 计数 `filters_skipped` | ⚠️ 仅记录 | ✅ | ⏭️ 计数 |
| `BackdropFilter` | ✅ `backdrops_drawn` | ⏭️ 计数 | ⚠️ 仅记录 | ✅ | ⏭️ 计数 |
| `Shader` | ⏭️ 计数 `shaders_skipped` | ✅ `shaders_drawn` | ⚠️ 仅记录 | ⏭️ 计数 | ⏭️ 计数 |

图例：✅ 真渲染 · ⏭️ 跳过但计数（符合原则）· ⚠️ 只记录不提交 · ❌ 静默忽略（违反原则）

## 2.3 缺口清单

### R1 · skia 在 2.0 里不是渲染器 🔴

`crates/floom-renderer-skia/src/lib.rs:1`：

```
//! Skia-oriented Scene adapter (command recording; GPU submit stays in legacy crate).
```

`SkiaRendererAdapter` 只有 `commands: Vec<SkiaCommand>` 和 `frames: u64`
（`lib.rs:85-89`），**没有任何 skip 计数器**。后果：

- 它不渲染任何像素；
- 它连"降级被记录"这条原则都不满足——`Shader` / `FilterPush` / `BackdropFilter`
  全都被忠实记录成命令，但没有任何路径会执行它们；
- **没有 `tests/` 目录**（其余四个后端都有）。

按当前状态，它是一个"意图记录器"，却和真正的后端并排放在 `floom-renderer-*` 命名空间里，
容易被误认为已实现。

### R2 · vello 没有 blur primitive，GPU 路径缺三类效果 🟠

`crates/floom-renderer-vello/src/gpu.rs`：

- 阴影：**已修复（B4）** —— `Rect` 的阴影经 `draw_blurred_rounded_rect` 真画出来
  （offset 折进 rect、spread 同时增大 rect 与圆角、blur 折成 σ），`shadows_drawn` 计数。
  `Path` / `GlyphRun` 的阴影仍 `shadows_skipped`——原语只能糊圆角矩形。
- 滤镜域：`gpu.rs` —— 内容照常画（不模糊），`totals.filters_skipped += 1`
- 背景滤镜：`gpu.rs` —— 什么都不画，`stats.filter_skips += 1`

后两者**已核实无法实现**，不是没人做：
① vello 全部 13 个命令无一读回已光栅化像素；② 层机制只做 blend 合成、无后处理钩子；
③ **栅格化只发生一次且在最后**——`BackdropFilter` 在流中段时整个 scene 已编码完，
要兑现它就得把前半光栅化→读回→模糊→画成图像→后半光栅化，而 API 一次只渲染一个 scene。
所以这是**缺一个落脚点**，不是缺一个 blur。software 能做到只因为它自己拥有像素缓冲。

三者都计数了，符合"不静默"原则。

### R3 · software 没有 shader runtime 🟡

`crates/floom-renderer-software/src/lib.rs:1161-1163`：`shaders_skipped += 1`。
已计数，符合原则。WGSL 只在 vello 上跑（`gpu.rs` 的 `shaders_drawn`）。

### R4 · tui 覆盖面最窄，且有 3 处静默忽略 🟡

`crates/floom-renderer-tui/src/grid.rs:216-289`：

- 静默忽略 `Rect.shadow` —— `SceneNode::Rect { bounds, fill, .. }`（`grid.rs:220`），
  `..` 把 `radius` 和 `shadow` 一起吞掉，**不计数**。
- 静默忽略 `Rect.radius` —— 同上，圆角在字符网格上无法表达，但**没有记一笔**。
- 渐变填充被 `grid.rs:221-224` 计数跳过（✅ 合规）。

其余跳过的（Path / Image / Shader / Group / Filter / Backdrop）走 `_ => grid.stats.skipped_one()`（`grid.rs:287`），✅ 合规。

字符网格没有软边缘，阴影与圆角"无法表达"是事实——但按本仓库自己的原则，
**"做不到"也要计数**，否则一个带阴影的卡片在 TUI 上会静默变成无阴影卡片，没人知道。

### R5 · `Renderer` trait 没有能力查询 🟠（架构缺口）

`crates/floom-renderer/src/lib.rs:55-81` 的 `Renderer` trait 只有
`begin_frame` / `render_scene` / `render_scene_with_resources` / `end_frame` / `trim_caches`。

**没有"我支持什么"的查询接口**。每个后端各自暴露自己的 stats
（vello 的 `VelloStats`、software 的 `SoftwareStats`、tui 的 `PaintStats`），
字段名还不一样（`shadows_skipped` vs `shadows_drawn` vs `skipped`）。

后果：host 无法在运行前知道"这个后端会不会把我的阴影丢掉"，
也无法把不同后端的降级归一上报给用户。`Capability` 位只描述**平台**，不描述**渲染后端**。

### R6 · `docs/renderer-backend-support.md` 已过时 🟠

该文档基于 v1 的 `PaintContext::draw_shadow` 默认空实现来写
（`docs/renderer-backend-support.md:12-23`）。但 2.0 里：

- 阴影是 `SceneNode::Rect` 的一个字段，不是 `PaintContext` 的方法；
- `floom-renderer` 的 `Renderer` trait 上**根本没有 `draw_shadow`**；
- `floom-renderer-software` 仍有 `draw_shadow`（`lib.rs:738`），但那是它自己的
  `PaintContext` 实现，不是 ABI 的一部分。

文档里"floom-renderer-web 丢阴影且无计数"这一条**已经不成立**：
2.0 的 `WebPresenter` 内部就是 `SoftwareRenderer`（`crates/floom-renderer-web/src/lib.rs:74,120`），
阴影经软件光栅化后 `put_image_data` 上屏，覆盖率与 software 完全一致。

**文档会误导后续实现者**，必须重写为本节的矩阵。

### R7 · 像素级回归只有 software 有 🟡

| 后端 | tests/ | 像素级断言 |
|---|---|---|
| software | `brush.rs` `filters.rs` `paths.rs` `shadow_spread.rs` `transforms.rs` | ✅ |
| vello | 无 | ❌ |
| web | `web_presenter.rs` | ❌（只断言不崩） |
| skia | 无 | ❌ |
| tui | 无 | ❌ |

macOS 门禁记录过一次真实教训（`docs/gates/macos.md:126-128`）：
"CJK tofu 这个缺陷 CI 抓不到，因为 `floom-snapshot` 的 golden 是 ASCII"。
**只有 software 有像素级纪律**，意味着另外四个后端的渲染正确性没有回归网。

## 2.4 结论

| 后端 | 判定 |
|---|---|
| `floom-renderer-software` | ✅ **参考实现**，唯一有像素级回归的后端 |
| `floom-renderer-vello` | ✅ 真渲染；Rect 阴影已画（B4）；滤镜/背景滤镜已核实无法实现（结构性，非工作量） |
| `floom-renderer-web` | ✅ 继承 software 全覆盖；上限是 CPU 光栅化 |
| `floom-renderer-tui` | ⚠️ 覆盖面最窄；3 处静默忽略违反原则 |
| `floom-renderer-skia` | ❌ **不是渲染器**，只记录命令，无计数器，无测试 |

---

# 第三部分 · 计划表

优先级：🔴 阻塞 / 🟠 高 / 🟡 中 / 🟢 低

## A 组 · 插件与 FFI（需求 1）

| ID | 任务 | 依赖 | 级别 | 状态 | 验收标准 |
|---|---|---|---|---|---|
| **A1** | 统一插件契约方向：C ABI 头为唯一事实源 | — | 🔴 | ✅ **完成** | 头文件明确"原生实现、Rust 调用"；Kotlin 半边改为 `@JvmStatic` + C shim；Swift 半边 `@_cdecl` 导出 |
| **A2** | 实现 `build.rs`：按目标平台编译并链接原生半边 | A1 | 🔴 | ✅ **完成（桌面）** | `nm` 能看到 `floom_<name>_enable` 符号；桌面走 `cc`。**Android/iOS 的构建接线仍待设备验证** |
| **A3** | Rust 半边绑定 C ABI | A1 | 🔴 | ✅ **完成** | `version()` / `enable()` / `disable()` / `capability()` 真实走 `extern "C"`，由生成工程的测试断言 |
| **A4** | 修路径不一致（G8） | — | 🟡 | ✅ **完成** | 头文件统一到 `native/common/floom_<name>.h`；`build.rs` 监视路径、C 源 `#include` 三者一致 |
| **A5** | 给 `NativeViewProvider` 加 C ABI 外壳 | A1 | 🔴 | ✅ **完成** | 新增 `crates/floom-native-ffi`：`FloomNativeProviderV1` C 结构体 + 函数指针表 + `CProvider` 适配层。C 侧量自己的 `sizeof`/`offsetof` 与 Rust 镜像逐项比对；Rust 侧用同形状函数指针表跑通控制器全生命周期 |
| **A10** | 插件支持 web / harmony / tui | A1–A3 | 🟠 | ✅ **完成（构建层）** | 六个平台的原生半边齐备；web(wasm32) 与 ohos 交叉编译通过；设备/浏览器运行未验证 |
| **A6** | 落地第一个真实插件 | A1–A5 | 🔴 | ✅ **完成（macOS）** | `crates/floom-plugin-nativeview`：原生半边是 Objective-C，真实 `NSView` 进真实 `NSWindow`，由框架控制器经 C ABI 驱动。**未覆盖 camera/webview 这类真能力**——需要设备 |
| **A6b** | 插件的 provider 如何到达宿主 | A5 | 🟠 | 🟡 **部分** | 插件的 Rust 半边暴露 `provider(host_window)`，宿主在 `WidgetHost::set_native_provider` 处安装。**注册表**（按 kind 选 provider）未做 |
| **A7** | 把 `floom-platform-mobile/packages/` 迁出为插件 | A5、A6 | 🟠 | ✅ **完成** | 13 个包 → 适配器留 4 个（clipboard/keyboard/safe_area/system_chrome）+ **9 个 `floom-plugin-<capability>/`**；适配器甩掉整棵 iOS 依赖树；架构门禁新增"平台适配器不得依赖插件"规则（已验证会拦） |
| **A8** | 补 SDK 精简的**结构性**约束 | A7 | 🟡 | ✅ **完成** | `Capability` 位集收缩到"每个位背后都有实现"：`OPEN_URL`/`FILE_DIALOG`/`DRM_KMS` 三个无实现的位**删除**（声称它们从此是编译错误）；stub 不再声称软键盘；web 不再声称 `NATIVE_VIEW`；桌面剪贴板跟随实现（只有 macOS 是真 `NSPasteboard`）；桌面 a11y 的 `NATIVE_BRIDGE` 改为随桥的存在派生 |
| **A9** | 插件 ABI 版本协商落地 | A1 | 🟡 | ✅ **完成** | `floom_<name>_abi_version()` 参与 enable 校验，版本不符时拒绝并给出明确错误 |

## B 组 · 渲染后端对齐（需求 2）

| ID | 任务 | 依赖 | 级别 | 验收标准 |
|---|---|---|---|---|
| **B1** | 重写 `docs/renderer-backend-support.md` 为 2.0 场景节点矩阵 | — | 🟠 | ✅ **完成** — 12 变体 × 5 后端矩阵；修正"web 无阴影计数"的过时结论（2.0 的 web 内部就是 software 渲染器）；新增"留下的缺口"一节 |
| **B2** | skia 要么补提交路径 + 计数器 + 测试，要么明确降级为"意图记录器" | — | 🔴 | ✅ **完成** — 改名 `floom-renderer-skia-abi`（**它本来就不是渲染器**：零外部依赖、只被自己的测试用、facade 不导出）；补 `shaders_skipped`/`filters_skipped`/`shadows_skipped`；补 3 项测试（5 → 8） |
| **B3** | tui 补齐静默忽略的计数 | — | 🟡 | ✅ **完成** — `radius` 与 `shadow` 不再被 `..` 吞掉，各计一次；补 2 项测试（21 → 23） |
| **B4** | vello 的 blur primitive | — | 🟠 | ✅ **完成（Rect）** — 上游 vello 0.7 **已有** `draw_blurred_rounded_rect`（之前『依赖上游』的判断是错的，从没人去查过）。offset 折进 rect、spread 同时增大 rect 与圆角（中心不动）、blur 按 `CSS_BLUR_TO_SIGMA` 折成 σ；阴影先于填充画。**Path / GlyphRun 的阴影仍跳过**——原语只能糊圆角矩形 |
| **B5** | 统一后端能力查询 | — | 🟠 | ✅ **完成** — `Renderer::capabilities` 与 `Presenter::capabilities` 返回 `RendererCapabilities{shadows,filters,backdrop_filters,shaders}`；默认**悲观**；五个后端各自诚实回答；`AppRunner::renderer_capabilities()` 透给宿主；5 项测试 |
| **B6** | 统一降级计数的暴露方式 | B5 | 🟡 | ✅ **完成** — `DegradationReport`（`floom-renderer/src/degradation.rs`）是**视图**而非替换：各后端保留自己的 stats，新增 `degradation()` 转换。四个 kind 分开（不合并成一个数——『丢了六件事』和『丢了六个阴影』是不同事实）。`unexpected(caps)` 减去 capabilities 已声明会发生的事，剩下的就是**意外**——把『capability bit 没有实现比没有更糟』变成运行时事实。tui 的单一 `skipped` 无法归因，报告如实承载总数并写明不能归因 |
| **B7** | 为 vello / tui / web 补像素级回归 | — | 🟠 | ✅ **完成（web / tui / software）** — `PixelSnapshot::of_buffer` 让**任何**后端都能有 golden（尺寸进哈希）；web 与 software 参考逐像素比对 + 自己的 golden；tui 的 golden 是**字符网格文本**（人能在 diff 里读懂）；software 补**文本渲染层**证据（墨迹是字形形状而非填充块）。**vello 的 GPU 路径仍无**——需要 GPU 上下文 |
| **B8** | shader 在非 GPU 后端的行为明确化 | — | 🟡 | ✅ **完成** — 文档明确"自定义着色器仅 GPU 后端可用"；software / web / tui / skia-abi 均计数 |

## C 组 · 承接（上一轮的平台后端计划，仍然有效）

| ID | 任务 | 级别 | 备注 |
|---|---|---|---|
| C1 | 修复 `native_appkit.rs:340` 的 `objc2` 编译错误 | 🔴 | `NSArray::iter` 已移除 |
| C2 | 恢复 workspace 构建（widget 迁移断链） | 🔴 | `floom-accuracy` + 多个示例 |
| C3 | 能力位诚实性：`DesktopPlatform` 不再声称未实现的 `CLIPBOARD`/`IME` | 🔴 | |
| C4 | 实现 `OPEN_URL` / `FILE_DIALOG` | 🟠 | A8 后这两个位**已删除**——没有实现的位不该存在。真要做时：加回位 + 实现（它们是插件的活，见 A7） |
| C5 | iOS / Harmony / Web / Embedded 真实后端 | 🟠 | Phase 19 |
| C6 | 各平台门禁补设备验证 | 🟠 | `docs/gates/*` 无 `fail` |

## 执行顺序

```
第一梯队（阻塞，其余都依赖它）
  A1 → A2 → A3            插件链路接通            ✅ 2026-10-22
  A5                      原生合成路径跨语言化      ✅ 2026-10-22
  A6                      第一个真实能力插件        ✅ 2026-10-22
  B2                      skia 定位澄清            ✅ 2026-10-22
  C1, C2, C3              恢复可构建 + 能力诚实     ✅ 2026-10-22

第二梯队
  A7 → A8 → A9            精简 SDK 结构 + ABI 版本  ✅ 2026-10-22
  B1, B5, B6              文档重写 + 能力查询 + 统计归一 ✅ 2026-10-23
  B3, B7                  tui 计数 + 像素回归      ✅ 2026-10-22（vello GPU 路径除外）
  C4, C5, C6              平台后端补齐 + 门禁      ⬜

第三梯队
  B4, B8                  vello blur / shader 行为文档 ✅ 2026-10-23（B8 早已 ✅；B4 = vello 真画阴影）
```

### 批次 1 的实际结果（2026-10-22）

A1 / A2 / A3 / A4 / A9 完成。**插件的 FFI 链路在仓库内打通并验证**：

- `floom create --plugin` 生成的 crate 直接 `cargo test` 通过（5 项）；
- `nm` 显示 4 个 `floom_<name>_*` 符号在二进制里——原生半边确实被编译并链接；
- 生成工程自带的 `the_version_comes_from_the_native_half` 断言版本号来自原生
  （原生返回 `"<name> 0.1.0"`，crate 版本是 `"0.1.0"`）——这是唯一能证明调用
  真的跨过边界的断言；
- 删掉 `native/desktop/` 后插件**仍然编译**、测试仍然全过（host-inert 路径验证）；
- `example/` 对 facade 的 `cargo check` 通过（1m12s，`desktop` + `renderer-gpu`）；
- CLI 新增 8 项生成物结构测试，全套 28 项通过。

**未验证的部分**（明确记录，不当作已完成）：Android 的 JNI C shim 与 iOS 的
`@_cdecl` 导出**没有在设备/模拟器上跑过**——它们只经过了符号名一致性的静态
检查。A2 的验收标准里"Android 走 cargo-ndk/cmake，iOS 走 xcodeframework"
这一条**尚未实现**：目前 `build.rs` 只对 C 平台（桌面/Android shim）用 `cc`，
iOS 分支只是检测已存在的 xcframework 并链接，没有驱动 `xcodebuild`。

### 与本次工作相邻但不属于它的两个失败

1. **`floom-dsl --test compile_fixture`** — DSL 的 `emit.rs` 无条件
   `use floom_widgets::{Checkbox, Radio, Select, Slider, TextField}`，而这些控件
   已随 widget 迁移移进卫星工作区 `crates/ecosystem/floom-ui/`（根 workspace
   `exclude` 了它）。**这是一个设计决策，不该由我单方面替迁移方选**：
   - (a) 收窄 DSL 的 emit 词表（把这几个 kind 降级为已有的 placeholder 机制，
     与 `Grid`/`List`/`Overlay` 同一处理）——约 10 行，代价是 DSL 暂时表达不了
     `checkbox`/`radio`/`select`/`slider`/`text_field`；
   - (b) 让 DSL 的目标 widget 路径可配置（`EmitOptions` 里加一项），默认
     `floom_widgets`，依赖卫星的应用指向 `floom_ui_composites`——保留表达力，
     但要动 API。**推荐 (b)**，因为 DSL 的词表与 UI IR 的 `NodeKind` 仍然一致，
   收窄它会让 IR 和 DSL 再次分叉。
2. **`floom-text` / `floom-types` 的 `line_height`** — 工作树里有未提交的
   在制品（给 `StyledText` 加 `line_height`），是并行的编辑器工作，未纳入本次提交。

## 一句话总结

- **需求 1**：机制（注册表 + ABI + 脚手架）**齐备**，但跨语言 FFI 那一段是空的——
  `build.rs` 不编译、`extern "C"` 不绑定、Kotlin/Swift 与头文件各说各话、`NativeViewProvider` 是 Rust-only。
  补上 A1–A6 即可成立，A7–A9 负责把"SDK 精简"从口号变成结构约束。
- **需求 2**：software 是参考实现，vello/web/tui 基本对齐（降级都有计数），
  **skia 根本不是渲染器**，tui 有 3 处静默忽略，`Renderer` trait 缺能力查询，
  支撑文档基于 v1 API 已过时。B1–B3 是低成本高回报。B4 曾被判定『依赖上游』——**判定错了**，vello 0.7 的 blur 原语一直在，只是从没人去读过上游的 API 清单。教训：写『被 X 阻塞』之前先确认 X 真的不存在。
