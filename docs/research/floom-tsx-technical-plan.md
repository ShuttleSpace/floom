# floom 借鉴 nsh TSX 的具体技术方案

> **核心约束（用户指明）**：用 TSX 开发项目，但**渲染和打包都由 floom 的 Rust 平台处理**。
> 即：TSX 是**前端 DSL**（开发体验），但**不走 nsh 的「原生 view 渲染」**，而是 lower 到 floom 已有的 `floom-ui-ir`，由 floom 自有渲染器（vello/wgpu、tiny-skia、TUI、WASM）绘制，由 Cargo/`just build` 打包。
>
> **关键发现**：floom 已有「单一属性表」`floom-ui-ir::UiDocument`——它 JSON 可序列化、词汇封闭（`NodeKind` 是 enum）、自带 `validate()`、`floom-dsl` 已完成 `source → UiDocument → Rust builder` 全链路。TSX 只需替换「前端 parser」这一段，下游（emit → 渲染）完全复用。

---

## 1. 架构总览：TSX 在 floom 管线中的位置

```
                    ┌─────────────────────────────────────────────┐
   .tsx 源文件 ───► │  floom-tsx (NEW)                            │
                    │   parse(TSX) → TSX-AST → UiDocument(IR)     │
                    │   + 生成 .d.ts 声明（编辑器感知，仿 nsh）     │
                    └───────────────┬─────────────────────────────┘
                                    │ UiDocument (JSON 可序列化)
                    ┌───────────────┴─────────────────────────────┐
                    │  复用现有下游（关键：不重写）                  │
                    │  • floom-dsl::emit  → Rust builder program   │
                    │  • 或 floom-hot-reload 的 key-differ（运行时）│
                    └───────────────┬─────────────────────────────┘
                                    │ Rust 构建树 / 增量文档
                    ┌───────────────┴─────────────────────────────┐
                    │  floom 渲染器（vello/wgpu / tiny-skia / TUI / WASM）
                    │  + floom 平台层（desktop/android/ios/harmony/web）
                    │  + floom 打包（Cargo feature + just build）
                    └─────────────────────────────────────────────┘
```

**与 nsh 的本质区别（必须守住）**：
- nsh：`TSX → QuickJS VM → 原生 view（UIKit/AppKit/...）`
- floom（本方案）：`TSX → floom-ui-ir → floom 渲染器 scene graph`
- 保留优势：GPU 性能、跨平台像素一致性、pixel-snapshot golden 测试、feature-gating。

---

## 2. 为什么 IR 是「鹊桥」

`floom-ui-ir::UiDocument` 现状（已读源码确认）：

```rust
pub struct UiDocument {
    pub version: IrVersion,
    pub root: NodeKey,
    pub bindings: Vec<Binding>,      // name + Kind(Handler | Value(ty))
    pub nodes: Vec<UiNode>,
}
pub struct UiNode {
    pub key: NodeKey,                // dotted, 稳定跨 reload
    pub kind: NodeKind,              // 封闭 enum：Text/Button/Stack/.../Native
    pub props: Vec<Prop>,            // name + PropValue
    pub slots: Vec<Slot>,            // 命名子节点列表
    pub events: Vec<EventBinding>,   // event: EventName → handler: String
}
```

- `NodeKind` 是**封闭 enum**（不是 `Custom(String)`），已含 Button/Checkbox/Stack/Grid/List/Conditional/Overlay/Dialog/Popup/Tabs/NavigationDrawer/Native(Web|Video|Map|TextInput) 等。
- `validate()` 已做：key 合法性、重复 key、根可达、slot 父子、环检测、binding 类型解析。
- `to_json()` / `from_json()` 已就绪 → **TSX 编译产物就是一段 JSON**，可直接落盘做缓存/热重载。
- `floom-dsl::emit` 已能把 `UiDocument` 变成 Rust builder（含结构共享、repeat 阈值提取函数）。

**结论**：TSX 前端只需产出 `UiDocument`，后面全是复用。这比 nsh 从零搭 TSX→原生 view 桥接省力一个数量级。

---

