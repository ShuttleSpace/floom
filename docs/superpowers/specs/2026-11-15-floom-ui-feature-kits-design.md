# floom-ui：用 per-component feature 合并 design kit

日期：2026-11-15
状态：**待批准**（设计文档，未动产品代码）

## 意图与 why

`crates/ecosystem/floom-ui` 目前是独立 satellite workspace，306 个 member。每个 design
system 被拆成 ~99 个「每组件一 crate」（`md-button` / `cupertino-button` / `fluent-button`…）
外加一个薄 kit crate（`floom-material` / `-cupertino` / `-fluent`）。后果：消费者要在
Cargo.toml 里列几十上百行依赖——`examples/floom-demo` 就列了 **294 行** `floom-widget-*`
依赖，源码里还有 294 处 `floom_widget_md_xxx::` 路径引用。

目标：像 `web-sys` 那样，每个 design system 收敛成 **1 个 crate**，每个组件一个 feature，
默认全关、按需开启。消费者从「列 N 个 crate」变成「1 行依赖 + 几个 feature」，且未开启的
组件代码根本不编译。

## 目标 / 非目标 / 反目标

- 目标：306 member → ~9；每个 DS 一个 crate；per-component feature 裁剪；删掉 `floom-harmony`。
- 目标：保住现有全部 ~293 个组件测试，且门禁必须继续跑它们（不能静默放水）。
- 目标：`examples/floom-demo` / `floom-showcase` 迁移到新依赖后能编译运行。
- 非目标：不改组件的行为、绘制、令牌取值；纯物理搬运 + 路径改写。
- 非目标：不把共享依赖（floom-core 等）做成 optional。`#[cfg]` 裁剪的是本 crate 的模块，
  共享依赖仍无条件编译——这是 web-sys 模型，不是 link-time 依赖消除。
- 非目标：不重写 `docs/PLAN.md`、`docs/COMPONENT_PARITY.md` 全文，也不改 `scripts/scaffold_*.py`
  / `swap_*.py`（一次性历史生成脚本，已执行完，留作记录）。
- 反目标：不能让 `cargo test --workspace` 在没有 feature 时「全绿但什么都没测」。

## 约束（已核实的事实）

| 事实 | 影响 |
|---|---|
| 组件 crate 实名 `floom-widget-{md,cupertino,fluent}-*`，目录名带前缀 | 合并后模块名要去前缀、连字符转下划线 |
| 每个组件 crate = 1 个 `src/lib.rs` + 1 个 `tests/*.rs`；`md-icon` 多一个 `icons.rs` | 可脚本化搬运 |
| 组件间交叉依赖只有 `icon` / `icon-paths`（~15 个组件），引用形如 `floom_widget_md_icon::MdIcon` | 合并后改 `crate::widgets::icon::…`，并用 feature 依赖 `checkbox = ["icon"]` 表达 |
| 薄 kit 已有 `mod button/gallery/scale/tokens`（令牌层），与组件 `button` 同名 | 组件必须放进独立命名空间 `widgets::`，避免撞名 |
| 薄 kit 依赖 `floom-design/style/theme/scene`；组件依赖 `floom-core/core-theme/signal/widgets/widget-runtime/ui-composites/accessibility`；**两层零互相引用** | 新 crate 的 `[dependencies]` 是两者并集，逻辑上仍是两层 |
| `floom-widget-runtime`、`floom-core-theme` 已有 `material/cupertino/fluent` feature；每个组件 crate 都写死 `features=["material"]` | 合并后单 crate 写一次，无条件启用 |
| 门禁 `gate.sh::ecosystem_check` 跑 `cargo test --workspace --no-fail-fast`，**无 `--all-features`**；CI `ci-plugins.yml` 同 | 必须加 `--all-features`，否则默认全关 → 组件测试全不跑 |
| 根 clippy 已是 `--all-features` | 一致，无需改 |
| `examples/floom-demo` 294 依赖 + 294 处引用；`floom-showcase` 3 依赖 + 6 处 | 迁移面，必须同步改 |
| 3 个 `floom-*` kit 与 `floom-harmony` 仓库内零消费方（除 examples 用组件层） | 删除 harmony 无内部断链 |
| `gate.sh` / `ci-plugins.yml` / `check-architecture-deps.sh` 的注释硬编码「306 members」「四个 design system」 | 数字断言要更新，否则注释变假 |

## Approaches（已选）

1. **全量 web-sys 式（已选）**：每个 vendor 的薄 kit + ~99 组件 crate → 1 个 crate，
   per-component feature，删 `floom-harmony`。306 → 3 个大 crate + 6 个共享 crate。
   *代价*：单次大 diff，tests/examples/门禁都要动。*收益*：Cargo.toml 从 294 行 → 3 行，
   feature 真裁剪编译量。
2. 只收薄 kit 层：删 3 个薄 crate 并入一个 facade，组件 crate 全留着。*否决*：
   crate 数几乎没减，没解决核心痛点。
