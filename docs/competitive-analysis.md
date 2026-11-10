<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# 竞品全景与 floom 工业级 1.0 评估

> 评估对象：**本仓库的 floom（Floom 2.0 workspace）**，不是抽象的 "floem 1.0"。
> 调研日期：2026-11-07。竞品事实来自官网 / GitHub README / GitHub REST API / crates.io REST API 当日直抓；
> floom 事实来自本仓库代码与 `docs/`（附 `文件:行`）。**竞品均未在本机编译或运行**，其"平台覆盖"除特别注明外均为官方声明，非我方实测。
> 六个通用关键指标：`① SDK/平台覆盖` `② 渲染成熟` `③ API 丰富度` `④ 开发便利性` `⑤ 扩展能力` `⑥ 生态基础地基`。
> 外加本库**硬性要求的第七维度**：`⑦ 资源效率与性能`（SDK 体积小 / 运行时 CPU·内存·GPU 占用少 / 渲染·执行性能高），见 §9。

---

## 0. 摘要（结论先行）

1. **floom 的工业级 1.0 卡点不是渲染、不是控件数量、也不是扩展能力，而是"平台实测覆盖"与"开发者表面"。**
   - `⑤ 扩展能力`：22 个能力插件 + 3 套 ABI（Renderer / Platform / Native provider）+ 机械架构门禁，是本次 20 个项目里**最硬**的一档，与 Flutter / Tauri / nsh 同列而纪律更强。
   - `② 渲染成熟`：5 个后端 + `RendererCapabilities` 悲观默认 + `DegradationReport` 计数降级 + GPU 像素级回归测试（断言墨水位置而非计数器），是同类自绘框架里**证据最严**的。
   - `③ API 丰富度`：~120 引擎/数据控件 + 44 composites + 4 套设计系统（306 crate 卫星）+ 完整动画/运动原语，仅缺 BUI 式招牌交互。
   - `① 平台覆盖`：**只有 `aarch64-apple-darwin` 一个目标"tested"**，其余 8 个全部 device-gated / target-gated（`docs/release-policy.md:49-59`）。这是"正式工业级"与"玩具"之间唯一的大门槛。
   - `④ 开发便利`：Chrome 式 devtools、三层 snapshot、accuracy/agent 闭环都在，但缺 `llms.txt`、缺 `init`/`package`、热重载不是文件级。TSX `.d.ts` **已随 `@types/floom` 发布**（`npm/@types-floom/index.d.ts`，898 行，见 §5④），剩余只是占位 emitter 接线。
   - `⑥ 生态地基`：release-policy 治理门禁比任何竞品都细，但全仓仍是 `0.2.0`、卫星未钉版、compat matrix 空白。

2. **没有一家竞品在"Rust 原生自绘保留模式 GUI"这一格构成成熟 1.0 对手。** 最接近的 makepad 发过 `makepad-widgets 1.0.0`（2025-05-13）但官网/llms 不可达；slint 是三许可的 `.slint` DSL + 公司化 1.18.1；flutter 是 Dart 生态；tauri/dioxus 是系统 WebView；leptos 明确搁置了 native rendering；rinf 把渲染外包给 Flutter；xilem/masonry 自称 alpha / toolkit；egui 是即时模式且明说每次发版都有破坏性变更。**这说明 floom 的技术定位有空位，值得用"发布纪律"而非"补功能"去占。**

3. **"工业级可用"的可执行定义**（由 5 分档竞品的共同做法反推，见 §8）：13 条门槛 = 10 条通用 + 3 条本库硬性要求（G11 SDK 体积 / G12 运行时 CPU·内存·GPU / G13 渲染·执行性能，见 §9）。floom 已具备其中 3、8、9 的**资产**，但 9 被 `#[ignore]` 关着；13 的 10% 预算脚本**基线已恢复、能绿能红**（WS-3.1，2026-11-09），仍未进 CI 且整套 20% 靠人工；11 的 10% 体积门禁**已落地、能绿能红**（WS-3.2，2026-11-09），仍缺第 4 档；1/2/4/5/6/10/12/13 是明确的待办。

---

## 1. 关键指标：评分口径（六个通用 + 第七维度）

| 分 | 含义 |
|---|---|
| 5 | 工业级标杆：该维度全面，且有实测矩阵、CI 证据或硬数据支撑 |
| 4 | 强：覆盖面广，个别可指名的缺口 |
| 3 | 可用/中等：有明显但非致命的缺口 |
| 2 | 弱/早期：单一依赖、范围窄或仅声明 |
| 1 | 极窄/缺失/有硬伤（许可证、停滞、封闭） |

**免责**：§4 的分数是基于 §3 已核实事实的**主观推断**，不是任何官方指标；竞品未实测，凡"平台覆盖"高分均为"声明/构建命令"而非我方验证。floom 一列按**当前已落地的可用状态**打分（不是潜力），凡"已具备但未接线/未实测"在 §5 单独说明。

**第七维度 `⑦` 不并入 §4 的六维矩阵**：它的评分口径是"已核实资产 + 是否形成门禁"，且竞品数字多为官方自报、彼此硬件不同口径不可直接相加，故单列于 §9.3。

`①` 特别区分「**声明支持**」与「**逐平台 CI 实测**」：这是 floom 与 flutter/tauri/rinf/nsh 的关键差距，也是 1.0 的定义性指标。

---

## 2. 竞品按技术路线分层（避免苹果比橘子）

| 路线 | 项目 | 共同特征 |
|---|---|---|
| **A. 自绘 GPU/CPU 保留模式** | **floom(本地)**, BUI, rune, makepad, slint, freya, gpui, gpuikit, gpui-kit, xilem, masonry | 自己拥有像素；可跨一致外观；无浏览器依赖 |
| **B. 系统原生控件** | nsh.dev, relm4(GTK4) | 每平台用 OS 自己的 view；IME/无障碍免费继承；外观随系统 |
| **C. WebView / HTML-DOM** | tauri, dioxus, leptos | UI 交给 HTML/CSS；二进制小；渲染能力=浏览器能力 |
| **D. 即时模式** | egui | 每帧重建三角网格；无保留树；易嵌入游戏引擎 |
| **E. 混合/业务逻辑桥** | rinf | Rust 只做业务逻辑，UI 外包给 Flutter |
| **F. 嵌入式 / FFI 绑定** | lvgl.rs, fui(marek-g) | 面向 MCU 或 Qt；绑定层是主体 |

> 关键判读：**A 路线里没有成熟的 1.0 竞品**。B/C 的"平台覆盖/开发便利"高分来自**借宿主**（系统控件、浏览器），与 floom 的自绘路线不等价，不能直接照抄；这些高分应转化为 floom 的**工程基建目标**（实测矩阵、CLI、llms、DevTools），而非架构改动。

---

## 3. 事实台账（20 个项目）

### 3.1 三家原始竞品

