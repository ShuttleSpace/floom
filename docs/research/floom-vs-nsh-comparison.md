# floom（本地项目）vs longbridge/nsh 深度对比分析

> **修订说明**：本文档修正了第一版分析中的三处事实错误：
> 1. ❌ 旧版称 floom "无自定义 CLI" → ✅ floom **已有 `floom-cli`**，可 `floom create` 生成 Flutter 风格脚手架
> 2. ❌ 旧版未提及 floom 对 Flutter 的参考 → ✅ floom **大量参考 Flutter**（插件迁移目录、插件 ABI 设计、CLI 脚手架结构均对标 Flutter）
> 3. ❌ 旧版认为 floom "无 TSX 支持" 是缺陷 → ✅ 正确定位应为：floom **可借鉴 nsh 的 TSX 开发模式**，但渲染与打包仍由 floom 的 Rust 平台处理（区别于 nsh 的原生 view 渲染）

---

## 1. 项目总览

| 维度 | floom（本地 lapce/floom） | longbridge/nsh |
|------|--------------------------|----------------|
| **定位** | 高性能、细粒度反应式的 Rust 原生 UI 库（lapce 编辑器同源） | "Build once, native everywhere" 跨平台原生 UI 框架（长桥出品） |
| **版本状态** | 2.0 分层架构，活跃开发中 | 0.1.1 初始发布 |
| **Stars** | 内部 workspace | 927 ★ / 71 Forks |
| **反应模型** | leptos_reactive 启发：`RwSignal`/`Scope`/`Effect`/`Memo`（在 `floom-signal`） | SolidJS 启发：`signal`/`memo`/`effect`/`owner`/`context` |
| **渲染方式** | **自定义渲染器**：vello+wgpu (GPU)、tiny-skia (软件)、TUI、WASM | **平台原生 view**：UIKit、AppKit、Win32、GTK4、HTML/CSS、GPUI |
| **开发语言** | Rust 为主；**已有 DSL + codegen**；可借鉴 nsh 引入 TSX | Rust + **TSX**（同一反应引擎，TSX 编译为 Rust 声明或运行时执行） |
| **CLI 工具** | ✅ `floom create` / `floom create --plugin`（Flutter 风格脚手架） | `nsh init` / `nsh build` / `nsh package` / `nsh run` |
| **设计系统** | 主题类 + Fluent i18n；`floom-material`/`floom-cupertino`/`floom-fluent`/`floom-harmony` 设计套件（在 floom-ui 卫星仓库） | 主题令牌 (token) + Tailwind 类实用工具 (`nsh/base`) |
| **对标参考** | **Flutter**（插件迁移、插件 ABI、脚手架）+ Xilem/Leptos/rui | GPUI、GPUI Kit、shadcn/ui、SolidJS |
| **平台** | Windows/macOS/Linux (winit+vello)、Android/iOS/HarmonyOS、Web/WASM、TUI、Embedded | iOS/Android/macOS/Windows/Linux/Web/GPUI |
| **嵌入** | `floom::embedder` API + `floom-native` 原生 view 组合（能力协商/槽位/池化） | `NshAppView`/`NshView` 嵌入现有原生 app |
| **打包发行** | Cargo feature 切面 + `just build <crate> <platform>` 统一入口 + 各平台原生 host 项目 | 签名包 (signed packages)、热 TSX 更新、sandbox |
| **示例** | `floom-showcase`/`floom-window`/`floom-macos`/`floom-android`/`floom-native-plugin`/`floom-probe` | `gallery`/`gallery-rs`/`counter`/`desktop`/`embed-*`/`bench`/`hopper` |

---

## 2. 核心相似点（确认无变化）

1. **Rust 原生 GUI 框架**，均避免 JS↔Rust FFI 开销。
2. **细粒度反应式**：两者都以 signal 为核心驱动 UI 更新（floom 走 leptos 路线，nsh 走 SolidJS 路线）。
3. **跨平台单一代码库**：覆盖桌面/移动/Web。
4. **组件库 + 布局（flexbox/grid）+ 主题** 均具备。
5. **开发者体验目标一致**：prelude、示例、易用 API。
6. **都参考成熟框架**：floom 参考 Flutter + Xilem/Leptos；nsh 参考 GPUI + SolidJS + shadcn。

---

## 3. 关键差异（修正版）

### 3.1 渲染哲学（根本差异）

| floom | nsh |
|-------|-----|
| 自有渲染器：GPU (vello/wgpu)、软件 (tiny-skia)、TUI、WASM 后端 | 平台原生 view：UIKit/AppKit/Win32/GTK4/HTML-CSS/GPUI |
| **优势**：跨平台外观一致、完全可控、GPU 性能、可 pixel-snapshot 做 golden 测试 | **优势**：天然符合平台外观/交互（菜单在桌面是弹窗、手机是 sheet） |
| **代价**：原生手感需自绘；无障碍/IME 需逐平台对接 | **代价**：非标准外观需手动绘制；跨平台像素级一致性难保证 |

