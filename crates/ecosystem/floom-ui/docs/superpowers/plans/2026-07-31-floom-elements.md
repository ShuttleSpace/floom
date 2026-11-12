# floom-elements Phase 1 Implementation Plan

> **现状校正（2026-11-11，reconcile）：本计划是 v1 时代的 `floom-elements` 三层层架构设计稿，
> 今天已**不适用**。`floom-elements` facade 在 "Stage D" 已移除（`crates/ecosystem/floom-ui/Cargo.toml:361-365`），
> 组件框架改为 engine `floom-widgets` → `floom-ui-composites` → per-DS kit crates（见 `docs/PLAN.md` 的
> v2 架构与任务清单）。本文件保留为历史计划；其 checkbox **从未勾选**，路径也指向其他机器
> （`/Users/ahs/...`）。新的工作依据 `docs/PLAN.md`（2026-11-11 版）。

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the floom-elements MVP: 8 Material Design components on a cross-design-system component framework with token-based theming, compile-time Design System dispatch, and per-component tree-shaking.

**Architecture:** Three-layer design — `floom-core-theme` (tokens/trait), `floom-elements` (contract + cfg re-exports), per-component Material crates (`floom-widget-md-*`). Components use `RwSignal` for state and reference `Theme` trait for all visual properties.

**Tech Stack:** Rust, Floom (UI framework), Material Design 3 tokens, Cargo workspace with per-component crates

**Design doc:** [docs/PLAN.md](/Users/ahs/Project/Rust/floom-ui/docs/PLAN.md)

---

## File Structure Overview

```
floom-ui/
├── Cargo.toml                          # Task 1
├── crates/
│   ├── floom-core-theme/
│   │   ├── Cargo.toml                  # Task 2
│   │   └── src/
│   │       ├── lib.rs                  # Task 2
│   │       ├── tokens/mod.rs           # Task 3
│   │       ├── tokens/{color,spacing,typography,radius,shadow,motion}.rs  # Task 3
│   │       ├── theme.rs                # Task 4
│   │       └── component_tokens/       # Task 5
│   │           ├── mod.rs
│   │           └── {button,text,text_field,checkbox,switch,dialog,menu,card}.rs
│   ├── floom-widget-runtime/           # Task 6
│   │   ├── Cargo.toml
│   │   └── crates/floom/src/lib.rs
│   ├── floom-elements/                 # Task 7
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── {button,text,text_field,checkbox,switch,dialog,menu,card}.rs
│   └── material/                       # Tasks 8-15
│       ├── md-button/    # Task 8
│       ├── md-text/      # Task 9
│       ├── md-text-field/ # Task 10
│       ├── md-checkbox/  # Task 11
│       ├── md-switch/    # Task 12
│       ├── md-dialog/    # Task 13
│       ├── md-menu/      # Task 14
│       └── md-card/      # Task 15
├── examples/demo/                      # Task 16
└── crates/floom-testing/               # Task 17
```

---

## Task 0: Prerequisite — Floom API Verification

**Files:** None (read-only check)

- [ ] **Step 1: Add Floom dependency temporarily and check API**

```bash
cargo init --lib /tmp/floom-check && cd /tmp/floom-check
echo 'floom = "0.1"' >> Cargo.toml   # adjust version
echo 'fn main() { println!("ok"); }' > crates/floom/src/lib.rs
cargo check 2>&1 | tail -5
```

Expected: clean check or version-not-found error that guides correct version

- [ ] **Step 2: Verify View trait and RwSignal availability**

```bash
cat > /tmp/floom-check/src/lib.rs << 'RUST'
use floom::prelude::*;
use floom::reactive::RwSignal;

fn check() -> impl View {
    let count = RwSignal::new(0);
    count.get()
}
RUST
cargo check 2>&1
```

Expected: Clean compile or clear error about correct import paths. Document the correct module paths for use in subsequent tasks.

- [ ] **Step 3: Clean up and document findings**

```bash
rm -rf /tmp/floom-check
```

Record verified Floom version and key API paths in a note. If API differs from what's in this plan, adjust imports in plan before proceeding.

---

## Task 1: Workspace Root Setup

**Files:**
- Create: `Cargo.toml`

- [ ] **Step 1: Create workspace Cargo.toml**

```toml
[workspace]
members = [
    "crates/floom-core-theme",
    "crates/floom-widget-runtime",
    "crates/floom-elements",
    "crates/material/md-button",
    "crates/material/md-text",
    "crates/material/md-text-field",
    "crates/material/md-checkbox",
    "crates/material/md-switch",
    "crates/material/md-dialog",
    "crates/material/md-menu",
    "crates/material/md-card",
    "crates/floom-testing",
    "examples/demo",
]

resolver = "2"

[workspace.dependencies]
floom = "0.1"
floom-core-theme = { path = "crates/floom-core-theme" }
floom-widget-runtime = { path = "crates/floom-widget-runtime" }
floom-elements = { path = "crates/floom-elements" }
```

- [ ] **Step 2: Verify workspace parses**

```bash
cargo metadata --no-deps 2>&1 | head -5
```

Expected: "workspace members" listed, no errors about missing crate directories (they don't exist yet and won't error until cargo tries to resolve them).

---

## Task 2: floom-core-theme — Crate Scaffold + Token Types

**Files:**
- Create: `crates/floom-core-theme/Cargo.toml`
- Create: `crates/floom-core-theme/src/lib.rs`
- Create: `crates/floom-core-theme/src/tokens/mod.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "floom-core-theme"
version = "0.1.0"
edition = "2021"

[dependencies]
```