| 项目 | 路线/许可/版本 | 平台覆盖 | 渲染 | API | 便利 | 扩展 | 生态/风险 |
|---|---|---|---|---|---|---|---|
| **BUI** | Blender C `kit` 解耦；**GPL-3.0**；1.8k★ / 156 commits / 单人 | Win32, X11, Wayland/Cocoa(实验)；**无移动/Web** | C 端口 CPU + 可选 D3D11/GL；**损伤渲染**(idle 0% CPU)；同像素 CPU/GPU；图片(Wuffs/JPEG/WebP/SVG)+视频管线；~5ms 首帧 | **45** Blender 组件 + **113** HTML 元素 + 现代 CSS(grid 命名区/`oklch`/`:has`/`@layer`/`@container`) + Tailwind v4 带编译期 "did you mean" | 多语言一行装；**tkinter 零改运行**；`bui build app.jsx` → 6.3–6.9MB 单文件(内嵌 QuickJS)，~70ms 首帧/19MB | **6 语言**(Py/RS/JS/React19/Svelte5/C/C++/ImGui) + C ABI | **GPL-3.0 工业禁用**；无 a11y/IME；外观只能像 Blender；C 内核维护负担 |
| **rune** | 纯 Rust 自研；MIT；**pre-release / dev 分支 / 不收 PR**；3.2k★ / Anomaly | macOS/Windows/Linux/Web（声明） | 自研 GPU 渲染 + 自带文本/布局/**弹簧** | `el().key().size().motion().on_press()` builder + signals；rune-kit 组件 | **headless → PNG + dump 元素树 + 脚本化输入**（agent 自验，其头条卖点）；"Give your coding agent this repo" | 封闭（贡献走 issue，agent 修复） | 无 1.0、无版本承诺、单厂商、组件集较小 |
| **nsh.dev** | TSX + 每平台系统原生控件；Apache-2.0；**v0.4.0**；Longbridge | **8 平台**：mac AppKit/Linux GTK4/Win32+Direct2D/Web HTML/iOS UIKit/Android views/TUI ratatui/GPUI | 借系统原生 view → **系统文本输入/IME/无障碍免费继承**；120FPS / ~1ms 首帧 / ~0.12ms tap→commit | `nsh/base` 无样式组件(导航/对话框/菜单/popover/tabs/select/数据表/代码编辑器/富文本内核) + Tailwind；TSX 与 Rust 同一引擎 | **单 `nsh` CLI**：init/run/check/test/package/sign；**~120ms 保存→运行**；TS7 原生 tsc；`nsh test --target ios`；`llms.txt`；perf trace | **签名动态模块**：Ed25519 `.nshpkg` + 离线根证书 + 吊销；每 view 一个 VM(内存/调用时长/节点上限) + 能力授予沙箱；发布后仍可加载新 UI | v0.4.0 年轻；TSX 为中心；Android API28+/iOS15+ 地板；Web 下载 ~1.1MB |

### 3.2 A 路线（自绘，floom 的真正同类）

| 项目 | 许可/版本/主体 | 平台 | 渲染 | API | 便利 | 扩展 | 生态/风险 |
|---|---|---|---|---|---|---|---|
| **floom(本地)** | MIT；**0.2.0**；edition 2024 / MSRV 1.91；本仓库 | **仅 1 个 tested** (macOS 26.6.2/M4 Pro)；其余 8 个 gated | 5 后端(software/vello/web/tui) + `skia_abi` 录音器；`RendererCapabilities` 悲观默认；`DegradationReport` 计数降级；`DamageRegion` ≥8 溢满帧；idle **frames=2/8s**；GPU 像素测试断言墨水位置 | ~60 引擎 + ~19 数据 + 44 composites + 4 设计系统(306 crate)；弹簧/关键帧/重定向/共享元素/WGSL | devtools(Chrome 式) + 三层 snapshot + `floom-accuracy::agent`；CLI 仅 `run`/`create`；TSX `.d.ts` 已发布 `@types/floom`（898 行，`dts_sync` 校验生成器与发布物一致）；**无 llms.txt** | **22 插件独立工作区** + Renderer/Platform/Native provider 3 ABI + provider `AbiMismatch` 强制拒绝 + 机械架构门禁 | release-policy 锁定面 + 6 步发布门禁 + 符合性套件 + 体积/性能/空闲门禁；但全 pre-1.0、卫星未钉版、`flameo` 缺失、embedder 明确不稳定 |
| **makepad** | MIT(repo) / **MIT OR Apache-2.0**(crates.io，口径不一致)；**`makepad-widgets 1.0.0` 2025-05-13**（A 路线唯一 1.0）；7.3k★；Makepad B.V. | wasm/WebGL, osx/Metal, win/DX11, linux/OpenGL；iOS/tvOS/Android/Quest 有构建命令 | 自研 GPU 2D/3D；live-editable design DSL；shader/渐变/阴影/模糊 | scriptable UI DSL；storybook 展示"每个 widget"；`makepad-widgets` ~31.9k LOC | **`cargo-makepad` 最强 CLI**(工具链/wasm/android/`--wasm-opt`/`--brotli`)；**Studio** 运行/检视/迭代 + AI 自动化；storybook；全篇 AI-accelerated 主线 | live DSL 自定义 widget；LLM/生成模型嵌入；运行时脚本 | 官网 `makepad.dev` 传输错误、`www` 仅 "Makepad is loading"、`llms.txt` 404；下载量小(2 万)；许可口径需尽调 |
| **slint** | **三许可**：GPLv3 / **Royalty-free 2.0(专有桌面/移动/Web 免费，需 `AboutSlint` 署名，排除嵌入式)** / Commercial 3.0；**v1.18.1**(2026-09-21)；**SixtyFPS GmbH**；24.4k★ | 桌面三平台(Gallery 截图) + Web 5 个 wasm demo + **逐型号嵌入式板卡**(Toradex/NXP i.MX、ESP32-S3/C3、STM32H7/MP1、RPi 4/Pico、Renesas、Infineon)；<300KiB RAM；no-std；Android/iOS 仅声明 | `.slint` DSL **AOT 编译为机器码** + property 引擎；渲染期三选一 femtovg(OpenGL ES2)/**Skia**/纯 CPU；无 GPU 亦可；可选 Qt 原生风格 | `.slint` 声明式 DSL；`std-widgets`；编译期 codegen 到 Rust/C++/JS(Node Beta)/Python(Beta)；style/theme 非 CSS | **LSP(补全+live preview)** + VS Code 扩展 + `slint-viewer --auto-reload` + SlintPad + **Figma→Slint** + 4 语言模板 + **`slint.dev/llms.txt`** + "Slint and AI MCP" | 4 语言绑定；自定义 component；VST 音频插件嵌入 | **"stable 1.x API… evolve carefully without breaking your code"**；企业落地 SK Signet/WesAudio/OTIV/Viiri；板卡伙伴 Toradex/NXP/RPi；嵌入式专有需按台 royalty(起 $1.00) |
| **freya** | MIT；**0.5.0**(2026-11-05)；3.3k★；**单人** Marc Espin | Linux/Win/mac/**Android**/Web（无 iOS）；官网与 `llms.txt` 平台口径不一致(后者只写 3 个) | **Skia**(Vulkan/OpenGL/Metal/software)；0.4 起自研 reactive core + 增量渲染；富文本(Rope+tree-sitter 代码编辑器)、动画、**shader/渐变/阴影/模糊 + AccessKit** | builder + hooks(`use_state`)；~20 内置组件 + 15+ feature crate(html/router/radio/i18n/markdown/code-editor/material-design/plotters/video/camera/webview/engine/sdk)；主题 token | **agent 友好最佳**：`llms.txt` + **`npx skills add marc2332/freya --skill freya`**；`devtools` feature(实时组件树)；`freya-testing` 无窗口 `render_to_file`；`freya-template` | 最系统的 feature 化；自定义 `Component`；`RenderCallback` 直画 Skia；自写 shader；Blitz HTML；WebView | 0.4 架构断代(弃 Dioxus core)；0.5 未 1.0；Polyfrost/Zerac 使用 |
| **gpui** (Zed) | crate Apache-2.0（zed 仓库 NOASSERTION）；**0.2.2 / pre-1.0**；Zed Industries；zed 仓库 94.1k★ | `gpui_platform`: mac Metal / Linux+FreeBSD wayland|x11 / Win32+DirectWrite；wasm32/iOS 仅 Cargo.toml 代码痕迹 | **hybrid immediate + retained，GPU 加速**；taffy/lyon/resvg/etagere/icu_segmenter/AccessKit；Win/Lin 具体图形 API 未核实 | `Entity` / `Render` / `Element` 三层；`div()` 瑞士军刀 + **Tailwind 式链式** `.flex().gap_3().text_xl().shadow_lg()`；typed Action/Key Dispatch；**核心仅 elements，组件库在外部** | 无 CLI；~30 examples；`[gpui::test]`；文档=README+2 篇 + **"best way to learn… read the Zed source code"**；可选 `inspector`/`hot-reload` feature(成熟度未核实)；无独立 llms.txt | 自定义 Element/View；`gpui_platform`；**"For the near future, GPUI is tied to Zed"** | "There will often be breaking changes between versions"；Zed + Delta 落地；生态靠 gpui-kit/gpuikit 外部补 |
| **gpuikit** (字面项目) | `iamnbutler/gpuikit`；MIT OR Apache-2.0；**0.9.0**；200★；单人 Nate Butler | 桌面随 GPUI + **Web 有实证**(wasm32 + WebGPU via wgpu + trunk，每次 push 构建托管 showcase，需 nightly) | 继承 GPUI hybrid GPU；WebGPU；`runtime_shaders` 运行时编 Metal shader | "SwiftUI ∪ web 组件库"；showcase 逐组件(Table/Slider/Editor/Markdown/Dialog/Popover/Context menu)；33.6k LOC/109 文件；**无数量** | 本地 + 托管 gallery 同一份 `showcase.rs`；docs.rs；`ELEMENT_COVERAGE`；`.tasks/verify`；`llms.txt` 404 | 4 feature；**不 re-export gpui**，应用须自声明 3 个依赖且对齐非官方 `gpui-unofficial 1.14` | **"🚧 Pre-1.0: expect breaking changes in every release. Pin your version."** |
| **gpui-kit** (若指此) | `longbridge/gpui-kit`(原 gpui-component，2026-09-03 改名)；Apache-2.0；0.9.0；**19.8k★**；Longbridge 公司 | 桌面三平台 + WASM + 实验 iOS(`gpui-pre-mobile`)；Android 未核实 | 继承 GPUI；宣称 120FPS；AccessKit | **75+ 组件/原语**；三层 `gpui-kit` / `gpui-base`(无样式行为层) / `gpui-component`(shadcn 风格)；`gpui-shell` JS 扩展运行时("no WebView, no DOM") | story gallery；**`gpui-kit.com/llms.txt`**；每页 `.md`；**AI agent skills**(`npx skills add`，含 design-guides)；headless 集成测试；`inspector` feature | `gpui-shell` 显式能力授权 + `HostModule`；Wry WebView 嵌入；native extensions | **Longbridge Pro 商业桌面应用在用**；285k 下载 |
| **xilem** | Apache-2.0；**0.4.0**(2025-10-29)；5.6k★；Linebender 社区；**自称 "experimental"/"alpha"** | 原生(winit) + web(`xilem_web` 原型/Trunk)；Android `masonry_android_view` "not yet generally usable"；iOS/MCU 未核实 | **原生 = Vello + wgpu + Parley/Fontique + AccessKit**；**Web = HTML-DOM（不是 Vello）**；Imaging 抽象多 2D 渲染器 | 强类型轻量 view 树 + diff 最小化更新保留 widget；`app_logic(&mut State) -> impl WidgetView<State> + use<>`；**12 个 view 原语**(flex/grid/sized_box/split/button/image/portal/progress_bar/prose/text_input/task/zstack) + lens/memoize | `cargo add xilem`；`to_do_mvc` example；docs.rs；`trunk serve`；无 CLI/热重载/inspector；`llms.txt` 404 | `xilem_core` 可承载其他响应式架构；自定义 `WidgetView`；双后端；任意 wgpu 窗口库 | 仅 3 版本/12k 下载；**MSRV Rust 1.96 且 "MSRV 提升不算 breaking change，patch 版也可能升"** |
| **masonry** | Apache-2.0；0.4.0；独立仓库**已归档**(392★，末次 push 2024-05-29)，已并入 xilem 的 `masonry/` | 平台无关 retained widget tree manager；`masonry_winit` + `masonry_android_view`(不可用)；**Web 不走 Masonry** | 保留树 + Imaging(默认 Vello+wgpu) + Parley + AccessKit | **"给框架作者用的 toolkit，不是应用框架"**；根 README: "If you're not sure… you probably want Xilem"；`Widget`/`NewWidget`/`PropertySet`/`AppDriver`；baseline widgets | **F11 widget inspector / F12 布局矩形**；tracing/tracy；`masonry_testing`(`assert_render_snapshot!`/`mouse_click_on`)；无 CLI/docs 站/llms | **最强维度**：运行时检视树、写单测、拆 `masonry_core`/`winit`/`testing` 三 crate 独立用、任意 wgpu 窗口库 | 0.4.0 无稳定性承诺 |