### 3.2 CLI 工具链（**修正**：floom 已有**）

- **floom-cli（已存在）**：
  - `floom create myapp` → 生成 **Flutter 风格**项目：Rust core 在 `src/`，每个目标平台一个目录 (`android/ios/ohos/web/windows/linux/macos/tui`)
  - `floom create --plugin` → 生成插件脚手架：Rust 能力 + 各平台原生半边 (C/Swift/Java/ArkTS/JS)，通过 `extern "C"` ABI 链接
  - 支持 `--platforms`、`--org`、`--floom-path`、`--force` 等选项
  - 无第三方依赖（仅 `std`），可机器上仅 `floom create` 就能构建
  - 28 项固化测试验证生成物结构（C ABI 符号一致性、Android JNI 契约、Swift `@_cdecl`、Web JS import 等）
- **与 nsh 对比**：nsh 的 `nsh` CLI 覆盖 init/build/package/run 全链路 + 热 TSX 更新；floom-cli 目前覆盖 create 脚手架（含 app 与 plugin 两套），尚未覆盖 build/package 的跨平台统一入口（但 `just build` 脚本补足了 build 段）。

### 3.3 对 Flutter 的参考（**新增**：floom 明确对标 Flutter**）

floom 在多处明确以 Flutter 为参考系：
- **`docs/plugin-catalog.md`**：《平台插件迁移目录（Flutter → Floom）》——逐条盘点 pub.dev 官方发布者 90 个插件包，结论是「迁什么、不迁什么、按什么顺序迁」，并受三条硬约束（一能力一 crate、host-inert 诚实性、无实现位不存在）。
- **`docs/platform-and-plugins.md`**：插件 ABI 设计直接对标 Flutter 的 federated plugin 模型（Java/Swift 原生半边 + 窄 ABI + `build.rs` 链接）。
- **`crates/floom-cli` README**：明确称脚手架为 "Flutter-style project"，`android/` 紧贴 `src/` 而不在 `native/` 下，与 Flutter 项目结构一致。
- **`examples/floom-android` 等**：Android 的 `native-activity` + Gradle 结构与 Flutter Android 集成模式一致。

### 3.4 开发语言与 TSX（**修正**：floom 可借鉴 nsh 的 TSX**）

- **nsh 的 TSX 模式**：
  - 应用可用 Rust、TSX、或混合编写；同一反应引擎驱动两端。
  - TSX 屏幕可在**运行时**以签名包形式加载（host 控制的 sandbox 能力）。
  - 构建期生成 TypeScript 声明到 `.nsh/types`，编辑器可感知。
  - 热 TSX 更新保留状态（state-preserving hot update）。
  - `examples/rust-exports` 演示 TSX 应用如何使用自有的 Rust crate 组件/模块/类。
  - 底层依赖 `nsh-js`（TSX/JS 运行时，QuickJS 类引擎）执行 TSX 模块。
- **floom 目前状态**：
  - 有 **`floom-dsl`**（自定义声明式 DSL：`source text → UI IR → Rust builder program`，手写词法/语法分析，自带 span 诊断）和 **`floom-codegen`**（将 `ComponentSchema` 生成 Swift/Kotlin/C/C++ 原生胶水）。
  - **尚无 TSX 支持**，但已具备借鉴所需的地基：
    - `floom-ui-ir` 已是「单一属性表」——DSL 与 codegen 都 lower 到它，TSX 也可 lower 到它。
    - `floom-dsl` 的三段式架构（parse → lower → emit）可直接复用于 TSX → IR → Rust。
- **用户指明的借鉴方向**（关键）：
  > floom 可借鉴 nsh 的 TSX：**用 TSX 开发项目，但渲染和打包都由 floom 的 Rust 平台处理**。
  - 即：TSX 作为 UI 描述语言（开发体验），但**不走 nsh 的「原生 view 渲染」**，而是走 floom 自有的渲染器（vello/wgpu/tiny-skia/TUI/WASM）。
  - 这保留了 floom 的跨平台像素一致性 + GPU 性能优势，同时获得 TSX/JSX 的开发愉悦度与 AI 生成 UI 的便利性。

### 3.5 设计系统 / 主题

| floom | nsh |
|-------|-----|
| 主题类 + `floom-material`/`floom-cupertino`/`floom-fluent`/`floom-harmony` 设计套件（floom-ui 卫星仓库，~350 crate） | `nsh/base` 无样式基础组件 + 主题令牌 (token) + Tailwind 类实用工具 |
| 设计套件 emit `StyleProps`（不绘制） | 设计令牌可导出/导入，跨平台一致 |
| Fluent i18n | 手动字符串处理（较轻） |

