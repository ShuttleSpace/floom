<!--
Last updated: 2026-07-31
Status: Design doc — brainstorming complete, awaiting user review
Source: .agents/floom-ui/skills/SKILL.md
-->

# floom-elements 架构与实施计划

Version: 0.1.0

## 1. 项目使命

在 Floom（Rust UI framework）之上构建跨 Design System 的 UI Component Framework。

目标：类似 Vue + Vuetify / React + Radix + Material UI / Flutter Widget + Theme 的开发体验。

```rust
use floom_elements::*;

Button::new("Save")
    .variant(ButtonVariant::Primary)
    .size(ButtonSize::Medium)
    .on_click(|| println!("clicked"));
```

应用通过 Cargo feature 切换 Design System，业务代码零修改：

```toml
# Material (default)
floom-elements = { version = "0.1", features = ["material"] }

# 或 Cupertino
floom-elements = { version = "0.1", default-features = false, features = ["cupertino"] }
```

---

## 2. 架构：三层 + Per-Component Crate

### 2.1 架构图

```
                          Application
                               |
                       floom-elements API
                       (Component Contract)
                       compile-time cfg re-export
                               |
          +--------------------+--------------------+
          |                    |                    |
   material/             cupertino/           fluent/
   (Design System)       (Design System)     (Design System)
          |                    |                    |
   Per-component:        Per-component:     Per-component:
   md-button             cupertino-button   fluent-button
   md-text               cupertino-text     fluent-text
   md-text-field         ...                ...
   ...
          |                    |                    |
          +------- floom-core-theme ---------------+
          |                    |                    |
          +---- floom-widget-runtime (utils) ------+
          |                    |                    |
          +--------------------+--------------------+
                               |
                          Floom Core
```

### 2.2 Layer 1: Component Contract (`floom-elements`)

- 定义 Props、Events、State 类型
- 通过 `cfg(feature = "material")` 等条件编译 re-export 具体实现
- 禁止硬编码视觉属性（颜色、间距、动画、字体），这些由 `floom-core-theme` 提供
- 应用层只依赖 `floom-elements`，不直接依赖任何 `floom-widget-*` crate

### 2.3 Layer 2: Design System Implementation

- 每组件一 crate：`floom-widget-<ds>-<component>`
- 编译隔离、真正的 tree-shaking
- 每个 crate 内部引用 `floom-core-theme` 获取设计令牌
- 可独立编译、测试、发布

### 2.4 Layer 3: Runtime Renderer

- Floom Core 负责 layout、paint、event、accessibility
- 组件 crate 通过 Floom 公开 API（`View` trait、`RwSignal`、事件系统）与 core 交互

---

## 3. Theme System: Token-First Design

### 3.1 原则

**禁止：** `Color::rgb(255, 0, 0)` 或 `padding: 16.0`
**必须：** `theme.colors().primary` 或 `theme.component(ComponentId::Button).height`

### 3.2 `floom-core-theme` crate 结构

```
floom-core-theme/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── tokens/
│   │   ├── color.rs       # ColorTokens: primary, secondary, surface, error, on_*
│   │   ├── spacing.rs     # SpacingTokens: xs(4), sm(8), md(16), lg(24), xl(32)
│   │   ├── typography.rs  # TypographyTokens: display, headline, title, body, label
│   │   ├── radius.rs      # RadiusTokens: none(0), sm(4), md(8), lg(16), full
│   │   ├── shadow.rs      # ElevationTokens: level_0..24
│   │   └── motion.rs      # MotionTokens: duration_short(150ms), medium(300ms), long(500ms)
│   ├── theme.rs           # Theme trait
│   └── component_tokens/
│       ├── mod.rs         # ComponentId enum + ComponentTokens registry
│       ├── button.rs      # ButtonTokens: height, padding_h, padding_v, icon_size, gap
│       ├── text_field.rs  # TextFieldTokens: height, padding, border_radius
│       └── ...
```

### 3.3 Theme trait

```rust
pub trait Theme: Clone + Send + Sync + 'static {
    fn colors(&self) -> &ColorTokens;
    fn spacing(&self) -> &SpacingTokens;
    fn typography(&self) -> &TypographyTokens;
    fn radius(&self) -> &RadiusTokens;
    fn shadow(&self) -> &ElevationTokens;
    fn motion(&self) -> &MotionTokens;
    fn component(&self, id: ComponentId) -> ComponentTokens;
}
```

### 3.4 Material 3 Theme