### 3.3 B/C/D/E/F 路线

| 项目 | 许可/版本/主体 | 平台 | 渲染 | API | 便利 | 扩展 | 生态/风险 |
|---|---|---|---|---|---|---|---|
| **flutter** | **BSD-3-Clause**；Google；**Flutter 3.47**；**181k★** | **官方 Supported / CI-tested / Unsupported 矩阵**：Android API24-37(CI 24-36)、iOS15-27(CI 18/26)、Win10/11(x64+arm64)、macOS12-27、Debian10-13/Ubuntu20.04-24.04、Web Chrome/Edge(JS+Wasm)/Firefox/Safari15.6+；**唯一给出逐平台 CI 测试版本**；custom engine embedder(Toyota 车机/LG webOS) | 完全自绘 "control over every pixel"；**Skia + Impeller**(iOS 唯一引擎不可切回；Android API29+ 自 3.27 默认；桌面自 3.47 默认；Web 仍 Skia)；预编译 shader/Metal/Vulkan；成熟文本/动画/fragment shader | Dart widget("everything is a widget") + **Material + Cupertino 双设计系统** + `ThemeData`；StatefulWidget/InheritedWidget/provider；pub.dev "tens of thousands of packages" | `flutter` CLI + `flutter create`；**stateful hot reload 标杆**("Reloaded 1 of 448 libraries in 978ms")；**DevTools 全套**(widget inspector/Performance/CPU/Memory/Network/Debugger/App size…)；**最强 agent**：`docs.flutter.dev/llms.txt` + 每页 "View as Markdown"(`.md`) + `/ai/get-started` + Dart/Flutter MCP server + `package skills` | pub.dev 插件 + platform channels + `dart:ffi`/FFIgen/JNIgen/Pigeon + **add-to-app**(Android/iOS/mac/Web) + platform views + custom engine embedders | `/release/breaking-changes` + `compatibility-policy`；官方 showcase 数十家(Google Pay/Earth、Toyota、BMW、Alibaba、ByteDance、Tencent、eBay、Nubank、LG…) |
| **tauri** | MIT OR Apache-2.0；**v2 稳定 2.12.5 / 3.0.0-alpha.6**；**112.6k★ / 42.6M 下载**；Commons Conservancy | Win10+/macOS10.15+/Linux(webkit2gtk4.1)/iOS-iPadOS9+/Android7+，单一代码库；`tauri-action` 全平台构建证据 | **系统 WebView**(WRY: WKWebView/WebView2/WebKitGTK/Android System WebView) + tao；**~600KB** 二进制、不带 runtime；无自研渲染 | **框架内置 UI 控件 = 0**（全交前端 HTML/JS/CSS） | `create-tauri-app` + `tauri dev/build/info/plugin new`；前端即时 HMR（**改 Rust 代码则窗口关闭重编**）；**全平台 devtools**(WebInspector/Safari/Edge)+CrabNebula DevTools；**最完整 `llms.txt` 体系**(`llms.txt`/`llms-small.txt`/`llms-full.txt`/`guides.txt`/`reference.txt`) | **一等插件**：Cargo crate + 可选 NPM 包 + Android library project + Swift package + 生命周期钩子 + permissions ACL；官方约 30 / 社区约 50；sidecar；Kotlin/Swift 深度集成 | v1→v2 破坏性大版本(独立文档站 + webkit2gtk 4.0→4.1)；1Password/Sourcegraph/Sentry 为**赞助商**(≠采用证据) |
| **dioxus** | MIT OR Apache-2.0；**0.7.10 / 0.8-alpha.3**；39.7k★ / 3.58M 下载；全职小团队(FutureWei/Satellite.im 资助) | Web(WASM DOM/SSR/hydrate，hello ~50kb)、Desktop(<3MB)、Mobile(.ipa/.apk)、SSR/Liveview；宣称可嵌入 Bevy/WGPU/embedded Linux；desktop crate 有 **5 个 headless_tests** | **VDOM + 组件级粗粒度响应式**；桌面/移动 = **系统 WebView**(`wry ^0.53.5` + `tao ^0.34.0`，Cargo.toml 硬核实)；实验 WGPU/Blitz/Freya(Skia) | `rsx!` + `use_signal`；全 HTML 元素/监听器；shadcn/ui 风格 primitives；内置 Tailwind；fullstack(axum/server functions/WS/SSE/SSR/Forms) | **`dx` CLI**(`serve`/`--hotpatch`/`bundle`/`--platform`)；**亚秒级 hot-patching** + 资源热重载；zero-config；**devtools 默认开启**(`wry/devtools` + `dioxus-devtools`) | 自定义 Element；嵌入 Bevy/WGPU；JNI/Objective-C/web-sys；`dioxus-std` + `dioxus-community` | 官网调研时全部抓取失败；0.x 跨小版本迁移 |
| **leptos** | MIT；**0.8.23**；21.5k★ / 5.86M 下载 / 141 版本；社区 org | **Web 原生且仅 Web**；FAQ 明确 **"generic rendering approach … has had to be shelved"**；桌面/移动需借 Tauri | 细粒度响应式，组件只跑一次、直建真实 DOM("no virtual DOM overhead")；无自研 GPU；文本/动画/滤镜由浏览器 CSS | `view!{}` **与**等价函数式 builder 并列；`#[component]`/signals/memos/`Resource`/server functions/router/`<Suspense>`；Tailwind；数十个第三方组件库 | `cargo-leptos`(`new`/`watch`) + 6 个 starter；**模板热重载 + 专用 Leptos language server + VSCode 扩展**；`leptosfmt`；Book；Playground | 自定义组件 + server functions 同构；Axum/Actix/Trunk/Spin/AWS/Cloudflare Workers | "APIs are basically settled… would not expect major breaking changes"，但 0.1→0.4 曾需重写、**至今未 1.0**；`llms.txt` 明确不存在 |
| **relm4** | Apache-2.0 OR MIT；**0.11.0**；2.0k★ / 1.4M 下载；Relm4 组织 | **GTK4/libadwaita → Win/Mac/Linux 桌面**；"runs on bare metal with no additional runtime" | **系统原生控件**；GPU/动画/滤镜明细未核实；有 `css`(`relm4-css`) feature | `view!{}` + `#[relm4::component]`；**Elm 模型**(`Msg`/`update`/`ComponentSender`)；异步任务；`relm4-components`/`icons`/`macros`；10 个 GNOME SDK feature flag | **The Relm4 book** + 模板 + VSCode 片段 + examples；~20 款真实 Flathub 应用(LACT/NixOS Configuration Editor…) | 可复用 `Component`、宏、生态 crate、`css` | 无 CLI/热重载/devtools/llms.txt；README 仍写 `0.10` 略滞后 |
| **rinf** | MIT；**8.11.0**；2.8k★ / 1.54M 下载；Cunarist | **Flutter 全 6 平台 "tested and supported" + CI `test_app.yaml` 链接**（本次调研唯一逐平台声明 tested 并给 CI 的）；eLinux 实验 | **自身不渲染**："Rust for native business logic, Flutter for … GUI"；native FFI 双向流式消息；**"no webviews, web servers, hidden threads"** | Rust 侧无 widget：`#[derive(DartSignal/RustSignal/SignalPiece)]` + serde → 自动生成 Dart 类；actor model + InheritedWidget/provider | `rinf_cli`(template/gen/server/config)；16+ 章文档；**无需浏览器/模拟器即可调试**；Dart hot restart 自动重启 Rust 逻辑；`llms.txt` 404 | FFI 是核心；可引入任意 Rust crate；"hundreds or even thousands of message APIs"；仅 Rust↔Dart | **README 顶部 "Looking for a New Maintainer"**；7→8 破坏性(弃 Protobuf/CLI 改名/Rust 1.85+edition2024)，有官方迁移文档 |
| **egui** | MIT OR Apache-2.0；**0.37.0**(2026-10-15)；**32.1k★ / 30.45M 下载**；单人 Emil Ernerfeldt(Rerun 赞助) | `eframe`: Web/Linux/Mac/Windows/**Android**；核心 "completely platform agnostic"；**无 iOS** | **纯即时模式**("no callbacks")；`epaint` 三角网格 + `egui_glow`/`egui-wgpu`；非目标="Native looking interface"；样式自承 "not yet as powerful as say CSS" | builder(`ui.add(...)`)；~10 类控件 + windows/panels/scroll；`Context::set_style`；**无 RSX/宏 DSL** | `eframe_template` + docs.rs + web demo + `egui_kittest`；**无 CLI/热重载/inspector/llms.txt**；官网是纯 wasm SPA(抓取只得 "Loading…") | 自定义 widget 是核心目标；自写 integration 四步；嵌入 bevy/three-d；`Shape::Callback` 3D 混合；**"If you are not using Rust, egui is not for you"** | **"New releases will have breaking changes… If you want something that doesn't break when you upgrade it, egui isn't for you (yet)"**；Rerun Viewer 落地 |
| **lvgl.rs** (`lvgl/lv_binding_rust`) | MIT；**0.6.2 (2023-04-02)**；952★；**仓库约 21 个月无推送** | **嵌入式专用**，`no_std` by default(16K RAM/64K Flash)；`embedded_graphics` + `lv_drivers`(实验)；仅 pointer 输入；桌面 SDL2 模拟器；Web 仅第三方 demo 链接 | LVGL C 2D + embedded_graphics；**Rust 绑定侧 Images/File system/Tasks 三项未实现**、缺全局 base style、仅基础动画 | **无 DSL**：`lvgl-sys`(bindgen unsafe) + `lvgl-codegen` → 安全包装；README 自承 "still in development… many features need to be exposed" | **摩擦最大**：`DEP_LV_CONFIG_PATH`/`lv_conf.h`、llvm/clang/SDL2、git submodule、nightly `-Z features`、`unsafe_no_autoinit`；无文档站/CLI/热重载/devtools/llms | FFI 是本体；自定义 display/input handler；`lvgl_alloc`；裸机钩子 | 上游 C LVGL 有 30+ widgets/100+ style props/Flexbox/Grid/MIT/LVGL Pro(付费)/NXP-Espressif-Renesas 生态，**但这是上游不等于 Rust 绑定生态** |
| **fui** (歧义，见 §A.1) | `marek-g/rust-fui` MVVM；**GPL-3.0-or-later WITH Classpath-exception-2.0**；**0.19.2**；**89★**；单人 | Linux(x11 & wayland via Qt) + Windows(Qt)；Android/Wasm 未勾选；macOS/iOS 未提 | 保留模式 MVVM(Property/bindings/observable collections)；"renderer agnostic" 但**仅 OpenGL 后端打勾**；**自述 "visual aspect … subject to change. Margins are missing"**；keyboard/AccessKit 未勾选 | `ui!` 宏；6 crates；"extensive styling"；`fui_controls` ~5.8k LOC；控件数未给 | **无 CLI/脚手架/热重载/devtools/官网**；仓库内 `doc/SUMMARY.md` 10 余章 | 自定义控件(`create_view`)；Qt 集成 | 单人、GPL 系、0.19.2、0 open issues |

> 另：用户列举中 **`slint` 出现两次**（已合并）；**`dixous` = dioxus**；**`relm`** 按 `Relm4/relm4`(GTK4) 认定，旧 `antoyo/relm`(GTK3, 2.4k★, MIT) 仅作对照；**`gpuikit`** 与 **`gpui-kit`** 是两个不同项目，两行都列出。详见附录 A。

---

## 4. 六维总评分矩阵（20 项目；第七维度见 §9.3）

分数为基于 §3 事实的**推断**（1–5）。`①` 列标注「实测/声明」：`T`=官方给出逐平台 CI 实测证据，`D`=仅声明或构建命令。粗体 = floom。

| 项目 | 路线 | ① 平台覆盖 | ② 渲染成熟 | ③ API 丰富度 | ④ 开发便利 | ⑤ 扩展能力 | ⑥ 生态地基 | 许可 |
|---|---|---|---|---|---|---|---|---|
| **flutter** | A | **5 T**(唯一逐平台 CI 版本矩阵) | **5** | **5** | **5** | **5** | **5** | BSD-3 |
| **tauri** | C | **5 T** | 3 | 1(内置控件=0) | **5** | **5** | **5** | MIT/Apache-2.0 |
| **nsh.dev** | B | **5 T**(8 平台) | 4(借原生) | 4 | **5** | **5**(签名沙箱) | 3 | Apache-2.0 |
| **slint** | A | **5 T**(含逐型号板卡) | 4 | 4 | 4 | 4 | **5** | **三许可** |
| **freya** | A | 4 D(无 iOS) | **5** | **5** | **5** | **5** | 3 | MIT |
| **BUI** | A | 3 D(无移动/Web) | 4 | **5** | 4 | **5**(6 语言) | 2 | **GPL-3.0** |
| **dioxus** | C | 5 D(+5 headless 测试) | 3 | 4 | **5** | 4 | 4 | MIT/Apache-2.0 |
| **makepad** | A | **5 D** | 4 | 4 | 4 | 3 | **4**(A 路线唯一 1.0) | MIT(口径不一) |
| **floom(本地)** | A | **2**(1 实测 / 8 gated) | **4** | **4** | **3** | **5** | 3 | MIT |
| **egui** | D | 4 D(无 iOS) | 3 | 3 | 3 | 4 | 4 | MIT/Apache-2.0 |
| **rune** | A | 3 D | 4 | 3 | 4 | 2 | 2 | MIT |
| **gpui-kit** | A | 3 D(实验 iOS) | 3 | **5**(75+ 组件) | **5** | 4 | 4 | Apache-2.0 |
| **leptos** | C | 2(Web-only) | 3 | 4 | 4 | 3 | 4 | MIT |
| **relm4** | B | 2(桌面) | 2 | 4 | 3 | 3 | 4 | Apache/MIT |
| **gpui** | A | 2( tied to Zed) | 4 | 3 | 2 | 3 | 3 | Apache-2.0 |
| **xilem** | A | 2(alpha) | 3 | 2 | 2 | 3 | 2 | Apache-2.0 |
| **gpuikit** | A | 2 | 3 | 3 | 2 | 2 | 1 | MIT/Apache-2.0 |
| **masonry** | A | 1 | 3 | 1 | 2 | 4 | 2 | Apache-2.0 |
| **rinf** | E | **5 T** | 2(不渲染) | 2 | 3 | 4 | 2 | MIT |
| **lvgl.rs** | F | 1(嵌入式) | 2 | 2 | 1 | 3 | 1 | MIT |
| **fui** | F | 1 | 2 | 2 | 1 | 2 | 1 | **GPL-3.0** |

**读法**：
- **floom 在 A 路线的横向位置**：`②③⑤` 处于第一梯队（与 flutter/freya/gpui-kit/BUI 同列），`①` 是唯一掉到 2 分的（连 Web-only 的 leptos、封闭的 rune 都在 2 以上有平台声明），`④` 3 分是中游，`⑥` 3 分是"治理最硬但无 1.0"。
- **`①` 是决定性差距**：flutter/tauri/nsh/rinf/slint 的 5 分全部靠"逐平台 CI 实测矩阵"或"每 target headless 测试"，而 floom 是 1 实测 + 8 gated。这是"能不能称工业级"的分水岭，且不靠改架构、只靠补 CI 与门禁。
- **`④` 的 5 分档**（flutter/tauri/nsh/dioxus/freya/gpui-kit）共享三件事：`llms.txt`、单 CLI 覆盖全生命周期、热重载。floom 缺这三件，且**三件都不是硬骨头**。
- **`⑥` 的 5 分档**（flutter/tauri/slint）都过了 1.0/v2 且有稳定版本承诺。floom 治理文档最细，但仍在 0.2.0。

---

## 5. 逐维度对标：floom 的位置、差距与可借鉴项

### ① SDK / 平台覆盖 — floom **2 / 最弱**
- **标杆**：flutter（Supported/CI-tested/Unsupported 三态 + 每平台版本号 + custom embedder 案例）、tauri（`tauri-action` 全平台构建）、rinf（README 明写 "tested" + CI 文件链接）、nsh（8 平台 + `nsh test --target ios`）、slint（逐型号嵌入式板卡 + <300KiB）。
- **floom 现状**：`docs/release-policy.md:49-59` 只有 `aarch64-apple-darwin` "tested"；`x86_64-apple-darwin` untested；Win/Linux device-gated（native-view 合成 Phase 16.2 未完成、无 CI 机器）；Android/iOS device-gated(Phase 17)；OHOS device-gated(Phase 19) 且 `docs/gates/ohos.md` 仅"示例能启动+基础交互"通过，**IME/CJK、无障碍、剪贴板、文件、safe-area、焦点全部 fail**；wasm / no-std target-gated。资产：headless 核心无 OS 依赖、整套测试可无显示运行；每平台已有 `docs/gates/*.md` 门禁文档（多数 fail）。
- **可借鉴**：把 `docs/gates/{android,ios,ohos,linux,windows}.md` 的 fail 项变成 **设备/模拟器 CI 任务**（对标 rinf 的 CI 文件、flutter 的 CI-tested 版本表、tauri 的 `tauri-action`）；在 `release-policy` 的 targets 表加 `to-1.0` 状态列，让"1 实测 → N 实测"可跟踪。

### ② 渲染成熟 — floom **4 / 第一梯队**
- **标杆**：flutter（Skia+Impeller，iOS 唯一引擎、预编译 shader）、freya（Skia 四后端 + 富文本 Rope/tree-sitter + shader/阴影/模糊 + AccessKit + 增量渲染）、BUI（损伤渲染 + CPU/GPU 同像素 + 图片/视频管线）。
- **floom 现状**：5 后端 + `skia_abi` 录音器；`RendererCapabilities` 4 旗悲观默认 `NONE`；`DegradationReport{shadows_skipped,filters_skipped,…}` + `unexpected()` 把"声称却丢弃"判为谎言（`floom-renderer/tests/degradation.rs:84`）；`DamageRegion` ≥8 脏矩形溢满帧；idle `frames=2`/8s、CPU 0.15s；**GPU 像素测试断言"墨水在哪"而非计数器**（`floom-renderer-vello/tests/gpu_pixels.rs:177+`），但**全 `#[ignore]`、需 `FLOOM_REQUIRE_GPU=1`**；vello **backdrop filter 未实现**（实测根因：vello 每次 `render_to_texture` 清目标，两 pass 不合成）；vello 阴影仅矩形；文本按显示密度栅格化 + CJK 回退栈。
- **可借鉴**：floom 的"降级计数 + 悲观能力位"是**比 flutter 更严的诚实性设计**，应把它做成对外文档卖点；把 GPU 像素测试从 `#[ignore]` 提为有卡 CI 上的 PR 级可视化回归（对标 rune 的 headless-PNG、masonry 的 `assert_render_snapshot!`）；补 backdrop filter（已有"根因"结论，方案明确）。

### ③ API 丰富度 — floom **4 / 第一梯队**
- **标杆**：BUI（45 Blender 组件 + 113 HTML + 现代 CSS + Tailwind v4 + 6 种写法）、gpui-kit（75+ 组件 + 三层 + gpui-shell）、flutter（Material+Cupertino + pub.dev 数万包）、freya（builder+hooks + 15+ feature crate + 主题 token）。
- **floom 现状**：~60 引擎控件（Tier-0/controls/compose/navigation/overlay/gesture/focus/canvas）+ ~19 数据控件 + 44 composites + 4 设计系统（306-crate 卫星）；**动画/运动是隐藏强项**（弹簧、关键帧、保留速度重定向、循环往返、隐式过渡、enter/exit、`Crossfade`、`AnimatedContent`、`SharedTransitionLayout`、WGSL shader、`Canvas`/`Spinner`、阴影+模糊+backdrop）。**缺**：BUI 式拖拽擦写数字字段、字段内数学求值 `2*pi`、模糊菜单搜索（`combobox.rs:35` 的 `matching()` 仅大小写不敏感子串）；**无 Tailwind/CSS**。
- **可借鉴**：招牌交互叠在现有控件上是高杠杆可见增益（drag-scrub 数字字段 + 字段求值 + 模糊搜索排序）；样式侧 `floom-style` 已有交互态选择器，可考虑一个 Tailwind 式糖层（BUI/gpui/dioxus/freya/nsh 都证明"链式/Tailwind 表达"是当前主流手感），但不必现在做。

### ④ 开发便利性 — floom **3 / 中游，最易提分**
- **标杆**：nsh（单 CLI + ~120ms + TS7 tsc + `llms.txt`）、flutter（stateful hot reload + DevTools 全套 + 每页 `.md` + MCP）、tauri（最完整 `llms.txt` 体系 + 全平台 devtools + `create-tauri-app`）、dioxus（`dx --hotpatch` + devtools 默认开 + zero-config）、freya（`llms.txt` + `npx skills add`）、slint（LSP + live preview + Figma）、makepad（`cargo-makepad` + Studio 检视迭代 + AI 自动化）、gpui-kit（story gallery + AI skills + headless 测试）、rune（headless PNG 自验）。
- **floom 现状**：Chrome 式 devtools（Elements/Style/Layout + Ctrl+Shift+C 拾取 + F12，隐藏启动）但 Style 行是"声明值非计算值"；三层 snapshot（Scene/Semantics/Pixel）；`floom-accuracy::agent`（screens/evaluate/benchmark/`AgentClient`/`ComponentSchema`）；TSX 能编译运行。**缺**：`llms.txt`（全仓无）、`init`/`package`、多处 emitter 占位未接线、热重载是进程内 id 保留树补丁而非文件监听、devtools 无 hover 高亮。**已补齐（2026-11-07 核实）**：TSX `.d.ts` 主体已发布为 `npm/@types-floom/index.d.ts`（898 行），由 `floom_tsx::generate_dts` 生成，`dts_sync.rs` 校验发布物与生成器不漂移；`948969e2` 修了 component ambient values。P1-3 只剩 emitter 接线与 `floom run` 落地 `.d.ts`。
- **可借鉴**：**补 `llms.txt`（+ 每页 `.md`）是零代码、最高 agent 杠杆**（对标 tauri/flutter/slint/freya/gpui-kit/nsh 全员）；CLI 补 `init`/`check`/`test`/`package`（对标 nsh/tauri/dx/rinf_cli）；`floom run` 文件监听热重载 + 量化延迟（对标 nsh 120ms / dioxus 亚秒）；TSX 按既有计划 `docs/superpowers/plans/2026-11-06-floom-tsx-dts.md` 收尾；Style 行补计算值。

### ⑤ 扩展能力 — floom **5 / 并列最强**
- **标杆**：floom（22 插件独立工作区 + Renderer/Platform/Native provider 3 ABI + provider `AbiMismatch` 强制拒绝 + host-inert + 机械架构门禁）、flutter（platform channels/FFIgen/JNIgen/Pigeon + add-to-app + platform views）、tauri（一等插件 + ACL + Swift/Kotlin + sidecar）、nsh（签名动态模块 + 能力沙箱）、BUI（6 语言绑定 + C ABI）、freya（feature 化 + RenderCallback 直画 Skia）、masonry（拆 3 crate 独立用 + 运行时检视树）。
- **floom 现状**：能力插件走"原生半实现 / Rust 半调用"的 5 函数 C ABI；原生视图 provider 是**版本化 vtable** 且**ABI 大版本不匹配即拒绝**（`floom-native-ffi/src/provider.rs:196`）；`check-architecture-deps.sh` L0–L9 强制向下依赖 + 禁 platform→plugin + 未注册 crate 即失败。**缺口**：per-plugin 的 `abi_version` 宏/Rust 侧比对**未找到**（只有 provider ABI 强制）；webview 的 JS→Rust `addScriptMessageHandler` 缺失；`floom::embedder` 明确 **experimental/不稳定**。
- **可借鉴**：补 per-plugin ABI 版本宏 + Rust 侧比对（对标 provider ABI 的 `AbiMismatch`）；完成 webview JS→Rust 桥。floom 这里的护城河（尤其"降级诚实计数 + ABI 强制拒绝 + 机械门禁"）应**锁定并写进对外故事**，而不是削足适履去模仿竞品。

### ⑥ 生态基础地基 — floom **3 / 治理最硬但无 1.0**
- **标杆**：flutter（181k★、3.47、breaking-changes 政策、数十家企业 showcase）、tauri（v2 稳定、Commons Conservancy、42.6M 下载）、slint（"stable 1.x API… without breaking your code"、企业落地、板卡伙伴）、makepad（**A 路线唯一 1.0**）、egui（30M 下载但明说每次发版破坏性）、dioxus（全职团队 + 机构资助）。
- **floom 现状**：`release-policy` 锁定面（prelude 46 名 / feature bundles / `Widget` / Renderer / Platform / NativeViewProvider / `WidgetHost` / widgets API）+ 6 步发布门禁 + 符合性套件 + 体积/性能/空闲门禁 + `docs_parity.rs` 把文档与树绑定——**这套治理比任何竞品都细**。但：全仓 `0.2.0`；卫星 compat matrix `1.0.0` 行空白；`flameo` 卫星目录缺失；`docs/platform-and-plugins.md` 关于 camera 的描述已过期（插件已存在）；无 `llms.txt`、无 docs 索引；embedder 不稳定。
- **可借鉴**：tag 1.0 + 按 release-policy 锁 ABI；填 compat matrix、补回 `flameo` 作参考 App；写一份 `/breaking-changes` 兼容性政策页（对标 flutter/tauri）；修文档漂移。

---

## 6. 可借鉴清单（按优先级，映射到 floom 具体缺口）

**P0 — 工业级 1.0 的硬门槛**
| # | 动作 | 对标 | floom 落点 |
|---|---|---|---|
| P0-1 | 把 8 个 gated 平台推到 tested：设备/模拟器 CI + 门禁 fail 项清零 | rinf `test_app.yaml`、flutter CI 版本矩阵、tauri `tauri-action`、nsh `test --target ios` | `docs/release-policy.md:49-59` targets 表、`docs/gates/*.md`、`scripts/gate.sh` |
| P0-2 | tag 1.0 + 按 release-policy 锁定 3 ABI + prelude/widgets | slint 稳定 1.x 承诺、tauri v2、makepad 1.0 | `crates/app/floom/tests/{prelude_surface,feature_graph}.rs`、各 ABI 合同套件 |
| P0-3 | 写 `/breaking-changes` + 兼容性政策，填生态 compat matrix | flutter `/release/breaking-changes`、tauri v1→v2 迁移页 | 新增 docs 页、`docs/ecosystem.md:33-41` 空白行 |
| P0-4 | 修文档漂移：补回 `flameo`、更新 camera 过期描述 | — | `docs/platform-and-plugins.md:303`、`docs/ecosystem.md:12` |

**P1 — 开发者表面（高杠杆、可见、多数零/低代码）**
| # | 动作 | 对标 | floom 落点 |
|---|---|---|---|
| P1-1 | 仓库根 `llms.txt`（+ 关键 docs 页 `.md` 版） | tauri 全套 llms、flutter 每页 .md + MCP、slint/freya/gpui-kit/nsh | 仓库根 + `docs/` |
| P1-2 | CLI 补 `floom init` / `check` / `test` / `package` | nsh init/run/check/test/package/sign、tauri dev/build/info、dioxus `dx` | `crates/app/floom-cli/src/main.rs`（现仅 `run`/`create`） |
| P1-3 | TSX 收尾：接线占位 emitter + `floom run` 落地 `.d.ts`/`tsconfig.json` | leptos language server、nsh TSX 工具链、dioxus rsx | `docs/superpowers/plans/2026-11-06-floom-tsx-dts.md` — **主体已于 `58e24f70` / `948969e2` 提交**（`npm/@types-floom/index.d.ts` 898 行 + `dts_sync` 测试），不是"实现未提交" |
| P1-4 | headless→PNG + 元素树 dump CLI；GPU 像素测试提为 PR 级可视化回归 | rune headless PNG、freya `render_to_file`、masonry `assert_render_snapshot!` | 复用 `floom-snapshot` + `floom-inspector` + `floom-accuracy::agent`；`gpu_pixels.rs` 去掉 `#[ignore]`（`FLOOM_REQUIRE_GPU` 于有卡 CI） |
| P1-5 | `floom run` 文件监听热重载 + 量化延迟 | nsh ~120ms、dioxus 亚秒、flutter stateful hot reload、makepad live DSL | 外层接 `floom-hot-reload`（现仅进程内树补丁） |
| P1-6 | 交互质感：拖拽擦写数字字段 + 字段内数学求值 + 模糊搜索排序 | BUI（拖拽/`2*pi`/`"ad cu"`） | 叠在现有 `counter`/`stepper`/`TextInput`；`crates/ecosystem/floom-ui/crates/floom-ui-composites/src/combobox.rs:35` |
| P1-7 | per-plugin `abi_version` 宏 + Rust 侧比对；webview JS→Rust 桥 | provider ABI 的 `AbiMismatch` 模式 | `crates/ecosystem/plugins/*/`，对标 `crates/native/floom-native-ffi/src/provider.rs:196` |
| P1-8 | devtools Style 行补"计算值"、hover 高亮 | Flutter DevTools widget inspector | `docs/devtools.md:224-233` 记录的声明值缺口 |

**P2 — 差异化（1.1+）**
| # | 动作 | 对标 |
|---|---|---|
| P2-1 | 签名动态模块 + 能力沙箱（发布后加载新 UI） | nsh Ed25519 `.nshpkg` + 每-view VM + 能力授予；gpui-kit `gpui-shell` `HostModule` |
| P2-2 | Tailwind/链式样式糖层 | BUI Tailwind v4、gpui 链式、freya 主题 token、dioxus 内置 Tailwind |
| P2-3 | 生态钉版 + 参考应用 | slint 企业案例、makepad storybook |

---

## 7. 别抄错方向（floom 的护城河）

- **别改系统原生控件路线**（nsh/relm4/tauri 的 `①④` 高分来自借宿主）：floom 的"一个自研 GPU 渲染器、跨平台一致品牌外观、无浏览器依赖、headless floor 485,904 B"对品牌一致型工业 App 是**卖点**；照抄 nsh 会丢掉自绘护城河。把它们的分数转成 floom 的**工程基建目标**（实测矩阵/CLI/llms/DevTools），不是架构改动。
- **别学 WebView 路线**（tauri/dioxus 的 `①⑤` 高分来自 wry）：floom 不带浏览器，体积与安全边界不同。
- **别学 VDOM**（dioxus）：floom 细粒度信号 + 保留树 + diff 最小化更新更强。
- **别学即时模式**（egui）：无保留树、无 a11y 语义树、样式能力自承不如 CSS。
- **别为"发个 1.0 tag"而 1.0**：makepad 的 1.0 是 `makepad-widgets` 单 crate 且官网/llms/文档不可达；floom 的 release-policy 门禁比"发个 tag"更硬，应**用门槛定义 1.0**，而不是反过来到处抄数字。
- **别学封闭**（rune 不收 PR / dev 分支）。
- **别引入许可证风险**（BUI/fui 的 GPL、slint 三许可的嵌入式 royalty、makepad 许可口径不一）——见附录 B。
- **别把 306-crate `floom-ui` 纳入 1.0 core promise**：release-policy 已明确 design systems 不锁版本，与 floom 同节奏反而拖累 core。
- 注：BUI 自称 "Twice as good as GPUI" 是其自我陈述，未核实。

---

## 8. 工业级 1.0 门槛（由 5 分档竞品共同做法反推）+ 推进路径

**"工业级可用"的 13 条可执行定义**（√ = 已具备资产；△ = 半成；✗ = 待办；第 11–13 条为本库硬性要求，展开见 §9）：

1. ✗ 逐平台 CI 实测矩阵（非"声明支持"）—— flutter/tauri/rinf/nsh
2. ✗ 稳定版本承诺 + 破坏性变更政策页 —— slint 1.x、flutter、tauri v2、makepad 1.0
3. √ 许可证尽调清晰（floom MIT，本仓为 MIT 是优势）—— 见附录 B
4. ✗ `llms.txt` + agent 工具（skills / MCP / 每页 .md）—— flutter/tauri/slint/freya/gpui-kit/nsh
5. ✗ 单 CLI 覆盖 init/run/check/test/package/sign —— nsh/tauri/dx/flutter/cargo-makepad/rinf_cli
6. ✗ 热重载（文件级 + 量化延迟）—— flutter/nsh/dioxus/leptos/slint
7. △ 全平台 DevTools（floom 已有 Chrome 式，但 Style 行声明值非计算值）—— flutter/tauri/nsh/masonry
8. √ 一等插件 + ABI 版本强制（floom **领先**；仅 per-plugin abi_version 与 webview JS→Rust 待补）
9. △ 可视化回归进 CI（floom 资产最全但 GPU 测试 `#[ignore]`）—— rune/freya/masonry/dioxus
10. ✗ 生态钉版（compat matrix / flameo / 文档索引）—— slint/tauri/flutter
11. △ **SDK / 产物体积尽量小**（floom headless floor 0.5 MiB 已很优；`just gate size` 10% 门禁已落地、能绿能红（WS-3.2，2026-11-09），仍缺 desktop+software 第 4 档）—— BUI 6.3 MB 单文件 / nsh Web 1.1 MB / slint <300 KiB / tauri ~600 KB / makepad CLI 裁剪
12. △ **运行时 CPU / 内存 / GPU 占用尽量少**（idle `frames=2`/8s 已守；无内存/GPU/逐帧分配预算）—— BUI 损伤渲染+稳定帧 0 分配 / nsh +3 MB 内存
13. △ **渲染 / 执行性能尽量高**（11 bench + 10% 脚本 + GPU 像素回归证据最严；基线已恢复、脚本能绿能红，但仍未进 CI、整套 20% 人工、GPU 测试 `#[ignore]`）—— nsh 120FPS/trace / BUI 逐帧基准 / freya 增量渲染