### 3.6 嵌入与运行时模块

| floom | nsh |
|-------|-----|
| `floom-native`：原生 view 组合（能力协商、槽位、池化）；`floom-native-ffi`：C ABI (Swift/Java/C++/ArkTS provider) | `NshAppView`/`NshView` 嵌入 iOS/Android/macOS；运行时加载签名 TSX 包 |
| 插件体系：`floom-plugin-*` 一能力一 crate，host-inert | 运行时 TSX 模块 + sandbox |

### 3.7 打包与发行

| floom | nsh |
|-------|-----|
| Cargo feature 切面 + `just build <crate> <platform>` 统一入口（Android→build-apk.sh，iOS→ios-bundle.sh，OHOS→hvigor Stage） | `nsh package` 签名包；热 TSX 更新 |
| 各平台 host 项目由 `floom-cli` 生成（`android/` 为 Gradle，`ios/` 为 Xcode pbxproj，`web/` 为 `index.html`） | 静态站（web）、独立 app、嵌入现有 app |

---

## 4. floom 可借鉴 nsh 的具体方向（按可行性排序）

### 4.1 🔥 最高优先级：TSX 开发模式（渲染/打包仍由 floom Rust 平台）

**为什么是最高优先级**：用户明确点名，且 floom 已有地基（`floom-ui-ir` + `floom-dsl` + `floom-codegen`），缺口主要在「TSX 前端」。

**可行路径**：
1. **frontend parser**：新增 `floom-tsx` crate，将 `.tsx` 解析为 `floom-ui-ir::UiDocument`（复用 `floom-dsl` 的 `lower`→`emit` 模式，但前端用 TSX 语法而非自定义 DSL）。
   - 借鉴 nsh：TSX 编译期生成 `d.ts` 声明，编辑器可感知 props/事件。
2. **两种运行模式**（借鉴 nsh 的「编译 vs 运行时」双轨）：
   - **编译期**：TSX → `floom-ui-ir` → Rust builder program（走 `floom-codegen` 的 emit 后端），最终由 floom 渲染器绘制 → **满足「渲染打包都由 floom Rust 平台处理」**。
   - **运行时（可选）**：TSX → 轻量 VM（如 QuickJS/nsh-js 类）执行，动态挂载到 `WidgetHost` → 支持 AI 生成 UI / 热更新（这正是 nsh 的签名包 + sandbox 场景）。
3. **共享 Rust 组件给 TSX**：参考 `examples/rust-exports`——Rust 组件/模块/类可被 TSX 引用，类型通过 `floom-ui-ir` schema 桥接。

**与 nsh 的本质区别（必须守住）**：
- nsh 的 TSX 最终渲染为**原生 view**（UIKit/AppKit/...）；floom 的 TSX 应渲染为 **floom 自有渲染器**的 scene graph。
- 这保留了 floom 的 GPU 性能 + 跨平台像素一致性 + pixel-snapshot golden 测试能力。

### 4.2 高优先级：设计令牌系统 + Tailwind 类实用工具

- 借鉴 nsh 的 `nsh/base` + 令牌 + Tailwind 类：在 `floom-style` 上引入令牌类型 + 类实用工具 (`text-primary`, `bg-surface`, `rounded-md`)。
- 复用 `floom-ui` 卫星仓库已有的 `floom-material`/`floom-cupertino`/`floom-fluent`/`floom-harmony`——把它们改为 emit 令牌而非硬编码 `StyleProps`。

### 4.3 高优先级：CLI build/package 补全

- `floom-cli` 已覆盖 `create`（app + plugin）。借鉴 nsh 的 `nsh build`/`nsh package`/`nsh run`：
  - `floom build --platform <p>` 包装现有 `just build` 脚本，给出统一结果行（`TIER=... PLATFORM=... CRATE=... ARTIFACT=...`）。
  - `floom package` 生成对应平台发行物（APK/IPA/HAP/静态站/独立 exe）。
  - `floom run --watch` 热重载（借鉴 nsh 热 TSX 更新；floom 已有 `floom-hot-reload` crate 可在此基础上扩展）。

### 4.4 中优先级：运行时 UI 加载（AI 生成屏幕）

- 借鉴 nsh 的签名包 + sandbox：让 floom 的 `floom-agent`/模块系统支持运行时加载 TSX/IR 定义（来自磁盘/网络），保留状态。
- floom 已有 `floom-agent` crate（inspect/tap/type/screenshot over channel），可作为 AI 生成 UI 的控制面。

### 4.5 中优先级：反应模型模式借鉴

- 研究 nsh 的 SolidJS 启发模型（`signal`/`memo`/`effect`/`owner`/`context`），特别是 owner-based 生命周期管理，看是否能精炼 `floom-signal` 的细粒度更新。