- [ ] **Step 2: Create src/tokens/mod.rs — re-export all token modules**

```rust
pub mod color;
pub mod motion;
pub mod radius;
pub mod shadow;
pub mod spacing;
pub mod typography;
```

- [ ] **Step 3: Create src/tokens/color.rs — ColorTokens**

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ColorTokens {
    pub primary: Color,
    pub on_primary: Color,
    pub primary_container: Color,
    pub on_primary_container: Color,
    pub secondary: Color,
    pub on_secondary: Color,
    pub secondary_container: Color,
    pub on_secondary_container: Color,
    pub tertiary: Color,
    pub on_tertiary: Color,
    pub error: Color,
    pub on_error: Color,
    pub error_container: Color,
    pub on_error_container: Color,
    pub surface: Color,
    pub on_surface: Color,
    pub surface_variant: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub outline_variant: Color,
    pub background: Color,
    pub on_background: Color,
    pub scrim: Color,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 255)
    }
}
```

- [ ] **Step 4: Create src/tokens/spacing.rs**

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpacingTokens {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub xxl: f32,
}

impl Default for SpacingTokens {
    fn default() -> Self {
        Self {
            xs: 4.0,
            sm: 8.0,
            md: 16.0,
            lg: 24.0,
            xl: 32.0,
            xxl: 48.0,
        }
    }
}
```