> 第 11–13 条的可执行任务见 [`1.0-launch-milestone.md`](1.0-launch-milestone.md) §4 (WS-3)。**这三条是"把已有资产变成门禁"，不是重做引擎**。

**推进路径**

- **P0（硬门槛，直接决定能否称工业级）**：P0-1 平台实测 → P0-2 锁 ABI + tag 1.0 → P0-3/P0-4 文档与生态收口 → **WS-3.1 恢复 perf 基线（已完成，2026-11-09）/ WS-3.2 体积门禁（已完成，2026-11-09）/ WS-3.3 资源基线（第 11–13 条）**。
- **P1（体验/可验证性，多数低代码、见效快）**：P1-1 `llms.txt`（零代码、最高 agent 杠杆，**建议第一个做**）→ P1-2 CLI 补齐 → P1-4 headless-PNG + GPU 像素测试进 CI → P1-3 TSX 收尾 → P1-5 热重载 → P1-6 招牌交互 → P1-7/P1-8 补 ABI/devtools 细节。
- **P2（1.1+ 差异化）**：P2-1 签名沙箱动态 UI → P2-2 样式糖层 → P2-3 生态钉版 + 参考应用。

---

## 9. 第七维度：资源效率与性能（本库额外硬性要求）

> 本库在六个通用指标之外还有一条**硬性评价要求**：`SDK 体积尽量小`、`运行时 CPU/内存/GPU 占用尽量少`、`渲染/执行性能尽量高`。
> 这一维度主要由 `floom` 自己的 `release-policy.md:89-160` 与本仓实测数字支撑；竞品数字**全部为官方自报**（官网/README），**未在同等硬件上复测**，因此下文用「声明」而非"实测"，且只引用可核实的具体数字。
> 1.0 落地任务见 [`1.0-launch-milestone.md`](1.0-launch-milestone.md) §4 (WS-3) 与 §0 的 G11/G12/G13。

