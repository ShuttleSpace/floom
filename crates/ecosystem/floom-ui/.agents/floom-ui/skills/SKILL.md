你的目标本质上是在 **Floom（Rust UI framework）之上构建一个类似 Vuetify 生态的组件抽象层 + 多 Design System 实现层**。

架构上类似：

```
                 Application
                      |
              floom-elements API
                      |
        +-------------+-------------+
        |             |             |
 floom-widgets-md floom-widgets-cupertino floom-widgets-fluent
        |             |             |
        +-------------+-------------+
                      |
                   Floom Core
                      |
              winit / skia / native
```

核心思想：

* `floom-elements` = Headless Component Contract（组件协议层）
* `floom-widgets-*` = Design System Adapter（视觉实现层）
* App 只依赖抽象，不依赖具体 UI 风格
* Cargo feature 实现 tree-shaking

下面给出一份可以直接作为 AI Agent / Claude Code / Cursor Skill 使用的设计文档。

---

# FLOOM ELEMENTS UI SYSTEM SKILL

Version: 0.1.0

## 1. Project Mission

实现 Rust Floom 生态中的跨 Design System UI Component Framework。

目标：

类似：

* Vue + Vuetify
* React + Radix + Material UI
* Flutter Widget + Theme

提供：

```rust
use floom_elements::*;

Button::new()
    .variant(ButtonVariant::Primary)
    .on_click(...)
```

应用可以通过 feature 替换：

```toml
[dependencies]

floom-elements = "0.1"

floom-widgets-md = {
    version="0.1",
    optional=true
}

floom-widgets-cupertino = {
    version="0.1",
    optional=true
}
```

切换：

```toml
features=[
    "material"
]
```

变成：

```toml
features=[
    "cupertino"
]
```

业务代码零修改。

---

# 2. Architecture Principles

## 2.1 Three Layer Architecture

## Layer 1: Component Contract

Package:

```
floom-elements
```

职责：

定义：

* API
* Props
* Events
* State
* Accessibility
* Layout Contract

禁止：

* 颜色
* padding
* font
* animation

例如：

```rust
pub trait Button {
    fn disabled(self,bool)->Self;

    fn variant(
        self,
        variant: ButtonVariant
    )->Self;

    fn on_click(
        self,
        callback: Callback
    )->Self;
}
```

---

## Layer 2: Design System Implementation

Packages:

```
floom-widgets-md
floom-widgets-cupertino
floom-widgets-fluent
floom-widgets-vant
floom-widgets-daisy
```

负责：

Material:

```
Button
 ├── elevation
 ├── ripple
 ├── shape
 └── color scheme
```

Cupertino:

```
Button
 ├── blur
 ├── spring animation
 ├── SF metrics
```

---

## Layer 3: Runtime Renderer

```
floom-core
```

负责：

* layout
* paint
* event
* accessibility

组件库不能直接访问 renderer。

---

# 3. Workspace Structure

```
floom-ui/

├── Cargo.toml

├── crates/

│
├── floom-elements
│
├── floom-core-theme
│
├── floom-widget-runtime
│
├── floom-widgets-md
│
├── floom-widgets-cupertino
│
├── floom-widgets-fluent
│
├── floom-widgets-vant
│
├── floom-widgets-daisy
│
├── floom-icons
│
├── floom-testing
│
└── examples/
```

---

# 4. Component API Specification

## 4.1 Component Naming

统一：

```
Xxx
```

例如：

```
Button

TextField

Dialog

Select

Menu

Tabs

Table
```

禁止：

```
MdButton
CupertinoButton
```

业务层不可感知。

---

# 5. Component Contract

每个组件必须包含：

```
component/
|
├── spec.rs
├── props.rs
├── event.rs
├── state.rs
└── tests.rs
```

例如 Button:

```rust
pub enum ButtonVariant {

 Primary,

 Secondary,

 Danger,

 Ghost,

 Text

}
```

```rust
pub struct ButtonProps {

 pub disabled:bool,

 pub loading:bool,

 pub size:Size,

 pub variant:ButtonVariant

}
```

---

# 6. Theme System

## 6.1 Token First Design

禁止：

```rust
Color::rgb(255,0,0)
```

必须：

```rust
theme.color.primary
```

定义：

```rust
pub struct Theme {


colors:Colors,


spacing:Spacing,


typography:Typography,


radius:Radius,


animation:Animation

}
```

---

# 7. Design Token Standard

参考：

* Material 3
* Apple HIG
* Fluent 2

统一：

```
tokens/

colors
spacing
typography
radius
shadow
motion
```

例如：

```rust
theme.button.height.md
```

---

# 8. Tree Shaking Design

## Goal

App:

```rust
use floom_elements::Button;
```

编译：

只包含：

```
Button
Theme
Event
```

不要：

```
Dialog
Table
Tree
```

---

## Implementation

Cargo feature:

```toml
[features]

default=[]


button=[
"floom-widgets-md/button"
]


dialog=[
"floom-widgets-md/dialog"
]
```

组件拆 crate：

推荐：

```
floom-widget-button

floom-widget-input

floom-widget-dialog
```

不要：

```
floom-widgets-md
```

一个巨大 crate。

