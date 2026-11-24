<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# 组件与设计系统

三道墙：**能画**（有实现）、**合规**（只用 token、状态齐全）、**可信**（有快照/语义断言）。
当前问题集中在第二道墙。

## 1. 分层清单

| 层 | 位置 | 规模 | 内容 |
| --- | --- | --- | --- |
| Tier-0 原语与控件 | `crates/widgets/floom-widgets/src` | 22 个顶层模块 + `gesture/`、`navigation/` | `primitives`、`controls`、`forms`、`text_input`、`surface`、`overlay`、`focus`、`canvas`、`ripple`、`svg_path`、`vector_icon`、`circular_progress`、`progress_linear`、`native`、`compose`、`widget_any`、`gesture`、`navigation`、`testkit` |
| 数据组件 | `crates/widgets/floom-widgets-data/src` | 5 模块 | `chart`、`data_grid`、`media`、`model`、`tree` |
| 跨 DS 复合组件 | `crates/ecosystem/floom-ui/crates/floom-ui-composites/src` | **48 个文件** | `accordion`、`badge`、`bottom_sheet`、`calendar`、`carousel`、`checkbox`、`chip`、`color_picker`、`combobox`、`counter`、`date_picker`、`dialog`、`menu`、`popup`、`radio`、`range_slider`、`select`、`slider`、`sparkline`、`stepper`、`tabs`、`text_field`、`time_picker`、`tooltip`、`tree_view`、`virtual_scroller`、`window_viewer` … |
| 设计系统 | `crates/ecosystem/floom-ui/crates/floom-{material,cupertino,fluent}` | **各 100 个 widget 文件** | 三套 DS 同构：同一批组件名 × 3 种视觉语言 |
| 主题与 token | `floom-core-theme`（`component_tokens/`、`global_theme`、`layout`、`props/`、`tokens/`、`theme.rs`）、各 DS 的 `tokens.rs`（material 342 行 / 27 个 pub 符号） | — | token 是唯一合法颜色/尺寸来源 |
| 变体与画廊 | `floom-design`（`gallery`、`style`、`variant`） | — | 变体定义与 `/components` 画廊 |

保留场景 `crates/core/floom-scene` 已支持 `ClipPush/ClipPop`、`GroupPush{transform}/GroupPop`、`Rect{fill,radius,shadow}`、`Path`、渐变与滤镜 —— 所以"没有裁剪/没有子树变换"是**旧文档的过时结论**，不要再照抄。

## 2. 设计系统一致性（可量化）

**事实**：颜色字面量全部落在组件的 `widgets/` 目录里，`tokens/` 目录干净，`floom-ui-composites` 零处。

| crate | 硬编码颜色处数（`Color::rgb|rgba|srgb`、`rgb8(`、`srgb_u8(`） | 位置 |
| --- | --- | --- |
| `floom-material` | **26** | `src/widgets/` |
| `floom-cupertino` | **29** | `src/widgets/` |
| `floom-fluent` | **27** | `src/widgets/` |

样本：`floom-material/src/widgets/navigation_drawer.rs:146`（`with_alpha(Color::rgba(0,0,0,255), SHADOW_ALPHA)`）、`date_picker.rs:173`、`radio_group.rs:129`（`PaintColor::rgba(0,0,0,0)`）、`floom-cupertino/src/widgets/rating.rs:158`（`Color::rgb(255,204,0)`——品牌黄被钉死在组件里）。

后果不是"看着不像"，而是：暗色模式与换肤要整树重绘（token 变了组件不读），且三套 DS 无法共享同一段状态逻辑。

## 3. 组件契约（每个组件都必须过的 10 条）

1. **颜色只来自 token**：组件源码里不允许出现颜色字面量（`tokens.rs` 白名单外）。
2. **状态全覆盖**：`default / hover / pressed / focused / disabled / loading / error / selected`，缺哪个要在文档里写明"不适用"。
3. **键盘可达**：Tab 到达、Enter/Space 激活、Esc 关闭、方向键在组内移动。
4. **焦点环**：由宿主绘制（`FocusRing::DEFAULT` = 2 px、`rgba(26,115,232,235)`、四条填充矩形），只在键盘导航后出现——键盘用户看得见，鼠标用户看不见。设计系统要自绘就 `set_focus_ring(None)`；无论谁画，环必须可见且不靠颜色单独表达（对比度）。
5. **语义**：`role` + `label`（或 `labelled_by`），状态映射到 `SemanticsState`（checked/expanded/disabled/busy）。
6. **尺寸与密度**：只用 token 的 spacing/size 档，不写字面 px。
7. **动效走 motion token**：时长/曲线来自 token，尊重"减少动态效果"。
8. **RTL/镜像**：`start/end` 语义，不用 `left/right`。
9. **快照**：至少一个 `floom-snapshot` 或 golden 断言，覆盖至少两个状态。
10. **无 panic 路径**：非法组合（空 items、越界 index）返回降级态，不 `unwrap()`。

## 4. 建议门禁（最小可执行）

```sh
bash scripts/gate.sh style   # 三个 DS 的 src/widgets/** 出现颜色字面量即失败（2026-11-21 起在 CI 与本地门禁里）
# 四层 token 的读写规则见 tokens.md：组件可读任一层，不得自己发明值
bash scripts/gate.sh a11y    # 尚未存在：每个公开组件至少一条 semantics 断言
```

`style` 已生效：82 处字面量清零（36 透明、22 黑、14 状态色移进各 crate 的 `src/tokens.rs`），
`scripts/check-design-tokens.sh` 对注入的 `Color::rgb(1,2,3)` 返回 1。
违反现状：#2、#3 无强制；#5 只有部分断言；#9 有 snapshot 能力但未逐组件覆盖。
**下一步**：补语义断言（每个组件一条，复用 `floom-accuracy` 的 `Semantics` 命令）。

## 5. 缺口

| 缺口 | 说明 |
| --- | --- |
| 三套 DS 不共享逻辑 | 100 × 3 份同构文件，一致性靠复制同步；状态机（toggle/expand/selection）在 DS 之外没有单一实现 |
| 缺少"组件清单即契约"的机读表 | `floom-accuracy::agent::schema::component_schema()` 已能生成 `ComponentSchema`（slots/properties/events/tokens/example），但没人拿它校验组件是否合规 |
| 自绘组件的语义缺位 | Canvas/Path 上的内容无法声明语义角色 |
| 自绘规模的性能门 | 无"每帧 1000 个 path 节点"这类基准 |
| 组件级 a11y 与几何 | 语义树有 label，但平台侧 frame 为零（见 [platforms.md](platforms.md) §5），所以"有 label"并不等于"能读" |

## 6. 与设计系统定位的关系

`floom-ui` 是**卫星 workspace**，不在 1.0 锁定面（`docs/vision.md` 非目标）。但"组件合规"是产品竞争力的一部分：三套 DS 是"集竞品之长"的可见面，不修 token 干净度，性能与一致性两条都拿不到分。