### 9.1 四个子面

| 子面 | 具体指标 | floom 现状（本仓可核实） | 1.0 要求 |
|---|---|---|---|
| **SDK 体积** | 二进制/下载体积、按用量裁剪、默认 feature graph | headless floor **485,904 B (≈0.5 MiB)**；`floom-showcase` 11.9 MiB；`floom-window --features real` 11.1 MiB；`feature_graph.rs` 守"默认不链窗口/GPU/工具"；增长 >10% 已由 `just gate size` **强制**（WS-3.2，3 档 10% 基线已签入、能绿能红） | 补 desktop+software 第 4 档 + 发布记录表 |
| **运行时 CPU** | 空闲帧数、损伤渲染、脏矩形、事件循环 park | `frames=2`/8s、CPU `0.15 s`（0.5% 核）、`about_to_wait` ~75、`DamageRegion` ≥8 溢满帧、`SceneFrameQueue` 有界快照丢最旧帧 | 守住既有门禁，禁止回归；新增资源预算不得破坏 idle 测试 |
| **内存 / GPU** | RSS/峰值、显存、GPU 时间、每帧分配 | `floom-devtools` 有逐帧 `budget_exceeded` + 分配计数 + `PerformanceSnapshot`，但 2.0 计划 §23.3 标 **PARTIAL（app wiring pending）**；`benches/memory_baseline.md` 缺失；**无内存/GPU 预算门禁** | 三类基线 + 阈值 + 强制门禁；接到真实 window loop |
| **渲染 / 执行性能** | 帧时间、bench 回归、golden、按需重绘 | 11 个 Criterion bench；`check-perf-budgets.sh` 管 10% 热基准（best-of-N，能 fail）；整套 20% 人工；GPU 像素测试断言墨水位置。**基线已于 2026-11-09 从 git 历史 `5f4cbcfd` 恢复**，实跑 9 条预算 best-of-2 全绿、改坏 median 能红 | hot-bench 进门禁（按单主机纪律）+ 记录帧预算 |