```rust
pub struct MaterialTheme {
    colors: MaterialColors,       // M3 Dynamic Color scheme
    spacing: M3Spacing,           // 4dp grid
    typography: M3Typography,     // Roboto type scale
    radius: M3Radius,
    shadow: M3Elevation,
    motion: M3Motion,
    component_tokens: M3ComponentTokens,
}

impl Theme for MaterialTheme { /* ... */ }
```

---

## 4. 编译期分发设计

### 4.1 决策

Phase 1 采用 **`cfg(feature)` compile-time re-export**，零运行时开销。
Phase 2+ 可扩展 `WidgetFactory` trait + global `OnceLock` 实现 runtime 切换。

### 4.2 Re-export 模式

```rust
// crates/floom-elements/src/button.rs

pub use floom_core_theme::button::{ButtonProps, ButtonSize, ButtonVariant};

// Compile-time dispatch: only one feature is ever active
#[cfg(feature = "material")]
pub use floom_widget_md_button::MdButton as Button;

#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_button::CupertinoButton as Button;

#[cfg(not(any(feature = "material", feature = "cupertino")))]
compile_error!("At least one Design System feature must be enabled");
```

### 4.3 Cargo.toml features

```toml
[features]
default = ["material"]
material = [
    "floom-widget-md-button",
    "floom-widget-md-text",
    "floom-widget-md-text-field",
    "floom-widget-md-checkbox",
    "floom-widget-md-switch",
    "floom-widget-md-dialog",
    "floom-widget-md-menu",
    "floom-widget-md-card",
]
cupertino = [
    "floom-widget-cupertino-button",
    # ...
]
```

### 4.4 业务层使用

```rust
// cargo build 时通过 RUSTFLAGS / Cargo.toml feature 控制
// 业务代码完全不变
use floom_elements::{Button, TextField, Dialog};
```

---

## 5. State Management

### 5.1 原则

- 基于 Floom 的 `RwSignal<T>` reactive 原语
- 禁止 `Rc<RefCell<State>>`
- 禁止 `Arc<Mutex<State>>`（除跨线程场景）

### 5.2 组件内部状态

```rust
// crates/material/md-button/src/state.rs

pub struct ButtonState {
    pub disabled: RwSignal<bool>,
    pub loading: RwSignal<bool>,
    pub pressed: RwSignal<bool>,
    pub hovered: RwSignal<bool>,
    pub focused: RwSignal<bool>,
}

impl ButtonState {
    pub fn from_props(props: &ButtonProps) -> Self {
        Self {
            disabled: RwSignal::new(props.disabled),
            loading: RwSignal::new(props.loading),
            pressed: RwSignal::new(false),
            hovered: RwSignal::new(false),
            focused: RwSignal::new(false),
        }
    }
}
```

---

## 6. Phase 1 Workspace 结构

### 6.1 目录树

```
floom-ui/
├── Cargo.toml                  # workspace root
├── docs/
│   └── PLAN.md
├── crates/
│   ├── floom-core-theme/       # Theme trait + tokens（Round 1）
│   ├── floom-widget-runtime/   # 共享工具、context helpers（Round 1）
│   ├── floom-elements/         # Contract + cfg re-exports（Round 2）
│   ├── floom-icons/            # Icon 类型（Round 3，与其他并行）
│   ├── floom-testing/          # Snapshot / visual regression（Round 4）
│   └── material/               # Material Design System（Round 3）
│       ├── md-button/
│       ├── md-text/
│       ├── md-text-field/
│       ├── md-checkbox/
│       ├── md-switch/
│       ├── md-dialog/
│       ├── md-menu/
│       └── md-card/
└── examples/
    └── demo/                   # 全组件验证 demo（Round 4）
```

### 6.2 依赖图与构建顺序

```
Round 1: floom-core-theme ─┬─> floom-widget-runtime
                            │
Round 2:                    └─> floom-elements
                                    │
Round 3:    +-----------------------+-----------------------+
            |       |       |       |       |       |       |
         md-button md-text md-text  md-    md-    md-    md-
                          -field   checkbox switch dialog menu card
            |       |       |       |       |       |       |
Round 4:    +--------------+-------+-------+-------+-------+
                            |
                    floom-testing
                    examples/demo
```

Round 3 的 8 个组件 crate 相互独立，可并行开发。Round 4 依赖 Round 3 全部完成。

---

## 7. 单个组件 crate 内部结构

以 `md-button` 为例：

```
crates/material/md-button/
├── Cargo.toml
├── src/
│   ├── lib.rs          # MdButton struct, builder API, View impl
│   ├── spec.rs         # ButtonSpec trait impl（对接 floom-elements 契约）
│   ├── props.rs        # ButtonProps 构造器
│   ├── state.rs        # ButtonState（RwSignal 状态机）
│   └── style.rs        # Material 样式（仅引用 Theme trait）
└── tests/
    ├── unit.rs         # Props/state 单元测试
    └── snapshots/
        └── button_default.snap   # snapshot regression
```