---

# 9. Dynamic Theme Switching

支持：

```rust
AppTheme::Material
AppTheme::Cupertino
```

但是：

默认静态。

推荐：

compile time:

```
feature switch
```

runtime:

```
trait ComponentFactory
```

---

# 10. Component Factory

核心：

```rust
pub trait WidgetFactory {


fn button(
 props:ButtonProps
)->impl View;


fn checkbox(
 props:CheckboxProps
)->impl View;


}
```

Material:

```rust
struct MaterialFactory;


impl WidgetFactory 
for MaterialFactory
{

fn button(...)
{

 MaterialButton::new()

}

}
```

Cupertino:

```rust
struct CupertinoFactory;
```

---

# 11. Application Usage

## Material

```rust
floom_elements::install(
    MaterialTheme
);


Button::new(
    "Save"
)
```

生成：

```
floom-widgets-md/Button
```

## Cupertino

只修改：

```rust
install(
 CupertinoTheme
)
```

业务不变。

---

# 12. Rust Trait Design

避免：

```rust
dyn Widget
```

原因：

* 性能损失
* object safety
* 编译复杂

优先：

Generic:

```rust
fn render<W:Widget>(
 widget:W
)
```

---

# 13. State Management

统一：

```
Component State
        |
        |
   Signal
        |
        |
   Floom reactive
```

组件不能保存：

```rust
Rc<RefCell<State>>
```

使用：

```rust
RwSignal<T>
```

---

# 14. Accessibility

每个组件必须支持：

```rust
aria_role()

aria_label()

keyboard_navigation()

focus()
```

Button:

必须：

```
Space
Enter
Tab
```

---

# 15. Animation Specification

统一接口：

```rust
Animation {


duration:

curve:

transition:


}
```

禁止：

```rust
sleep()
```

---

# 16. Testing Specification

## Unit Test

覆盖：

props:

```
80%
```

state:

```
90%
```

---

## Snapshot Test

目录：

```
tests/snapshots
```

例如：

```
button_material.snap
button_cupertino.snap
```

---

## Visual Regression

使用：

```
floom-testing
```

流程：

```
render

capture

compare

approve
```

---

# 17. Integration Test

每个 Design System:

必须：

```
examples/

button

input

dialog

navigation
```

保证 API 一致。

---

# 18. Benchmark

必须测试：

## Compile

```
cargo build --release
```

指标：

```
binary size
compile time
```

---

## Runtime

指标：

```
startup time

memory

fps

event latency
```

目标：

Material Button:

```
< Flutter Button +10%
```

---

# 19. Code Style

## Rust

使用：

```
rustfmt

clippy

cargo-deny
```

CI:

```bash
cargo fmt --check

cargo clippy -- -D warnings

cargo test
```

---

# 20. Naming Rules

Component:

```
PascalCase
```

Props:

```
camelCase
```

Files:

```
snake_case
```

Example:

```
button.rs

text_field.rs
```

---

# 21. Documentation Requirement

每个组件必须：

```
README.md

API.md

examples/

tests/
```

包含：

* API
* screenshot
* theme differences
* accessibility

---

# 22. AI Agent Implementation Rules

## Before coding

必须：

1. 阅读 component spec
2. 创建 API contract
3. 创建 snapshot
4. 实现 Material
5. 实现其他 theme

禁止：

直接写 UI。

---

# 23. Development Roadmap

## Phase 1 MVP (3个月)

实现：

```
Button

Text

TextField

Checkbox

Switch

Dialog

Menu

Card
```

支持：

```
Material
```

---

## Phase 2

增加：

```
Cupertino

Fluent
```

---

## Phase 3

高级组件：

```
Table

Tree

VirtualList

Chart

Navigation
```

---

## Phase 4

生态：

```
Theme Builder

Figma Token Import

CLI Generator
```

---

# 24. Difficulty Assessment

| 模块                 | 难度    |
| ------------------ | ----- |
| 基础 Button/Input    | ⭐⭐    |
| Theme Token        | ⭐⭐⭐   |
| 多 Design System    | ⭐⭐⭐⭐  |
| Tree shaking       | ⭐⭐⭐⭐  |
| Layout abstraction | ⭐⭐⭐⭐⭐ |
| Accessibility      | ⭐⭐⭐⭐  |
| Flutter 级生态        | ⭐⭐⭐⭐⭐ |

整体：

> 类似实现一个 Rust 版 Vuetify，需要 1~2 人约 1 年达到成熟水平。

MVP：

> 1 个熟悉 Rust + Floom 的开发者，3~4 个月可以完成。

---

# 25. 最关键的技术决策

不要直接复制 Vuetify。

推荐：

```
Radix UI
+
Material Design Tokens
+
Flutter Widget Architecture
+
Rust Trait System
+
Cargo Feature Tree Shaking
```

组合。

最终形态：

```
floom-elements
        |
        |
 Component Contract
        |
 +------+------+------+
 |      |      |      |
 MD   IOS   Fluent  Vant
```

这套架构可以让 Floom 具备类似：

* Flutter Widget ecosystem
* Vue Vuetify productivity
* React Radix abstraction
* Rust zero-cost abstraction

的能力。