### 9.2 竞品横向对比（官方自报，未复测）

| 项目 | SDK / 产物体积 | 运行时 CPU / 空闲 | 内存 | 渲染 / 执行性能 | 该维度可借鉴点 |
|---|---|---|---|---|---|
| **BUI** | `bui build` 单文件 **6.3–6.9 MB**（按 `features.json` 裁剪，内嵌 QuickJS）；约 19 MB RAM | **损伤渲染：idle 0% CPU**，hover 只重画碰到的 widget | 图片缓存 256 MiB 有预算 | **稳定帧 0 分配**；1000 缩略图 2.9ms p99 / 100 MB；~5ms 首帧；比 gpui-fast 省 2–4× CPU/帧 | 损伤渲染 + 0 分配稳定帧 + 按用量裁剪，都是"预算化"的范本 |
| **nsh.dev** | Web 下载 **~1.1 MB**；TSX 可执行 19.8 MB(mac)/25.7 MB(lin)/28.6 MB(win)，Rust 版 12.4–16.5 MB；hello 应用代码 <1 KB | 引擎启动 **75–155 µs**；~1ms 首帧；~0.12ms tap→commit；**改动只触碰读了该 signal 的节点** | TSX ~20 MB / Rust ~19 MB / 裸窗 ~17 MB（macOS，仅 +3 MB） | **120 FPS**；22 条 sparkline + order book p95 <4ms；`nsh test --trace` 输出 ops/mount/nodes | "只更新读了该 signal 的节点" + perf trace 可量化，值得抄进 G13 |
| **slint** | runtime **< 300 KiB RAM**（`llms.txt`）；no-std；AOT 编译 `.slint` | property 引擎 + 编译期渲染后端三选一 | 极低（嵌入式导向） | AOT + 可选纯 CPU 渲染；无 GPU 亦可 | 体积/内存的嵌入式级上限意识 |
| **tauri** | **~600 KB** 二进制（用 OS WebView，不打包 runtime） | 前端 HMR | 借系统 WebView（无自报数字） | 由前端/浏览器决定 | "不带 runtime" 的默认体积意识 |
| **dioxus** | Web hello **~50 KB**；桌面 **<3 MB** | VDOM 组件级粗粒度更新（非细粒度） | — | Web wasm 直出 DOM / SSR | 默认 feature 图小体积 |
| **makepad** | `cargo-makepad --strip / --wasm-opt / --brotli / --profile=small / --no-threads` | 自研 GPU 2D/3D | — | GPU 加速 | **按用量裁剪的 CLI 化**（最值得抄给 `floom package`） |
| **egui** | 即时模式，无保留树 | 每帧重建（idle 不省） | — | 三角网格绘制 | 反面参照：无损伤渲染则 CPU 维度吃亏 |
| **freya** | Skia 多后端 | **增量渲染**（0.4 起自研 reactive core） | — | 富文本 Rope + tree-sitter 虚拟化 | 增量渲染 + 虚拟化 |
| **rune** | — | — | — | **自研 springs + headless PNG**；agent 自验 | headless 数值化验证 |
| **flutter** | 引擎自带（较大，无自报数字） | Impeller 预编译 shader | — | **Skia + Impeller**；stateful hot reload ~978ms | 帧预算/性能画像工具（DevTools Performance） |
| **gpui / gpui-kit** | — | gpui-kit 宣称 **120 FPS** | — | hybrid immediate+retained GPU | — |
| **xilem / masonry** | — | view 树 diff 后**最小化更新**保留 widget | — | Vello + wgpu | diff 最小化 = 执行性能设计 |
| **rinf** | 不打包 WebView / 无隐藏线程 | "no webviews, web servers, hidden threads, unnecessary memory copying" | — | 渲染外包 Flutter | 零拷贝/零隐藏线程的开销纪律 |
| **lvgl.rs** | 上游 C：16K RAM / 64K Flash | — | 极低（MCU） | 绑定层不完整 | 资源上限意识（非同等量级，仅参照） |
| **leptos / relm4 / fui / gpuikit / lvgl.rs** | 未给出可核实数字 → **未核实** | 未核实 | 未核实 | 未核实 | — |