### 7.1 `lib.rs` API 示例

```rust
use floom::prelude::*;
use floom_core_theme::prelude::*;

pub struct MdButton {
    props: ButtonProps,
    state: ButtonState,
    theme: RwSignal<MaterialTheme>,
}

impl MdButton {
    pub fn new(label: impl Into<String>) -> Self { /* ... */ }
    pub fn variant(mut self, v: ButtonVariant) -> Self { /* ... */ }
    pub fn size(mut self, s: ButtonSize) -> Self { /* ... */ }
    pub fn disabled(mut self, d: bool) -> Self { /* ... */ }
    pub fn loading(mut self, l: bool) -> Self { /* ... */ }
    pub fn on_click(mut self, cb: impl Fn() + 'static) -> Self { /* ... */ }
}

impl View for MdButton {
    fn view(&self) -> impl View {
        // Reference self.theme.get() for all visual tokens
        // ...
    }
}
```

---

## 8. Per-Component Crate 策略

| 组件 | Material crate | Cupertino crate (Phase 2) | Fluent crate (Phase 2) |
|------|--------------|--------------------------|------------------------|
| Button | `floom-widget-md-button` | `floom-widget-cupertino-button` | `floom-widget-fluent-button` |
| Text | `floom-widget-md-text` | `floom-widget-cupertino-text` | `floom-widget-fluent-text` |
| TextField | `floom-widget-md-text-field` | `floom-widget-cupertino-text-field` | `floom-widget-fluent-text-field` |
| Checkbox | `floom-widget-md-checkbox` | `floom-widget-cupertino-checkbox` | `floom-widget-fluent-checkbox` |
| Switch | `floom-widget-md-switch` | `floom-widget-cupertino-switch` | `floom-widget-fluent-switch` |
| Dialog | `floom-widget-md-dialog` | `floom-widget-cupertino-dialog` | `floom-widget-fluent-dialog` |
| Menu | `floom-widget-md-menu` | `floom-widget-cupertino-menu` | `floom-widget-fluent-menu` |
| Card | `floom-widget-md-card` | `floom-widget-cupertino-card` | `floom-widget-fluent-card` |

---

## 9. 开发路线图

### Phase 1 — MVP（Material only）

**目标：** 基础组件可用，架构验证通过

| 交付物 | 说明 |
|--------|------|
| `floom-core-theme` | Theme trait + Color/Spacing/Typography/Radius/Shadow/Motion tokens |
| `floom-widget-runtime` | 通用 context helpers、layout 工具 |
| `floom-elements` | 8 个组件的 Props/Events 定义 + cfg re-export |
| Material 组件 x8 | Button, Text, TextField, Checkbox, Switch, Dialog, Menu, Card |
| `floom-testing` | Snapshot testing 基础设施 |
| `examples/demo` | 全组件交互 demo |

### Phase 2 — 多设计系统

- Cupertino 组件 x8
- Fluent 组件 x8
- Theme 切换 demo

### Phase 3 — 高级组件

- Table, Tree, VirtualList, Chart, Navigation
- 每组件 3 个 Design System 实现

### Phase 4 — 生态

- Theme Builder（可视化编辑 tokens）
- Figma Token Import（从 Figma 导入设计令牌）
- CLI Generator（`floom generate button` 自动生成组件模板）
- 文档站点

---

## 10. 约束与规范

### 10.1 组件 API 规则

- 组件名：`PascalCase`（`Button`, `TextField`）
- Props 字段：`snake_case`（`on_click`, `max_length`）
- 文件名：`snake_case`（`button.rs`, `text_field.rs`）
- 禁止在 API 层暴露 Design System 名称（不要 `MdButton`, `CupertinoButton`）

### 10.2 Accessibility

每个组件必须支持：
- `aria_label()`
- `keyboard_navigation()`
- `focus()` 管理
- Button：Space / Enter / Tab

### 10.3 Testing 要求

- Props 单元测试覆盖率 ≥80%
- State 状态机测试覆盖率 ≥90%
- Snapshot 测试：每个 variant + size 组合
- Integration test：每个 Design System `examples/` 目录

### 10.4 Code Style

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

---

## 11. 技术参考

- Floom: https://github.com/lapce/floom
- Material Design 3: https://m3.material.io/
- Radix UI (headless component pattern): https://www.radix-ui.com/
- Flutter Widget Catalog (API style reference): https://api.flutter.dev/