3. 每个 DS 一个粗 feature（非 per-component）：改动小。*否决*：给不了组件级裁剪，
   不是用户要的 web-sys 模型。

## 已定决策（来自提问）

- 范围：**全量 web-sys 式**（方案 1）。
- 默认姿态：**`default = []` 全关** + 门禁/CI 加 `--all-features`。真按需裁剪；
  代价是要动一行 `gate.sh`、一行 `ci-plugins.yml`，并给 `floom-demo` 显式 `features=["full"]`。

## 设计细节

### 结果结构（以 material 为例）

`crates/floom-material/` 从「薄 kit」升级为「完整 kit」，保留 crate 名与位置：

```
crates/floom-material/
  Cargo.toml          # [features]: 每组件一条 + full + default=[]
  src/
    lib.rs            # 原令牌层 4 个 mod + `#[cfg(feature="button")] pub mod widgets` 内各组件
    button.rs gallery.rs scale.rs tokens.rs   # 原薄 kit，原样保留
    widgets/
      button.rs app.rs card.rs …             # 原 md-*/src/lib.rs
      icon/{mod.rs,icons.rs}                # md-icon 的两文件
  tests/
    button.rs app.rs …                      # 原 md-*/tests/*.rs
    gallery.rs motion.rs                    # 原 kit crate 自带，必须保留、不可覆盖
                                           # （三个 kit 都有 tests/gallery.rs，故不得扁平化；
                                           #   组件里无 gallery/motion 组件，不冲突）