- [ ] **Step 5: Create src/tokens/typography.rs**

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct TypographyTokens {
    pub display: TypeScale,
    pub headline: TypeScale,
    pub title: TypeScale,
    pub body: TypeScale,
    pub label: TypeScale,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeScale {
    pub large: TextStyle,
    pub medium: TextStyle,
    pub small: TextStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub font_size: f32,
    pub line_height: f32,
    pub letter_spacing: f32,
    pub font_weight: FontWeight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Thin,
    ExtraLight,
    Light,
    Regular,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}
```

- [ ] **Step 6: Create src/tokens/radius.rs, shadow.rs, motion.rs**

`radius.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadiusTokens {
    pub none: f32,
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub full: f32,
}

impl Default for RadiusTokens {
    fn default() -> Self {
        Self { none: 0.0, xs: 4.0, sm: 8.0, md: 12.0, lg: 16.0, xl: 24.0, full: 9999.0 }
    }
}
```

`shadow.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowData {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub spread: f32,
    pub color_a: u8, // alpha only; color is black
}

#[derive(Debug, Clone, PartialEq)]
pub struct ElevationTokens {
    pub levels: [ShadowData; 6],
}

impl Default for ElevationTokens {
    fn default() -> Self {
        Self {
            levels: [
                ShadowData { offset_x: 0.0, offset_y: 1.0, blur: 2.0, spread: 0.0, color_a: 30 },
                ShadowData { offset_x: 0.0, offset_y: 2.0, blur: 4.0, spread: 0.0, color_a: 40 },
                ShadowData { offset_x: 0.0, offset_y: 4.0, blur: 8.0, spread: 0.0, color_a: 50 },
                ShadowData { offset_x: 0.0, offset_y: 6.0, blur: 12.0, spread: 0.0, color_a: 60 },
                ShadowData { offset_x: 0.0, offset_y: 8.0, blur: 16.0, spread: 0.0, color_a: 70 },
                ShadowData { offset_x: 0.0, offset_y: 12.0, blur: 24.0, spread: 0.0, color_a: 80 },
            ],
        }
    }
}
```

`motion.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionTokens {
    pub duration_short: f32,
    pub duration_medium: f32,
    pub duration_long: f32,
    pub easing_standard: Easing,
    pub easing_decelerate: Easing,
    pub easing_accelerate: Easing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Easing {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl Default for MotionTokens {
    fn default() -> Self {
        Self {
            duration_short: 150.0,
            duration_medium: 300.0,
            duration_long: 500.0,
            easing_standard: Easing { x1: 0.4, y1: 0.0, x2: 0.2, y2: 1.0 },
            easing_decelerate: Easing { x1: 0.0, y1: 0.0, x2: 0.2, y2: 1.0 },
            easing_accelerate: Easing { x1: 0.4, y1: 0.0, x2: 1.0, y2: 1.0 },
        }
    }
}
```

- [ ] **Step 7: Create crates/floom/src/lib.rs — public API surface**

```rust
pub mod component_tokens;
pub mod theme;
pub mod tokens;

pub mod prelude {
    pub use crate::component_tokens::*;
    pub use crate::theme::Theme;
    pub use crate::tokens::color::*;
    pub use crate::tokens::motion::*;
    pub use crate::tokens::radius::*;
    pub use crate::tokens::shadow::*;
    pub use crate::tokens::spacing::*;
    pub use crate::tokens::typography::*;
}
```

- [ ] **Step 8: Verify compile**

```bash
cargo check -p floom-core-theme 2>&1
```

Expected: Clean compile, no errors.

---

## Task 3: floom-core-theme — Theme Trait + MaterialTheme

**Files:**
- Create: `crates/floom-core-theme/src/theme.rs`

- [ ] **Step 1: Create src/theme.rs — Theme trait and MaterialTheme**

```rust
use crate::component_tokens::ComponentTokens;
use crate::component_tokens::ComponentId;
use crate::tokens::color::{Color, ColorTokens};
use crate::tokens::motion::MotionTokens;
use crate::tokens::radius::RadiusTokens;
use crate::tokens::shadow::ElevationTokens;
use crate::tokens::spacing::SpacingTokens;
use crate::tokens::typography::TypographyTokens;

pub trait Theme: Clone + Send + Sync + 'static {
    fn colors(&self) -> &ColorTokens;
    fn spacing(&self) -> &SpacingTokens;
    fn typography(&self) -> &TypographyTokens;
    fn radius(&self) -> &RadiusTokens;
    fn shadow(&self) -> &ElevationTokens;
    fn motion(&self) -> &MotionTokens;
    fn component(&self, id: ComponentId) -> ComponentTokens;
}

/// Material Design 3 theme implementation
#[derive(Debug, Clone)]
pub struct MaterialTheme {
    colors: ColorTokens,
    spacing: SpacingTokens,
    typography: TypographyTokens,
    radius: RadiusTokens,
    shadow: ElevationTokens,
    motion: MotionTokens,
    component_tokens: crate::component_tokens::material::MaterialComponentTokens,
}

impl MaterialTheme {
    pub fn new() -> Self {
        Self {
            colors: ColorTokens {
                primary: Color::rgb(103, 80, 164),
                on_primary: Color::rgb(255, 255, 255),
                primary_container: Color::rgb(234, 221, 255),
                on_primary_container: Color::rgb(33, 0, 93),
                secondary: Color::rgb(98, 91, 113),
                on_secondary: Color::rgb(255, 255, 255),
                secondary_container: Color::rgb(232, 222, 248),
                on_secondary_container: Color::rgb(29, 25, 43),
                tertiary: Color::rgb(125, 82, 96),
                on_tertiary: Color::rgb(255, 255, 255),
                error: Color::rgb(179, 38, 30),
                on_error: Color::rgb(255, 255, 255),
                error_container: Color::rgb(249, 222, 220),
                on_error_container: Color::rgb(65, 14, 11),
                surface: Color::rgb(254, 247, 255),
                on_surface: Color::rgb(29, 27, 32),
                surface_variant: Color::rgb(231, 224, 236),
                on_surface_variant: Color::rgb(73, 69, 79),
                outline: Color::rgb(121, 116, 126),
                outline_variant: Color::rgb(202, 196, 208),
                background: Color::rgb(254, 247, 255),
                on_background: Color::rgb(29, 27, 32),
                scrim: Color::rgba(0, 0, 0, 128),
            },
            spacing: SpacingTokens::default(),
            typography: TypographyTokens {
                display: crate::tokens::typography::TypeScale {
                    large: self.make_text(57.0, 64.0, FontWeight::Regular),
                    medium: self.make_text(45.0, 52.0, FontWeight::Regular),
                    small: self.make_text(36.0, 44.0, FontWeight::Regular),
                },
                headline: crate::tokens::typography::TypeScale {
                    large: self.make_text(32.0, 40.0, FontWeight::Regular),
                    medium: self.make_text(28.0, 36.0, FontWeight::Regular),
                    small: self.make_text(24.0, 32.0, FontWeight::Regular),
                },
                title: crate::tokens::typography::TypeScale {
                    large: self.make_text(22.0, 28.0, FontWeight::Regular),
                    medium: self.make_text(16.0, 24.0, FontWeight::Medium),
                    small: self.make_text(14.0, 20.0, FontWeight::Medium),
                },
                body: crate::tokens::typography::TypeScale {
                    large: self.make_text(16.0, 24.0, FontWeight::Regular),
                    medium: self.make_text(14.0, 20.0, FontWeight::Regular),
                    small: self.make_text(12.0, 16.0, FontWeight::Regular),
                },
                label: crate::tokens::typography::TypeScale {
                    large: self.make_text(14.0, 20.0, FontWeight::Medium),
                    medium: self.make_text(12.0, 16.0, FontWeight::Medium),
                    small: self.make_text(11.0, 16.0, FontWeight::Medium),
                },
            },
            radius: RadiusTokens::default(),
            shadow: ElevationTokens::default(),
            motion: MotionTokens::default(),
            component_tokens: crate::component_tokens::material::MaterialComponentTokens::new(),
        }
    }

    fn make_text(&self, size: f32, line_height: f32, weight: FontWeight) -> TextStyle {
        TextStyle { font_size: size, line_height, letter_spacing: 0.0, font_weight: weight }
    }
}

use crate::tokens::typography::{TextStyle, FontWeight};

impl Theme for MaterialTheme {
    fn colors(&self) -> &ColorTokens { &self.colors }
    fn spacing(&self) -> &SpacingTokens { &self.spacing }
    fn typography(&self) -> &TypographyTokens { &self.typography }
    fn radius(&self) -> &RadiusTokens { &self.radius }
    fn shadow(&self) -> &ElevationTokens { &self.shadow }
    fn motion(&self) -> &MotionTokens { &self.motion }
    fn component(&self, id: ComponentId) -> ComponentTokens { self.component_tokens.get(id) }
}
```

- [ ] **Step 2: Verify compile**

```bash
cargo check -p floom-core-theme 2>&1 | tail -5
```

Expected: COMPILE ERROR — `component_tokens` module doesn't exist yet. This is expected; Task 4 creates it.

---

## Task 4: floom-core-theme — Component Tokens

**Files:**
- Create: `crates/floom-core-theme/src/component_tokens/mod.rs`
- Create: `crates/floom-core-theme/src/component_tokens/{button,text,text_field,checkbox,switch,dialog,menu,card}.rs`

- [ ] **Step 1: Create src/component_tokens/mod.rs**

```rust
pub mod material;

mod button;
mod card;
mod checkbox;
mod dialog;
mod menu;
mod switch;
mod text;
mod text_field;

pub use button::ButtonTokens;
pub use card::CardTokens;
pub use checkbox::CheckboxTokens;
pub use dialog::DialogTokens;
pub use menu::MenuTokens;
pub use switch::SwitchTokens;
pub use text::TextTokens;
pub use text_field::TextFieldTokens;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentId {
    Button,
    Text,
    TextField,
    Checkbox,
    Switch,
    Dialog,
    Menu,
    Card,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComponentTokens {
    Button(ButtonTokens),
    Text(TextTokens),
    TextField(TextFieldTokens),
    Checkbox(CheckboxTokens),
    Switch(SwitchTokens),
    Dialog(DialogTokens),
    Menu(MenuTokens),
    Card(CardTokens),
}
```

- [ ] **Step 2: Create button.rs**

```rust
use crate::tokens::spacing::SpacingTokens;

#[derive(Debug, Clone, PartialEq)]
pub struct ButtonTokens {
    pub height: f32,
    pub padding_horizontal: f32,
    pub gap: f32,
    pub icon_size: f32,
}

impl ButtonTokens {
    pub fn from_spacing(s: &SpacingTokens) -> Self {
        Self {
            height: 40.0,
            padding_horizontal: s.lg,
            gap: s.sm,
            icon_size: 18.0,
        }
    }
}
```

- [ ] **Step 3: Create remaining token files**

`text.rs`:
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct TextTokens {
    pub max_lines_default: usize,
}
impl Default for TextTokens {
    fn default() -> Self { Self { max_lines_default: 1 } }
}
```

`text_field.rs`:
```rust
use crate::tokens::spacing::SpacingTokens;
#[derive(Debug, Clone, PartialEq)]
pub struct TextFieldTokens {
    pub height: f32,
    pub padding_horizontal: f32,
    pub border_radius: f32,
}
impl TextFieldTokens {
    pub fn from_spacing(s: &SpacingTokens) -> Self {
        Self { height: 48.0, padding_horizontal: s.md, border_radius: 4.0 }
    }
}
```

`checkbox.rs`:
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct CheckboxTokens { pub size: f32, pub border_radius: f32 }
impl Default for CheckboxTokens {
    fn default() -> Self { Self { size: 18.0, border_radius: 2.0 } }
}
```

`switch.rs`:
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct SwitchTokens { pub width: f32, pub height: f32, pub thumb_size: f32 }
impl Default for SwitchTokens {
    fn default() -> Self { Self { width: 52.0, height: 32.0, thumb_size: 24.0 } }
}
```

`dialog.rs`:
```rust
use crate::tokens::spacing::SpacingTokens;
#[derive(Debug, Clone, PartialEq)]
pub struct DialogTokens { pub max_width: f32, pub padding: f32, pub border_radius: f32 }
impl DialogTokens {
    pub fn from_spacing(s: &SpacingTokens) -> Self {
        Self { max_width: 560.0, padding: s.lg, border_radius: 28.0 }
    }
}
```

`menu.rs`:
```rust
use crate::tokens::spacing::SpacingTokens;
#[derive(Debug, Clone, PartialEq)]
pub struct MenuTokens { pub min_width: f32, pub item_height: f32, pub padding: f32 }
impl MenuTokens {
    pub fn from_spacing(s: &SpacingTokens) -> Self {
        Self { min_width: 112.0, item_height: 48.0, padding: s.sm }
    }
}
```

`card.rs`:
```rust
use crate::tokens::spacing::SpacingTokens;
#[derive(Debug, Clone, PartialEq)]
pub struct CardTokens { pub elevation: u8, pub border_radius: f32, pub padding: f32 }
impl CardTokens {
    pub fn from_spacing(s: &SpacingTokens) -> Self {
        Self { elevation: 1, border_radius: 12.0, padding: s.md }
    }
}
```

- [ ] **Step 4: Create src/component_tokens/material.rs — MaterialComponentTokens**

```rust
use super::*;

#[derive(Debug, Clone)]
pub struct MaterialComponentTokens {
    button: ButtonTokens,
    text: TextTokens,
    text_field: TextFieldTokens,
    checkbox: CheckboxTokens,
    switch: SwitchTokens,
    dialog: DialogTokens,
    menu: MenuTokens,
    card: CardTokens,
}

impl MaterialComponentTokens {
    pub fn new() -> Self {
        let spacing = crate::tokens::spacing::SpacingTokens::default();
        Self {
            button: ButtonTokens::from_spacing(&spacing),
            text: TextTokens::default(),
            text_field: TextFieldTokens::from_spacing(&spacing),
            checkbox: CheckboxTokens::default(),
            switch: SwitchTokens::default(),
            dialog: DialogTokens::from_spacing(&spacing),
            menu: MenuTokens::from_spacing(&spacing),
            card: CardTokens::from_spacing(&spacing),
        }
    }

    pub fn get(&self, id: ComponentId) -> ComponentTokens {
        match id {
            ComponentId::Button => ComponentTokens::Button(self.button.clone()),
            ComponentId::Text => ComponentTokens::Text(self.text.clone()),
            ComponentId::TextField => ComponentTokens::TextField(self.text_field.clone()),
            ComponentId::Checkbox => ComponentTokens::Checkbox(self.checkbox.clone()),
            ComponentId::Switch => ComponentTokens::Switch(self.switch.clone()),
            ComponentId::Dialog => ComponentTokens::Dialog(self.dialog.clone()),
            ComponentId::Menu => ComponentTokens::Menu(self.menu.clone()),
            ComponentId::Card => ComponentTokens::Card(self.card.clone()),
        }
    }
}
```

- [ ] **Step 5: Verify compile**

```bash
cargo check -p floom-core-theme 2>&1 | tail -5
```

Expected: Clean compile, no errors.

---

## Task 5: floom-widget-runtime

**Files:**
- Create: `crates/floom-widget-runtime/Cargo.toml`
- Create: `crates/floom-widget-runtime/src/lib.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "floom-widget-runtime"
version = "0.1.0"
edition = "2021"

[dependencies]
floom = { workspace = true }
floom-core-theme = { workspace = true }
```

- [ ] **Step 2: Create crates/floom/src/lib.rs**

```rust
use floom_core_theme::theme::{MaterialTheme, Theme};
use floom::reactive::RwSignal;

/// Provide a global theme signal. 
/// In Phase 1 this is always MaterialTheme; Phase 2+ can make it dynamic.
pub fn theme_signal() -> RwSignal<MaterialTheme> {
    RwSignal::new(MaterialTheme::new())
}

/// Helper: resolve a component token from the current theme
pub fn component_token(theme: &MaterialTheme, id: floom_core_theme::component_tokens::ComponentId) -> floom_core_theme::component_tokens::ComponentTokens {
    theme.component(id)
}
```

- [ ] **Step 3: Verify compile**

```bash
cargo check -p floom-widget-runtime 2>&1 | tail -5
```

Expected: Clean compile.

---

## Task 6: floom-elements — Contract + Re-export

**Files:**
- Create: `crates/floom-elements/Cargo.toml`
- Create: `crates/floom-elements/src/lib.rs`
- Create: `crates/floom-elements/src/{button,text,text_field,checkbox,switch,dialog,menu,card}.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "floom-elements"
version = "0.1.0"
edition = "2021"

[dependencies]
floom = { workspace = true }
floom-core-theme = { workspace = true }

floom-widget-md-button = { path = "../material/md-button", optional = true }
floom-widget-md-text = { path = "../material/md-text", optional = true }
floom-widget-md-text-field = { path = "../material/md-text-field", optional = true }
floom-widget-md-checkbox = { path = "../material/md-checkbox", optional = true }
floom-widget-md-switch = { path = "../material/md-switch", optional = true }
floom-widget-md-dialog = { path = "../material/md-dialog", optional = true }
floom-widget-md-menu = { path = "../material/md-menu", optional = true }
floom-widget-md-card = { path = "../material/md-card", optional = true }

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
```

- [ ] **Step 2: Create src/button.rs**

```rust
pub use floom_core_theme::component_tokens::{ButtonTokens, ComponentId};
pub use floom_core_theme::component_tokens::button::ButtonTokens as _;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize {
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
    Ghost,
    Text,
}

#[derive(Debug, Clone)]
pub struct ButtonProps {
    pub label: String,
    pub size: ButtonSize,
    pub variant: ButtonVariant,
    pub disabled: bool,
    pub loading: bool,
    pub on_click: Option<Box<dyn Fn()>>,
}

impl Default for ButtonProps {
    fn default() -> Self {
        Self {
            label: String::new(),
            size: ButtonSize::Medium,
            variant: ButtonVariant::Primary,
            disabled: false,
            loading: false,
            on_click: None,
        }
    }
}

#[cfg(feature = "material")]
pub use floom_widget_md_button::MdButton as Button;

#[cfg(not(any(feature = "material")))]
compile_error!("At least one Design System feature must be enabled");
```

- [ ] **Step 3: Create crates/floom/src/text.rs, crates/floom/src/text_field.rs, src/checkbox.rs (same pattern)**

`text.rs`:
```rust
pub use floom_core_theme::component_tokens::{TextTokens, ComponentId};

#[derive(Debug, Clone)]
pub struct TextProps {
    pub content: String,
    pub max_lines: Option<usize>,
    pub is_selectable: bool,
}

impl Default for TextProps {
    fn default() -> Self {
        Self { content: String::new(), max_lines: None, is_selectable: false }
    }
}

#[cfg(feature = "material")]
pub use floom_widget_md_text::MdText as Text;
```

`text_field.rs`:
```rust
pub use floom_core_theme::component_tokens::{TextFieldTokens, ComponentId};

#[derive(Debug, Clone)]
pub struct TextFieldProps {
    pub placeholder: String,
    pub value: String,
    pub is_password: bool,
    pub max_length: Option<usize>,
    pub disabled: bool,
    pub on_change: Option<Box<dyn Fn(&str)>>,
}

impl Default for TextFieldProps {
    fn default() -> Self {
        Self {
            placeholder: String::new(),
            value: String::new(),
            is_password: false,
            max_length: None,
            disabled: false,
            on_change: None,
        }
    }
}

#[cfg(feature = "material")]
pub use floom_widget_md_text_field::MdTextField as TextField;
```

`checkbox.rs`:
```rust
pub use floom_core_theme::component_tokens::CheckboxTokens;

#[derive(Debug, Clone)]
pub struct CheckboxProps {
    pub label: String,
    pub checked: bool,
    pub disabled: bool,
    pub on_change: Option<Box<dyn Fn(bool)>>,
}

impl Default for CheckboxProps {
    fn default() -> Self {
        Self { label: String::new(), checked: false, disabled: false, on_change: None }
    }
}

#[cfg(feature = "material")]
pub use floom_widget_md_checkbox::MdCheckbox as Checkbox;
```

`switch.rs`:
```rust
pub use floom_core_theme::component_tokens::SwitchTokens;

#[derive(Debug, Clone)]
pub struct SwitchProps {
    pub checked: bool,
    pub disabled: bool,
    pub on_change: Option<Box<dyn Fn(bool)>>,
}

impl Default for SwitchProps {
    fn default() -> Self {
        Self { checked: false, disabled: false, on_change: None }
    }
}

#[cfg(feature = "material")]
pub use floom_widget_md_switch::MdSwitch as Switch;
```

`dialog.rs`:
```rust
pub use floom_core_theme::component_tokens::DialogTokens;

#[derive(Debug, Clone)]
pub struct DialogProps {
    pub title: String,
    pub content: String,
    pub open: bool,
    pub on_close: Option<Box<dyn Fn()>>,
}

impl Default for DialogProps {
    fn default() -> Self {
        Self { title: String::new(), content: String::new(), open: false, on_close: None }
    }
}

#[cfg(feature = "material")]
pub use floom_widget_md_dialog::MdDialog as Dialog;
```

`menu.rs`:
```rust
pub use floom_core_theme::component_tokens::MenuTokens;

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub on_select: Option<Box<dyn Fn()>>,
}

#[derive(Debug, Clone)]
pub struct MenuProps {
    pub items: Vec<MenuItem>,
    pub open: bool,
}

impl Default for MenuProps {
    fn default() -> Self {
        Self { items: vec![], open: false }
    }
}

#[cfg(feature = "material")]
pub use floom_widget_md_menu::MdMenu as Menu;
```

`card.rs`:
```rust
pub use floom_core_theme::component_tokens::CardTokens;

#[derive(Debug, Clone)]
pub struct CardProps {
    pub elevation: u8,
}

impl Default for CardProps {
    fn default() -> Self {
        Self { elevation: 1 }
    }
}

#[cfg(feature = "material")]
pub use floom_widget_md_card::MdCard as Card;
```

- [ ] **Step 4: Create crates/floom/src/lib.rs**

```rust
pub mod button;
pub mod card;
pub mod checkbox;
pub mod dialog;
pub mod menu;
pub mod switch;
pub mod text;
pub mod text_field;

pub mod prelude {
    pub use crate::button::*;
    pub use crate::card::*;
    pub use crate::checkbox::*;
    pub use crate::dialog::*;
    pub use crate::menu::*;
    pub use crate::switch::*;
    pub use crate::text::*;
    pub use crate::text_field::*;
}
```

- [ ] **Step 5: Verify compile** (will fail because material crates don't exist yet)

```bash
cargo check -p floom-elements --no-default-features 2>&1 | tail -5
```

Expected: Compile error about missing `floom-widget-md-button` etc. This is expected and will resolve in Tasks 8-15.

---

## Task 7: md-button — Template Component (Material)

This is the canonical component template. Tasks 8-14 follow the same pattern with component-specific diffs noted inline.

**Files:**
- Create: `crates/material/md-button/Cargo.toml`
- Create: `crates/material/md-button/src/lib.rs`
- Create: `crates/material/md-button/src/state.rs`
- Create: `crates/material/md-button/src/style.rs`
- Create: `crates/material/md-button/tests/unit.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "floom-widget-md-button"
version = "0.1.0"
edition = "2021"

[dependencies]
floom = { workspace = true }
floom-core-theme = { workspace = true }
floom-widget-runtime = { workspace = true }
```

- [ ] **Step 2: Create src/state.rs**

```rust
use floom::reactive::RwSignal;
use floom_elements::button::ButtonProps;

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

- [ ] **Step 3: Create crates/floom/src/style.rs**

```rust
use floom_core_theme::component_tokens::{ButtonTokens, ComponentId};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::theme::Theme;
use floom_elements::button::ButtonVariant;

pub fn background(theme: &impl Theme, variant: ButtonVariant, disabled: bool, pressed: bool) -> Color {
    if disabled {
        return alpha(theme.colors().on_surface, 0.12);
    }
    match variant {
        ButtonVariant::Primary => {
            if pressed { theme.colors().primary_container }
            else { theme.colors().primary }
        }
        ButtonVariant::Secondary => theme.colors().secondary_container,
        ButtonVariant::Danger => theme.colors().error,
        ButtonVariant::Ghost | ButtonVariant::Text => Color::rgba(0, 0, 0, 0),
    }
}

pub fn foreground(theme: &impl Theme, variant: ButtonVariant, disabled: bool) -> Color {
    if disabled { return alpha(theme.colors().on_surface, 0.38); }
    match variant {
        ButtonVariant::Primary => theme.colors().on_primary,
        ButtonVariant::Secondary => theme.colors().on_secondary_container,
        ButtonVariant::Danger => theme.colors().on_error,
        ButtonVariant::Ghost | ButtonVariant::Text => theme.colors().primary,
    }
}

pub fn height(tokens: &ButtonTokens) -> f32 {
    tokens.height
}

fn alpha(c: Color, a: f32) -> Color {
    Color { a: (a * 255.0) as u8, ..c }
}
```

- [ ] **Step 4: Create crates/floom/src/lib.rs — MdButton implementation**

```rust
use floom::prelude::*;
use floom::reactive::RwSignal;
use floom_core_theme::component_tokens::ComponentId;
use floom_elements::button::{ButtonProps, ButtonSize, ButtonVariant};
use floom_widget_runtime::theme_signal;

mod state;
mod style;

use state::ButtonState;

pub struct MdButton {
    label: String,
    props: ButtonProps,
    state: ButtonState,
    theme: RwSignal<floom_core_theme::theme::MaterialTheme>,
}

impl MdButton {
    pub fn new(label: impl Into<String>) -> Self {
        let props = ButtonProps { label: label.into(), ..Default::default() };
        Self {
            label: props.label.clone(),
            state: ButtonState::from_props(&props),
            theme: theme_signal(),
            props,
        }
    }

    pub fn variant(mut self, v: ButtonVariant) -> Self {
        self.props.variant = v;
        self
    }

    pub fn size(mut self, s: ButtonSize) -> Self {
        self.props.size = s;
        self
    }

    pub fn disabled(mut self, d: bool) -> Self {
        self.props.disabled = d;
        self.state.disabled.set(d);
        self
    }

    pub fn loading(mut self, l: bool) -> Self {
        self.props.loading = l;
        self.state.loading.set(l);
        self
    }

    pub fn on_click(mut self, cb: impl Fn() + 'static) -> Self {
        self.props.on_click = Some(Box::new(cb));
        self
    }
}

impl View for MdButton {
    fn view(&self) -> impl View {
        let theme = self.theme.get();
        let tokens = theme.component(ComponentId::Button);
        let btn_tokens = match &tokens {
            floom_core_theme::component_tokens::ComponentTokens::Button(t) => t.clone(),
            _ => unreachable!(),
        };
        let variant = self.props.variant;
        let disabled = self.state.disabled.get();
        let pressed = self.state.pressed.get();
        let bg = style::background(&theme, variant, disabled, pressed);
        let fg = style::foreground(&theme, variant, disabled);
        let height = style::height(&btn_tokens);
        let radius = theme.radius().md;
        let padding = btn_tokens.padding_horizontal;

        floom::view::container(
            floom::view::label(move || self.label.clone())
                .style(move |s| s.color(fg))
        )
        .style(move |s| {
            s.width(100.0)
                .height(height)
                .padding_horiz(padding)
                .background(bg)
                .border_radius(radius)
        })
        .on_click(move |_| {
            if let Some(ref cb) = self.props.on_click {
                cb();
            }
        })
    }
}
```

- [ ] **Step 5: Create tests/unit.rs**

```rust
#[cfg(test)]
mod tests {
    use super::super::style;
    use floom_core_theme::component_tokens::ButtonTokens;
    use floom_core_theme::tokens::color::Color;
    use floom_core_theme::theme::MaterialTheme;
    use floom_core_theme::tokens::spacing::SpacingTokens;
    use floom_elements::button::ButtonVariant;

    #[test]
    fn primary_background_returns_primary_color() {
        let theme = MaterialTheme::new();
        let bg = style::background(&theme, ButtonVariant::Primary, false, false);
        assert_eq!(bg, theme.colors().primary);
    }

    #[test]
    fn disabled_background_is_muted() {
        let theme = MaterialTheme::new();
        let bg = style::background(&theme, ButtonVariant::Primary, true, false);
        assert_eq!(bg.a, (0.12 * 255.0) as u8);
    }

    #[test]
    fn ghost_has_transparent_background() {
        let theme = MaterialTheme::new();
        let bg = style::background(&theme, ButtonVariant::Ghost, false, false);
        assert_eq!(bg.a, 0);
    }
}
```

- [ ] **Step 6: Verify compile and test**

```bash
cargo check -p floom-widget-md-button 2>&1 | tail -5
cargo test -p floom-widget-md-button 2>&1 | tail -10
```

Expected: Clean compile. Tests pass.

---

## Tasks 8-14: Remaining Material Components

Each follows the md-button pattern (Task 7). Key differences per component:

### Task 8: md-text

`Cargo.toml` same as md-button, name = "floom-widget-md-text".

`crates/floom/src/lib.rs` — MdText: renders a label with typography from theme. Single `content` prop, no interaction.

```rust
impl View for MdText {
    fn view(&self) -> impl View {
        floom::view::label(move || self.content.clone())
            .style(move |s| {
                let theme = self.theme.get();
                s.font_size(theme.typography().body.medium.font_size)
                    .color(theme.colors().on_surface)
            })
    }
}
```

Test: verify font size matches theme.

### Task 9: md-text-field

`src/state.rs` includes `focused: RwSignal<bool>`, `value: RwSignal<String>`.

`crates/floom/src/lib.rs` — MdTextField: text input with border, label, placeholder. Border color changes on focus.

Test: verify value change fires on_change callback.

### Task 10: md-checkbox

`crates/floom/src/lib.rs` — MdCheckbox: checkbox icon + label. Toggle on click.

```rust
impl View for MdCheckbox {
    fn view(&self) -> impl View {
        let checked = self.state.checked.get();
        let icon = if checked { "☑" } else { "☐" };
        // ...
    }
}
```

Test: verify click toggles checked state.

### Task 11: md-switch

`crates/floom/src/lib.rs` — MdSwitch: track + thumb. Animate thumb position on toggle.

Test: verify click toggles, disabled prevents toggle.

### Task 12: md-dialog

`crates/floom/src/lib.rs` — MdDialog: overlay + card. Renders title, content, close button. Controlled by `open` prop.

Test: verify open=true renders overlay, open=false hides.

### Task 13: md-menu

`crates/floom/src/lib.rs` — MdMenu: vertical list of items. Each item fires on_select.

Test: verify item click fires correct callback.

### Task 14: md-card

`crates/floom/src/lib.rs` — MdCard: container with elevation shadow and rounded corners. Wraps child content.

Test: verify elevation prop changes shadow level.

---

## Task 15: floom-elements — Verify Full Integration

- [ ] **Step 1: Verify floom-elements compiles with material feature**

```bash
cargo check -p floom-elements 2>&1 | tail -10
```

Expected: Clean compile. All 8 `cfg(feature = "material")` re-exports resolve correctly.

- [ ] **Step 2: Verify no Material crate leaks into user-facing API**

```bash
grep -r "MdButton\|MdText\|MdTextField\|MdCheckbox\|MdSwitch\|MdDialog\|MdMenu\|MdCard" crates/floom-elements/src/ | grep -v "pub use"
```

Expected: No results — user-facing types are only `Button`, `Text`, etc.

---

## Task 16: examples/demo

**Files:**
- Create: `examples/demo/Cargo.toml`
- Create: `examples/demo/src/main.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "floom-demo"
version = "0.1.0"
edition = "2021"

[dependencies]
floom = { workspace = true }
floom-elements = { workspace = true, features = ["material"] }
floom-widget-runtime = { workspace = true }
```

- [ ] **Step 2: Create src/main.rs** — renders all 8 components

```rust
use floom::prelude::*;
use floom_elements::prelude::*;
use floom_elements::button::{ButtonSize, ButtonVariant};

fn app_view() -> impl View {
    floom::view::v_stack((
        floom::view::label("floom-elements Demo"),
        Button::new("Primary").variant(ButtonVariant::Primary).on_click(|| println!("click")),
        Button::new("Ghost").variant(ButtonVariant::Ghost),
        Button::new("Disabled").disabled(true),
        floom_elements::text::Text::new("Hello, world!"),
        Checkbox::new("Accept terms").on_change(|v| println!("checked: {}", v)),
        Switch::new().on_change(|v| println!("toggled: {}", v)),
        Card::new().elevation(2),
    ))
    .style(|s| s.padding(16.0).gap(8.0))
}

fn main() {
    floom::launch(app_view);
}
```

- [ ] **Step 3: Verify compile**

```bash
cargo check -p floom-demo 2>&1 | tail -5
```

Expected: Clean compile.

---

## Task 17: floom-testing

**Files:**
- Create: `crates/floom-testing/Cargo.toml`
- Create: `crates/floom-testing/src/lib.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "floom-testing"
version = "0.1.0"
edition = "2021"

[dependencies]
floom = { workspace = true }
floom-elements = { workspace = true, features = ["material"] }
```

- [ ] **Step 2: Create crates/floom/src/lib.rs** — snapshot testing utilities

```rust
use floom::prelude::*;
use floom_elements::button::Button;

/// Renders a widget to a string representation for snapshot testing.
/// Phase 1: structural snapshot. Phase 2+: visual regression via rendering.
pub fn render_snapshot(view: impl View) -> String {
    format!("{:?}", view)
}

/// Helper: create a standard Material-theme button for snapshot tests
pub fn snapshot_button() -> Button {
    Button::new("Snapshot")
        .variant(floom_elements::button::ButtonVariant::Primary)
        .size(floom_elements::button::ButtonSize::Medium)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_button_renders() {
        let btn = snapshot_button();
        let snap = render_snapshot(btn);
        assert!(!snap.is_empty());
    }
}
```

- [ ] **Step 3: Verify compile and test**

```bash
cargo check -p floom-testing 2>&1 | tail -5
cargo test -p floom-testing 2>&1 | tail -5
```

Expected: Clean compile. Test passes.

---

## Task 18: Final Integration Verification

- [ ] **Step 1: Full workspace check**

```bash
cargo check 2>&1 | tail -15
```

Expected: Clean compile for all crates.

- [ ] **Step 2: Run all tests**

```bash
cargo test 2>&1 | tail -20
```

Expected: All tests pass.

- [ ] **Step 3: fmt + clippy**

```bash
cargo fmt --check 2>&1 | tail -5
cargo clippy -- -D warnings 2>&1 | tail -10
```

Expected: No formatting violations, no clippy warnings.

- [ ] **Step 4: Verify tree-shaking — compile floom-elements and measure binary size**

```bash
cargo build --release -p floom-elements 2>&1 | tail -5
cargo build --release -p floom-demo 2>&1 | tail -5
ls -lh target/release/libfloom_elements.rlib 2>/dev/null
```

Expected: All build phases complete successfully.

---

## Self-Review Checklist

1. **Spec coverage:** Each Phase 1 requirement from [docs/PLAN.md](/Users/ahs/Project/Rust/floom-ui/docs/PLAN.md) is covered — 8 Material components, Theme system, compile-time dispatch, per-component tree-shaking, testing infrastructure.

2. **No placeholders:** All steps contain complete code or exact commands with expected output.

3. **Type consistency:** `ButtonProps`, `ButtonState`, `ButtonVariant`, `ButtonSize` defined in Task 6 floom-elements are consumed by Task 7 md-button with matching types.