**读法**：
- **floom 的资产其实很强**：headless floor 0.5 MiB 与 nsh Web 1.1 MB / dioxus 桌面 <3 MB / BUI 6.3 MB 同一量级或更优；idle `frames=2`/8s + `DamageRegion` 是 BUI "idle 0% CPU" 的可测等价物；11 个 bench + 10% 预算脚本 + GPU 像素回归是**本清单里最严的性能证据**。
- **floom 的缺口是"没把资产变成门禁"**：体积已有 `just gate size`（WS-3.2），但**仍缺 desktop+software 第 4 档**且同样未进合并门禁（单主机纪律）；内存/GPU/逐帧分配**没有预算**；10% 热基准门禁虽已恢复（WS-3.1）但**未进合并门禁**；整套 20% 靠人工；`floom-devtools` 的性能画像停在 headless 协议未接真实窗口。**这些都是工程接线，不是重做引擎**。
- **别抄错方向**：egui 的即时模式（无损伤渲染）、tauri/dioxus 的 WebView 体积（借系统 runtime）、lvgl 的 MCU 上限（不同量级）都不等价于 floom 的自绘桌面 App；可抄的是**预算化 + 按用量裁剪 + 数值化 trace**这三件事。

### 9.3 该维度的评分（1–5，推断）

| 项目 | SDK 体积 | 运行时 CPU/内存/GPU | 渲染/执行性能 | 综合 |
|---|---|---|---|---|
| **floom(本地)** | **4**（0.5 MiB floor + 10% 门禁已落地，WS-3.2） | **3**（idle 已守，内存/GPU 无预算） | **4**（证据最严，10% 门禁已恢复但未进 CI） | **3.7** |
| BUI | 4 | **5**（损伤渲染/0 分配/缓存预算） | **5**（逐帧基准齐全） | 4.7 |
| nsh.dev | **5**（Web 1.1 MB + 分层体积表） | **5**（+3 MB 内存/120FPS/trace） | **5**（p95 <4ms） | **5.0** |
| slint | **5**（<300 KiB RAM） | 4 | 3 | 4.0 |
| tauri | **5**（600 KB） | 3（借 WebView） | 2（框架不管渲染） | 3.3 |
| dioxus | 4（<3 MB / 50 KB） | 3 | 3 | 3.3 |
| makepad | 4（CLI 裁剪） | 3 | 4（自研 GPU 2D/3D） | 3.7 |
| freya | 3 | 4（增量渲染） | 4 | 3.7 |
| flutter | 2（自带引擎） | 4 | **5**（Impeller） | 3.7 |
| egui | 3 | 2（即时模式每帧重建） | 3 | 2.7 |
| xilem / masonry / gpui / gpui-kit / rune / leptos / relm4 / rinf / fui / gpuikit / lvgl.rs | 未核实（多数无可比数字） | 未核实 | 未核实 | n/a |