## 3. Route A：编译期（推荐先落地）

TSX 在**构建期**被编译为 Rust builder，进入 app 二进制。无运行时 VM，最贴合「渲染打包都由 floom Rust 平台处理」。

### 3.1 映射表（JSX → IR）

| TSX 写法 | `UiDocument` 产物 |
|----------|------------------|
| `<Button label="hi" />` | `UiNode { kind: Button, props: [Prop{name:"label", value: Str("hi")}] }` |
| `<Stack>{children}</Stack>` | `UiNode { kind: Stack, slots: [Slot{name:"default", children}] }` |
| `<Button onClick={h} />` | `UiNode { events: [EventBinding{event: Click, handler:"h"}] }` |
| `{count}` 在 prop 中 | `PropValue::Ref("count")` → 解析到 `bindings` |
| `show ? <A/> : <B/>` | `UiNode { kind: Conditional, props:[Prop{name:"when", value: Ref("show")}], slots:[a,b] }` |
| `items.map(i => <Row key={i.id} />)` | `UiNode { kind: List, slots:[...] }`，key 来自 `key=` |
| `style={...}` / `class="text-primary"` | `props` 里落入对应 `StyleProps`（见 §5） |

### 3.2 反应式绑定（最难、最关键的一环）

nsh 能在 TSX 和 Rust 间共享反应引擎，是因为 **SolidJS 本身在 TSX 层就是 signal**：
```tsx
let [count, setCount] = createSignal(0);   // 作者显式写 signal
```
floom 要做到同样效果，必须**在 TSX 侧暴露与 `floom_signal` 同构的 API**：

```tsx
// floom-tsx 提供的 TSX 侧 shim（类型声明由 .d.ts 生成）
import { createSignal, createMemo, createEffect, useContext, provideContext } from "floom";

export function Counter() {
  const [count, setCount] = createSignal(0);
  const double = createMemo(() => count() * 2);
  return (
    <Stack axis="vertical" gap={8}>
      <Text label={() => `Value: ${count()}`} />   {/* 读 signal → 自动订阅 */}
      <Button label="Increment" onClick={() => setCount(count() + 1)} />
    </Stack>
  );
}
```

**编译期翻译规则**（floom-tsx 的 lower 阶段）：
- `createSignal(initial)` → `floom_signal::create_signal(initial)`；解构 `[get, set]` → `(RwSignal::get, RwSignal::set)`（或 `Scope` 管理生命周期）。
- `count()` 读取 → 翻译成 `count.get()`；`setCount(v)` → `count.set(v)`。
- `createMemo(fn)` → `floom_signal::create_memo(fn)`；`createEffect` → `create_effect`。
- `provideContext`/`useContext` → `floom_signal::{provide_context, use_context}`。
- 闭包捕获的 signal 若需跨 `Child::new`（要求 `Send+Sync`），按 `floom-showcase` 的做法升级为 `SyncBaseSignal`/`Arc<Mutex<...>>`。

这样**编译产物就是 Rust 开发者手写的等价代码**，零运行时开销，零 VM。

### 3.3 构建集成

**方案 A（推荐，最简单，契合 floom-cli 现状）**：app 仓库 `build.rs`：

```rust
// build.rs
fn main() {
    let out = std::env::var("OUT_DIR").unwrap();
    let tsx = glob::glob("src/**/*.tsx").unwrap();
    let mut doc = floom_tsx::compile_project(tsx)?;   // .tsx → UiDocument
    let rust = floom_dsl::emit::emit(&doc, WidgetSource::WithComposites);
    std::fs::write(format!("{out}/ui_gen.rs"), rust).unwrap();
    println!("cargo:rerun-if-changed=src");
}
```
```rust
// src/main.rs
mod ui_gen { include!(concat!(env!("OUT_DIR"), "/ui_gen.rs")); }
fn main() {
    floom::launch(|host| ui_gen::mount(host));   // 生成的 mount 函数
}
```

**方案 B（更优雅，后续）**：`floom-tsx-macro` 过程宏 `floom_tsx::include_tsx!("app.tsx")`，编译期展开。