### 4.6 低优先级：GPUI 作为可选后端

- nsh 的 GPUI 后端演示了快速 GPU 方案。floom 已有 vello/wgpu，无需引入 GPUI，但可参考其「嵌入 GPUI app」模式（floom 已有 `floom-embedded` 平台层）。

---

## 5. nsh 可借鉴 floom 的 reciprocity

| floom 强项 | nsh 潜在收益 |
|-----------|-------------|
| GPU 渲染器（vello/wgpu、tiny-skia、TUI、WASM） | 当前 nsh 后端为原生 view（CPU/原生）；引入 GPU 渲染器可提升 Web/跨平台性能 |
| ~80 crate 精细 feature-gating | nsh 可模块化 crate 结构，按需引入 |
| Inspector/Devtools（F12 元素检查器） | nsh 可加类似 devtools 管道调试布局/信号/状态 |
| Taffy flexbox/grid 布局 | nsh 布局可改用 Taffy 获得跨平台一致布局 |
| Fluent i18n | nsh 可加结构化国际化 |
| pixel-snapshot golden 测试 | nsh 可加跨后端像素回归 |

---

## 6. 具体行动建议（修正版，含已具备能力）

| 优先级 | 行动 | floom 现状 | 借鉴来源 |
|--------|------|-----------|----------|
| **P0** | TSX 前端：`.tsx` → `floom-ui-ir` → (编译期 Rust / 运行时 VM) | 有 `floom-dsl`+`floom-ui-ir`+`floom-codegen`，无 TSX | nsh 的 TSX 模式（但渲染走 floom 自有渲染器） |
| **P1** | 设计令牌 + Tailwind 类实用工具 | 有 `floom-*` 设计套件 emit `StyleProps` | nsh `nsh/base` + token |
| **P1** | CLI `build`/`package`/`run` 补全 | 有 `floom create`（app/plugin）+ `just build` | nsh `nsh build/package/run` |
| **P2** | 运行时 UI 加载（AI 生成屏幕） | 有 `floom-agent`、`floom-hot-reload` | nsh 签名包 + sandbox |
| **P2** | 反应模型精炼 | 有 `floom-signal` (leptos 路线) | nsh SolidJS owner 模型 |
| **P3** | 原生 view 嵌入完善 | 有 `floom-native` + `floom::embedder` | nsh `NshAppView` 模式 |
| **P3** | GPU 后端多样化 | 已有 vello/wgpu + tiny-skia | nsh GPUI 后端参考 |

---

## 7. 结论

**floom** 与 **longbridge/nsh** 都是志向远大的 Rust GUI 框架，但路径互补：

- **floom** 强在**分层、feature-gated 架构** + **多渲染后端**（GPU/软件/TUI/WASM）+ **成熟组件生态** + **Flutter 风格脚手架 CLI** + **类 Flutter 插件体系**。它优化了编辑器这类对性能/像素一致性敏感的原生应用，且**已具备借鉴 TSX 的地基**（`floom-ui-ir` 作为单一属性表、`floom-dsl` 三段式、codegen 后端）。

- **nsh** 强在**「build once, native everywhere」** + **平台原生渲染** + **TSX 双语言开发** + **完整 CLI+pipeline** + **SolidJS 反应模型** + **签名包/运行时模块**。

**最有价值的借鉴（按用户指明 + 可行性）**：

1. 🔥 **TSX 开发模式**：用 TSX 写 UI，但**渲染与打包仍由 floom 的 Rust 平台处理**（不走 nsh 原生 view）。floom 已有 `floom-ui-ir`/`floom-dsl`/`floom-codegen` 地基，缺口是 TSX 前端 parser + 编译期 emit 后端（或可选运行时 VM）。
2. **设计令牌 + Tailwind 类**：现代化 floom 的 theming，复用已有 `floom-*` 套件。
3. **CLI build/package/run 补全**：在已有 `floom create` 之上补全生命周期。
4. **运行时 UI 加载**：结合 `floom-agent`，支持 AI 生成/动态 TSX 屏幕。
5. **Flutter 参考延续**：floom 已在插件/脚手架对标 Flutter，nsh 的 TSX 模式可视为「Flutter 的 Dart UI + JS 生态」的另一种解法，floom 可同时吸收两者。

**关键纠正**：floom 不是「没有 CLI 的 leptos 翻版」，而是**已有 Flutter 风格 CLI + 类 Flutter 插件体系 + 自定义 DSL/IR/codegen 地基**的成熟框架；其真正的「可借鉴 nsh」缺口是 **TSX 开发模式**与**设计令牌系统**，而非基础设施缺失。

---
*分析修订于 2026-11-02。基于本地 floom workspace (`/Users/humphrey/Project/Rust/floem`) 与 longbridge/nsh GitHub 仓库。*