> 综合分是把"已核实资产"与"是否形成门禁"一起算的：floom 3.7 与 makepad/freya/flutter 同档，**低于 nsh(5.0) 与 BUI(4.7) 的原因不是引擎弱，而是预算没接线**。这正好对应 `1.0-launch-milestone.md` WS-3 的六个任务：恢复基线（已完成）、体积门禁（已完成）、资源基线、接 window loop、CI 策略、发布记录表。

### 9.4 落地映射（→ milestone）

| 本维度发现 | milestone 任务 |
|---|---|
| 基线被归档移出仓库、`check-perf-budgets.sh` 曾直接崩 | **WS-3.1 已完成（2026-11-09）**：从 git 历史 `5f4cbcfd` 恢复 3,140 B 原文件 + `exit 2` 分支，实跑能绿能红 |
| 体积曾只记录不强制、无 per-bundle 预算 | **WS-3.2 已完成（2026-11-09）**：`just gate size` + 已签入 3 档 10% 基线，能绿能红；第 4 档待补 |
| 无内存/GPU/逐帧分配预算；`benches/memory_baseline.md` 缺失 | WS-3.3 `just gate resources` |
| `floom-devtools` 性能画像 PARTIAL（app wiring pending） | WS-3.4 接 `floom-window --features real` |
| "不进 CI 是有意的"单主机纪律 | WS-3.5 固定参考机 + SKIP≠PASS |
| 发布只记体积/空闲 | WS-3.6 六类数字表 |

---

## 附录 A：名称歧义认定与预设修正

| 线索 | 认定 | 说明 |
|---|---|---|
| `dixous` | **dioxus**（`DioxusLabs/dioxus`） | 用户拼写 |
| `relm` | **Relm4**（`Relm4/relm4`，GTK4） | 旧 `antoyo/relm`(GTK3, 2.4k★, MIT) 仅作对照；其 README 抓取失败，"已废弃"未核实 |
| `lvgl.rs` | `lvgl/lv_binding_rust`（crate `lvgl`） | `rafaelcaricio/lvgl-rs` 是同仓库旧路径（API 同 id）；**上游 C `lvgl/lvgl` 不是评分对象** |
| `fui` | `marek-g/rust-fui`（MVVM GUI） | **歧义**：crates.io 的 `fui`(`xliiv/fui`) 是 cursive 系 **TUI 表单库**(MIT, 2.1.0, 2022 停更)；若原意指后者，应归入"终端"格。**待确认** |
| `gpuikit` | `iamnbutler/gpuikit`（字面项目，**不是** gpui-kit 的笔误） | 其 README 明写 "gpuikit is not gpui-kit"；命名史 vitesse→gpui-kit(2024-05)→gpuikit(2025-11-14)。矩阵两行都列（含 `longbridge/gpui-kit`） |
| `gpui-kit` | `longbridge/gpui-kit`（原 gpui-component，2026-09-03 改名） | `huacnlee/gpui-component` 与 `longbridge/gpui-component` 都 302 到同一 repo；GitHub topic `gpui-kit` 有 70 仓库、star 最高者是 BUI 自己 |

**三处需修正的直觉**：
1. **Slint 不是 "GPLv3/商业双许可"**：核实为**三许可**——GPLv3 / **Royalty-free 2.0（专有桌面/移动/Web 免费，需展示 `AboutSlint` 或徽标，明确排除嵌入式）** / Commercial 3.0（专有含嵌入式）；文档示例 MIT。嵌入式专有才必须买商业许可（每台 royalty 起 $1.00）。
2. **xilem 的 Web 后端是 HTML-DOM**（`xilem_web` 用 DOM 元素），**不是 Vello+wgpu**；Vello 只用于原生（Masonry）后端。
3. **`gpuikit` 与 `gpui-kit` 是不同项目**；`gpui` 单 crate 0.2.2 的生态依赖外部 kit，两者不应合并计分。

## 附录 B：许可证与尽调风险

| 类别 | 项目 | 说明 |
|---|---|---|
| **安全（MIT / Apache-2.0 / BSD）** | **floom(MIT)**、egui、dioxus、leptos、rinf、rune、freya、lvgl.rs、relm4、masonry、xilem、gpuikit、nsh、gpui-kit、gpui(crate)、flutter(BSD-3)、makepad(repo MIT) | floom 的 MIT 在本清单中属最宽松一档，是 1.0 的**优势**而非风险 |
| **需尽调** | slint | 三许可；专有桌面/移动/Web 免费但要署名，**嵌入式专有收费** |
| **需尽调** | makepad | repo `LICENSE` = MIT(署 Makepad B.V.)，crates.io 元数据 = MIT OR Apache-2.0，口径不一致 |
| **需尽调** | gpui | crate Apache-2.0，但 zed 仓库整体 license 字段 NOASSERTION |
| **工业禁用** | BUI | **GPL-3.0** |
| **工业禁用** | fui(marek-g) | **GPL-3.0-or-later WITH Classpath-exception-2.0** |
| **维护风险** | rinf | README 顶部 "Looking for a New Maintainer" |
| **维护风险** | lvgl.rs | crate 停在 0.6.2(2023)，仓库约 21 个月未推送 |
| **封闭/不稳定承诺** | rune(不收 PR)、gpuikit(每版破坏性)、egui(每版破坏性)、gpui(频繁破坏性且 tied to Zed)、xilem(MSRV 提升不算 breaking) | 采用前需评估升级策略 |

## 附录 C：证据标准与未核实项

- **竞品未实测**：所有"平台覆盖"除注明 `T`（官方 CI/测试证据）外均为官方声明或构建命令；`②` 中"成熟度"多为官方定性描述，无第三方量化基准。
- **竞品抓取失败/不可达**：`makepad.dev`（传输错误）、`www.makepad.dev`（仅 JS 壳）、`makepad.dev/llms.txt`/`egui.rs/llms.txt`/`relm4.org/llms.txt`/`freya`(存在)/`dioxuslabs.com`(超时)/`leptos.dev/llms.txt`(404)/`book.leptos.dev/llms.txt`(404)/`cunarist.github.io/rinf/llms.txt`(404)/`gpui.rs/llms.txt`(返回首页)/`nate.rip/gpuikit/llms.txt`(404)/`xilem.dev/llms.txt`(404)/`slint.dev/llms.txt`(**存在**)。**结论**：竞品中确认有 `llms.txt` 的为 tauri、flutter、slint、freya、gpui-kit、nsh；确认无的为 leptos、egui、relm4、rinf、gpuikit、xilem、makepad；dioxus/gpui 未核实。
- **各竞品普遍未核实**：正式企业采用（除 flutter/tauri/slint/makepad/egui 有具名案例或赞助方外）、逐平台运行时实测、破坏性变更政策的书面承诺（除 egui/flutter/tauri/slint/makepad）。
- **floom 事实全部来自本仓库**：`docs/{release-policy,architecture,renderer-backend-support,platform-and-plugins,embedder,ecosystem,devtools,motion-gap,examples}.md`、`docs/gates/*.md`、各 crate 测试（`contract.rs`/`capabilities.rs`/`degradation.rs`/`render_handoff.rs`/`gpu_pixels.rs`/`contracts.rs`/`lifecycle.rs`/`accessibility.rs`/`winit_capabilities.rs`/`negotiation.rs`）、`scripts/{check-architecture-deps,check-plugin-templates,run-golden-examples}.sh`、`crates/app/floom/tests/{prelude_surface,feature_graph,docs_parity}.rs`、`CHANGELOG.md`，均附 `文件:行`。

## 附录 D：主要来源（竞品）

- BUI: github.com/ppmpreetham/BUI · bui.preetham.top
- rune: github.com/anomalyco/rune
- nsh.dev: nsh.dev · github.com/longbridge/nsh
- slint: slint.dev(/pricing,/supported-boards.html,/llms.txt) · github.com/slint-ui/slint
- flutter: flutter.dev · docs.flutter.dev(/reference/supported-platforms,/tools/hot-reload,/llms.txt) · github.com/flutter/flutter
- egui: egui.rs · github.com/emilk/egui
- xilem/masonry: xilem.dev · github.com/linebender/xilem · github.com/linebender/masonry
- dioxus: dioxuslabs.com(不可达) · github.com/DioxusLabs/dioxus
- freya: freyaui.dev · github.com/marc2332/freya
- fui: github.com/marek-g/rust-fui · crates.io/crates/fui
- gpui: gpui.rs · github.com/zed-industries/zed (crates/gpui)
- gpuikit: github.com/iamnbutler/gpuikit · nate.rip/gpuikit
- gpui-kit: gpui-kit.com · github.com/longbridge/gpui-kit
- leptos: leptos.dev · github.com/leptos-rs/leptos
- lvgl.rs: github.com/lvgl/lv_binding_rust · crates.io/crates/lvgl
- makepad: makepad.dev(不可达) · github.com/makepad/makepad
- relm4: relm4.org · github.com/Relm4/relm4
- rinf: cunarist.github.io/rinf · github.com/cunarist/rinf
- tauri: tauri.app(/v2) · v2.tauri.app(/llms.txt) · github.com/tauri-apps/tauri
- 版本/star/下载量：`api.github.com/repos/*` 与 `crates.io/api/v1/crates/*`（2026-11-07 快照）