### 3.4 复用 floom-dsl 的 emit 后端

`floom-dsl/src/emit.rs` 已证明能用：输入 `UiDocument`，输出 Rust builder，且做了结构共享（repeat 阈值提取为 `fn`/循环）。floom-tsx **不重写 emit**，只把 `UiDocument` 交给它。`WidgetSource::WithComposites` 已支持引入 `floom-ui-composites`（checkbox/radio/slider/textfield 等），与 nsh 的 `nsh/base`+套件分层的思路一致。

---

## 4. Route B：运行时（可选，给 AI / 热更新 / 插件）

当需要从**网络/磁盘**加载 TSX（AI 生成 UI、热更新、签名插件包）时，走 VM 路线。

### 4.1 VM 选型（对标 nsh-js）

| 选项 | 体积 | 沙箱 | 成熟度 | 备注 |
|------|------|------|--------|------|
| `rquickjs` (QuickJS) | 小 | 好 | 高 | **最接近 nsh-js**，推荐 |
| `deno_core` (V8) | 大 | 好 | 高 | 重，启动慢 |
| `boa` (纯 Rust) | 中 | 中 | 中 | 纯 Rust 但 ES 覆盖不全 |

参考 nsh 用 QuickJS 类引擎，floom 用 `rquickjs` 最佳：小体积、可 sandbox、Rust 绑定成熟。

### 4.2 运行时挂载流程

```
TSX 包（签名） → rquickjs 沙箱执行 → 导出 UiDocument（JSON）
   → floom-hot-reload 的 key-differ 计算增量
   → 应用到运行中的 WidgetHost（按 NodeKey 匹配，状态保留）
```

- **状态保留**：`floom-ui-ir` 的 `NodeKey` 是 dotted、稳定跨编辑/reload（`node.rs` 注释明确：「hot reload matches old and new nodes by key, so state survives」）。这正是 nsh 热 TSX 更新保留状态的等价机制——floom 已有地基。
- **沙箱**：`rquickjs` 的 `Context` 限制可访问的宿主函数（只允许 `floom_tsx_runtime` 暴露的 `createSignal`/`mount`/`fetch` 等），host 控制能力边界——对应 nsh 的 「host-controlled sandbox capabilities」。

### 4.3 与 Route A 共享同一套 TSX shim

Route B 的 TSX 侧 API（§3.2）在 VM 内由 `floom-tsx-runtime` 提供：
- 编译期：shim 是「类型声明 + 翻译规则」。
- 运行时：shim 是「VM 内的 JS 实现」，驱动细粒度重渲染（SolidJS 风格的 effect 图，运行在 VM 里）。
- 两者产出**同一种 `UiDocument`**，下游一致。这是 nsh 「same reactive engine on both sides」的 floom 版。

---

## 5. 设计令牌 + Tailwind 类（与 TSX 配套）

nsh 的 `nsh/base` + Tailwind 类是 TSX 开发体验的关键。floom 已有 `floom-material`/`floom-cupertino`/`floom-fluent`/`floom-harmony` 套件（emit `StyleProps`），只需：

- 在 `floom-style` 引入 `Token` 类型（颜色/间距/圆角/字体令牌）。
- 提供 `class="text-primary bg-surface rounded-md"` 解析：TSX 里 `class` 属性 → lower 时查令牌表 → 展开为 `StyleProps`，落入 `UiNode.props`。
- 复用 `floom-ui` 卫星仓库的现有套件作为令牌来源，避免重写绘制。

---

## 6. 编辑器支持（仿 nsh `.nsh/types`）

floom-tsx 在编译期额外生成 `.d.ts`：
- 从 `floom-ui-ir` 的 schema（kind/props/events 表）自动生成每个组件的 TS 类型。
- 从 `floom-ui` 套件生成 `floom` 包的类型声明。
- 让 TSX 作者在 VS Code 里获得 props/事件补全与类型检查——nsh 的 `model-readable documentation`/`llms.txt` 等价物，也是 AI 生成 UI 的安全护栏。

---

## 7. 具体 crate 拆分