```

- **crate 名**：`floom-material` / `floom-cupertino` / `floom-fluent`（沿用现名，
  消费者心智不碎）。**删** `crates/floom-harmony/` 及其 member 行。
- **模块路径**：`floom_material::widgets::button::MdButton`。组件放 `widgets::` 命名空间，
  与薄 kit 顶层的 `button_metrics` / `MaterialTheme` 等不冲突。
- **feature 名** = 去前缀的目录名、连字符→下划线：`md-button`→`button`、
  `md-radio-group`→`radio_group`、`md-text-field`→`text_field`、
  `cupertino-icon-paths`→`icon_paths`。`full` = 所有组件 feature 之和。
- **交叉依赖**：依赖 icon 的组件 feature 写成 `checkbox = ["icon"]`；依赖 icon_paths
  的写成 `icon_button = ["icon", "icon_paths"]`。映射由脚本从原 Cargo.toml 自动推导。
- **DS feature**：`floom-widget-runtime` / `floom-core-theme` 在 `[dependencies]` 里无条件带
  `features = ["material"]`（cupertino/fluent 同理）。整个 crate 就是该 DS，不按组件门控。
- **依赖并集**：薄 kit 的 5 个 + 组件层的 8 个，去重。`[dev-dependencies]` 保留
  `floom-snapshot` / `floom-app` / `floom-layout` 等现有测试依赖。

### tests 迁移

293 个 `tests/*.rs`：
1. 每个文件顶部加 `#![cfg(feature = "<组件 feature>")]`——默认全关时文件编译为空，
   `--all-features` 时全跑。比 293 条 `[[test]] required-features` 省事。
2. `floom_widget_md_button::` → `floom_material::widgets::button::`（三个前缀各一套映射）。
   注意与上文「组件内部交叉引用改 `crate::widgets::`」是**两套规则**：tests 是链接本 crate 的
   独立 crate，必须用外部路径 `floom_<ds>::widgets::`；组件 `src/` 合并进同一 crate 后是
   模块间引用，必须用 `crate::widgets::`（edition 2021 下 crate 不能以自身名引用自己）。

### examples 迁移

- `examples/floom-demo/Cargo.toml`：294 行依赖 → 3 行，
  各带 `features = ["full"]`。
- `examples/floom-demo/src` + `floom-showcase/src`：按上述前缀映射批量改 300 处路径。
  `floom-showcase` 只需 `features = ["text", "button", "checkbox"]`。

### workspace / 门禁 / CI / 注释

- `crates/ecosystem/floom-ui/Cargo.toml`：members 306 → ~9；删 `[workspace.dependencies]`
  里 3 条 `floom-widget-*-icon` path 定义（不再有独立 crate）。
- `scripts/gate.sh::ecosystem_check`：`cargo test --workspace --all-features --no-fail-fast`；
  注释里「306 members」改为新的真实数量。
- `.github/workflows/ci-plugins.yml::ecosystem-check`：同样加 `--all-features`，改注释数字。
- `scripts/check-architecture-deps.sh`：注释里 design system 列表去掉 `floom-harmony`。
- `Cargo.lock`（floom-ui 自有）：随 workspace 变化重新生成。
- `docs/PLAN.md` / `COMPONENT_PARITY.md` / `scripts/*`：不改（见非目标），但在实施收尾时
  用一行注释标注「架构已变，见本设计文档」，避免后人被旧描述误导。

### 实施顺序（每步一次 Conventional Commit，不 push）

1. 生成脚本 + 合并 material（一个 vendor 打通），`cargo test --all-features` 全绿。
2. 同法合并 cupertino、fluent。
3. 删 `floom-harmony`；收 workspace members / dependencies。
   **原子性**：删除 `crates/material/md-*` 等组件 crate 目录，与删除
   `Cargo.toml` 里 `[workspace.dependencies]` 的 `floom-widget-*-icon` 三条 path 定义，
   **必须在同一步内完成**。否则残留的 `workspace = true` 引用（md-rating / md-list_group /
   md-checkbox_btn / md-select / md-expansion_panels / md-stepper / md-speed_dial /
   md-confirm_edit 等）会 dangle 导致编译失败。
4. 迁移 examples（demo + showcase）。
5. 改门禁 / CI / 注释，跑完整 `gate.sh ecosystem` + `cargo check --workspace`（默认全关）
   + `cargo check --no-default-features --features floom-material/button`（证明单组件能编译）。
6. 提交本设计文档。

### 验证（缺一不可）

- `cd crates/ecosystem/floom-ui && cargo test --workspace --all-features --no-fail-fast`
  → 293 组件测试 + 令牌层测试全绿。
- `cargo check --workspace`（`default=[]`）→ 能编译，且刻意证明「不编译组件代码」。
- `cargo check -p floom-material --no-default-features --features button` → 单组件裁剪编译通过。
- 根 `cargo check -p floom-demo -p floom-showcase` → examples 迁移正确。
- `bash scripts/gate.sh ecosystem` → 门禁通过。

## Builder check 反馈（2026-11-15，独立 reviewer）

reviewer 按真实代码核对了设计，挑出以下需落实的点，已并入上文：

- **edition 陷阱（最关键）**：组件 crate 是 `edition = "2021"`；合并进的 kit crate
  解析 `edition.workspace = true` → `"2024"`。若 kit crate 保持 2024，~294 个组件文件会在
  2024 规则下重编译（`gen` 关键字保留、let-chain 优先级、`unsafe_op_in_unsafe_fn`
  deny-by-default 等），把"纯物理搬运"变成"294 文件整改"，完全超出本设计范围。
  **决定（my call）**：合并后的 kit crate 在 `Cargo.toml` 里写死 `edition = "2021"`
  （不走 workspace），与组件原地 edition 一致，保住纯搬运承诺。这是一行改动，但必须在
  生成脚本里显式设置，不能依赖 workspace 继承。
- **kit 自带 tests 不可丢**：`floom-material/tests/{gallery.rs, motion.rs}`、`floom-cupertino`
  / `floom-fluent` 各带 `tests/gallery.rs`，是真实 snapshot/golden 测试。迁移脚本必须保留它们，
  且因三个 kit 都有同名 `gallery.rs`，必须留在各自 kit 的 `tests/` 目录、**不得扁平化**。
  已确认没有组件叫 `gallery` / `motion`，故不冲突。
- **icon-paths 三套不对称**：material **没有** `md-icon-paths`（路径数据内嵌在
  `md-icon/src/icons.rs`），仅 cupertino / fluent 有 `*-icon-paths` → feature `icon_paths`。
  脚本必须按**每个组件 crate 自己的 `Cargo.toml`** 推导 feature 图，不得假设三套对称，
  否则 material kit 会凭空出现 dangling 的 `icon_paths` feature + 空 `widgets::icon_paths` 模块。
- **examples 验证**：迁移 `floom-showcase` 时，先 grep 它真实的依赖/引用集合，确认就是
  `[text, button, checkbox]`，不要直接套用文档假设。

reviewer 已确认文档下列判断属实、无需改动：组件代码零 `crate::`/`super::`/`include!`/
`#[macro_export]`/`build.rs` 引用；`--all-features` 下 `floom-core-theme` /
`floom-widget-runtime` 的 DS feature 用 OR 组合、三者同时开语义等价；`floom-ui-composites`
是无条件全局依赖且无回环；`floom-harmony` 仓库内零消费方。

## 留给 builder 的判断

- 具体 feature 依赖图（`checkbox = ["icon"]` 等）由脚本读原 Cargo.toml 生成，不手写。
- 组件模块命名冲突（如 `md-text` 与 `md-text-field`）按目录名机械化转换，不做特例。
- `floom-ui-composites` / `floom-accessibility` 等只被少数组件用的依赖：本轮不设 optional
  （见非目标）。若将来要进一步削共享依赖编译量，再按 `dep:` 绑定 feature。

## 风险

- **大 diff 不可逆**：一次性删 ~293 个 crate 目录。靠分步提交 + 每步验证兜底。
- **门禁放水**：这是最危险的坑。默认全关 + 忘了加 `--all-features` = 全绿但零测试。
  `gate.sh` 与 CI 两处都要改，验证里专门有一条 `cargo test --workspace --all-features`。
- **feature 漏映射**：某个组件依赖 icon 但漏写 `= ["icon"]`，只开该组件 feature 时会编译失败。
  验证里的「单组件编译」检查能抓到 `button` 这类无依赖的，但抓不到所有组合——脚本从
  原依赖自动推导 + `--all-features` 全编译是主要保障。