| Crate | 职责 | 现状 |
|-------|------|------|
| `floom-tsx` (NEW) | TSX/JSX → `UiDocument`；`compile()` / `compile_project()`；.d.ts 生成 | 新增 |
| `floom-tsx-runtime` (NEW, optional) | `rquickjs` 沙箱 + TSX shim 的 JS 实现 + `UiDocument` delta 输出 | 新增 |
| `floom-tsx-macro` (NEW, optional) | `include_tsx!` 过程宏 | 新增 |
| `floom-ui-ir` | IR 词汇表 + validate + JSON | ✅ 已有 |
| `floom-dsl` | `emit` 后端（复用，不改） | ✅ 已有 |
| `floom-hot-reload` | key-differ（Route B 复用） | ✅ 已有 |
| `floom-style` / `floom-ui` 套件 | 令牌 + Tailwind 类解析 | ✅ 已有，需加令牌层 |

### 7.1 floom-tsx 内部结构（草图）

```
floom-tsx/src/
  lib.rs          // compile(source) -> Result<UiDocument, TsxError>
  parser.rs       // swc_ecma_parser → TSX AST（选 swc，不手写：TSX 非小语法）
  lower.rs        // TSX AST → UiDocument（含 §3.1 映射 + §3.2 信号 shim 识别）
  decl.rs         // UiDocument/schema → .d.ts 生成
  error.rs        // 带 span 的诊断（对齐 floom-dsl 的 Span 风格）
```

**parser 选型**：floom-dsl 因自定义小语法而手写 lexer/parser；**TSX 不是小语法**，手写是错的。用 `swc_ecma_parser`（Rust 原生、生产级）解析为 AST，再 lower。这是务实选择。

---

## 8. 分阶段落地

| 阶段 | 范围 | 交付 |
|------|------|------|
| **P0** | floom-tsx：静态 TSX（无信号）→ UiDocument → emit → Rust | 一个 counter.tsx 编译进 floom-showcase 跑起来 |
| **P1** | TSX 信号 shim（createSignal 等）→ floom_signal 翻译 | 编译期信号驱动的 TSX 组件 |
| **P2** | `class=` Tailwind 令牌解析 + .d.ts 生成 | 编辑器补全 + 主题类 |
| **P3** | Route B：rquickjs 沙箱 + 增量挂载（复用 floom-hot-reload） | `floom-agent` 驱动的 AI 生成 UI demo |
| **P4** | floom-cli 集成：`floom create --template tsx` 脚手架 | 对标 nsh `nsh init my-app --template ts` |

---

## 9. 与 nsh 对比的保留项（checklist）

复用 nsh 的**开发体验**，但守住 floom 的**架构优势**：

- ✅ 借鉴：TSX 双语言开发、编译期+运行时双轨、.d.ts 编辑器支持、Tailwind 类、签名包/沙箱（Route B）、热更新保留状态。
- ✅ 守住：渲染走 floom 自有渲染器（非原生 view）；打包走 Cargo/`just build`；pixel-snapshot golden 测试；feature-gating；~80 crate 分层。
- ⚠️ 不照搬：nsh 的 GPUI 后端、UIKit/AppKit 原生 view 渲染、QuickJS 作为**唯一**运行模型（floom 默认编译期，VM 仅可选）。

---

## 10. 一句话总结

> floom 已有 `floom-ui-ir` 这座桥。`floom-tsx` 只需在桥的「上游」接一个 TSX 前端（swc 解析 + lower 到 `UiDocument` + 信号 shim 翻译），「下游」的 `floom-dsl::emit` → 渲染器 → 打包全部复用。默认编译期零 VM（满足「渲染打包都由 floom Rust 平台处理」），可选运行时 VM（rquickjs）用于 AI/热更新——这与 nsh 的 TSX 模式同构，但渲染锚定在 floom 自己的 GPU/软件渲染器上。

---
*技术规划基于 floom workspace (`/Users/humphrey/Project/Rust/floem`) 真实 IR 与 DSL/codegen 实现，对照 longbridge/nsh 的 TSX 模式。*
