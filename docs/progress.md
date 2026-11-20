<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# 推进计划与进度表

[roadmap.md](roadmap.md) 说"还差什么"，本文件说"**现在做到哪、下一步动什么、什么算做完**"。
两者分工：roadmap 是目标，本文件是执行台账。任何"已完成"必须附一条可重跑的命令与它的退出码。

## 1. 三条不动摇（长期执行时唯一不许讨价还价的东西）

1. **判据不降级**：性能/资源不许用"和上一版比"代替"和竞品比"；平台状态不许用"能编译"代替"实测"（`pass` 之外的词一律写 `fail` 或 `n/a`，`SKIP ≠ PASS`）。
2. **一次只开一条主线**：任何时刻只有一行 `进行中`。第二条主线要等第一条出 `完成` 或 `放弃(理由)`。
3. **每一步自带退出门**：没有验证命令的步骤不许开工。验证命令先于代码存在——否则它只是愿望。

## 2. 停手条件（出现即停下来问，不许自行降级）

- 某步的验证需要在**我们没有的设备/账号**上跑（新平台、商店、CI runner）：停下来要决定，不许用本地模拟器结果假装。
- 某步的 diff 超出它自己的退出判据范围（顺手重构、顺手改风格）：停手，单独立项。
- 两处实现二选一的决策（如 OHOS 后端）：写进文档的一个决定，不许"两个都留着"。

## 3. 计划：6 个阶段、18 步

序号 `S<阶段>.<步>`。**退出判据**决定"做完"，**验证命令**决定"谁说做完"。

### S0 让事实可测（一切的前置）

| ID | 动作 | 退出判据 | 验证命令 |
| --- | --- | --- | --- |
| S0.1 | 把仓库镜像到 CI 可跑的宿主（GitHub）并推现有 workflow，或改用可访问 runner | 一条公开链接的绿 build：macOS + Linux + Windows 三系统 `cargo test` | 打开那条 build 的 URL（必须是别人也能点的链接） |
| S0.2 | 把 `deny`/`perf`/`size` 接进同一套门禁口径（`gate.sh --list` 与实际 CI 步骤一致） | `gate.sh --list` 与 CI 步骤集合相同，缺一即红 | `bash scripts/gate.sh --list`，逐条比对 workflow |

### S1 平台 binding 从"声明"到"实测"

| ID | 动作 | 退出判据 | 验证命令 |
| --- | --- | --- | --- |
| S1.1 | **OHOS 后端选型**：复活 `floom-winit-ohos`（有事件循环/输入/IME）或补齐 `floom-platform-harmony`（补输入导出） | `platforms.md` §4 写下一个决定 + 一条能收 touch 的路径；另一半代码删除或明确标注 deprecated | `grep -rn "floom-winit-ohos" crates/` 只剩一处实现路径 |
| S1.2 | OHOS 上屏 5 个缺陷：早退不 `AbortBuffer`、失败仍 `Flush`、三 atomic 非成组、尺寸精确相等、`stride` 单位 | 五个缺陷各自有注释指明修法；`stride` 单位有上机结论 | 上机：连续 60 s 无丢帧；日志无 `AbortBuffer missing` |
| S1.3 | iOS CJK：字体扫描/注册（照 `crates/app/floom/src/platform/ohos_font.rs` 配方） | `你好世界` 首帧非 tofu | `idb screenshot` 与基线截图比对 |
| S1.4 | iOS a11y：把布局帧传给 `UIAccessibilityElement` | `idb ui describe-all` 的 frame 非零 | `idb ui describe-all`，与 `semantics` 树的 bounds 对齐 |
| S1.5 | Android：a11y `perform_action` 真实现 + 聚焦元素可见（焦点环） | `gates/android.md` item3/item7 由 `fail`/`partial` 转 `pass` | 设备上 TalkBack 能点击；截图见焦点环 |
| S1.6 | 桌面：剪贴板非 macOS 实现 + AT-SPI/UIA 桥 + `file-picker` trait | `gates/linux.md`/`windows.md` 首项转 `pass` | 三平台 `cargo test -p floom-platform-desktop` + 手工复制粘贴 |
| S1.7 | 平台状态表改真：把达标的 target 从 `device-gated` 改 `tested` | ≥3 target `tested` 含 ≥1 移动端（G1 达成） | `grep -c "tested" docs/release-policy.md`；`gates/*.md` 无 `fail` |

### S2 契约与门禁（锁住已有优势）

| ID | 动作 | 退出判据 | 验证命令 |
| --- | --- | --- | --- |
| S2.1 | 插件 `abi_version` 字段 + 运行期校验 | 旧 provider 被拒并提示"重新编译插件" | `cargo test -p floom-plugin-*`，含一条用旧 ABI 的负例 |
| S2.2 | 资源门禁（RSS / 逐帧分配 / GPU 时间） | `just gate resources` 能绿能红 | 故意把阈值调小 → 红；恢复 → 绿 |
| S2.3 | 清零 82 处硬编码颜色（material 26 / cupertino 29 / fluent 27） | 三个 `*/src/widgets/` 目录 0 处 `Color::rgb*`；`just gate style` 存在且绿 | `rg -c "Color::r(gb\|gba)\|srgb_u8" crates/ecosystem/floom-ui/crates/*/src/widgets/` |
| S2.4 | 组件契约门禁：`ComponentSchema` 校验 token/属性/事件齐全 | 门禁能对新增组件报错 | `just gate components` 对一个故意缺 token 的组件变红 |

### S3 开发者表面

| ID | 动作 | 退出判据 | 验证命令 |
| --- | --- | --- | --- |
| S3.1 | CLI `check`/`test` 从 cargo 转发变成真门禁子集 + `doctor` | `floom check` 在坏项目上报出具体问题；`floom doctor` 报环境矩阵 | `cargo test -p floom-cli`，含坏项目 fixture |
| S3.2 | 热重载保留状态 + 记录延迟 | 改一个颜色 ≤2 s 见到；组件状态不丢 | 手工 + 延迟数字写进 release notes |
| S3.3 | DevTools 计算值 + hover 高亮接真实 window loop | Style 行显示计算值；hover 高亮在真窗口可用 | `cargo run -p floom-showcase`，F12 逐项点 |

### S4 AI-first（护城河，不许拖到最后）

| ID | 动作 | 退出判据 | 验证命令 |
| --- | --- | --- | --- |
| S4.1 | 把 agent 口接进真实 `floom run` 进程（Task 25.3） | 真实 app 上 `Tree`/`Semantics`/`Screenshot`/`Tap` 可用，默认关闭 | `floom run --agent-port N` + 客户端脚本跑通 4 命令 |
| S4.2 | 稳定语义 id（不暴露 NodeId） | 同一 UI 两次运行选择器解析到同一节点 | 两次运行的 `Semantics` 输出 diff 为空 |
| S4.3 | `floom mcp`：把上面 4–6 个能力包成 MCP 工具 | agent 编辑器里能直接"点按钮、读树、截图" | MCP 客户端（Claude/Cursor）实测一段脚本 |
| S4.4 | 失败工件成套：testkit 失败同时产出树 + 语义 + PNG + trace | 任何断言失败都附四件 | 故意断言的负例产出工件目录 |
| S4.5 | token 分层文档（`llms.txt` 15 分钟版 / 1 小时版 / 全量） | 三档各自可独立读通 | 让 agent 只读 15 分钟版写出一个能跑的窗口 |

### S5 生态与发布

| ID | 动作 | 退出判据 | 验证命令 |
| --- | --- | --- | --- |
| S5.1 | 兼容矩阵 + `flameo` + crate 命名决定（`floom` 已被占用） | matrix 有真实单元格；`flameo` 存在或被删声明 | `cargo test -p floom --test docs_parity`；README 无死链 |
| S5.2 | G2 版本承诺 + breaking 变更流程 | `release-policy` 写死 semver 面；有一个 breaking 的示范流程 | 走一遍流程，产出 PR 模板 |
| S5.3 | G9 GPU 像素测试进 CI | 不再 `#[ignore]` | CI 日志里出现该 job |
| S5.4 | 发布记录表（四判据/体积/测试数/平台状态/延迟/帧预算） | 每版一行，有数字 | `docs/performance.md` 的记录表有 ≥1 行 |

## 4. 进度表

状态词只有六个：`未开始`、`进行中`、`待验证`、`完成`、`阻塞(原因)`、`放弃(理由)`。
**只追加、不改历史行**：重开一条就加新行，旧行留作记录。`证据` 列放命令或文件路径，不放形容词。

| ID | 事项 | 状态 | 证据 | 更新于 |
| --- | --- | --- | --- | --- |
| D.1 | 文档重整（26 篇 / 6,371 行 → 14 篇 / 1,537 行，15 篇归档） | 完成 | `cargo test -p floom --test docs_parity` → 7 passed；`floom-wip/2026-11-20-01-docs-consolidation/{MANIFEST.md,docs-parity.log,link-check.log}` | 2026-11-20 |
| D.2 | 代码图谱（graphify） | 完成 | `graphify-out/graph.json`：38,933 节点 / 87,985 边 / 1,538 社区；`GRAPH_REPORT.md` | 2026-11-20 |
| A.0 | 代码审计：OHOS / android+ios binding / 插件 / 组件风格 | 完成 | `docs/platforms.md`（OHOS 7 + iOS 3 + Android 3 条）、`docs/plugins.md` §3、`docs/components.md`（82 处） | 2026-11-20 |
| S0.1 | CI 真的会跑 | 未开始 | — | — |
| S0.2 | 门禁口径一致 | 未开始 | — | — |
| S1.1 | OHOS 后端选型 | 未开始 | 决策点已写在 `docs/platforms.md` §4 | — |
| S1.2 | OHOS 上屏 5 缺陷 | 未开始 | 缺陷清单：`docs/platforms.md` §4 | — |
| S1.3 | iOS CJK 字体 | 未开始 | 配方：`docs/platforms.md` §5（`ohos_font.rs`） | — |
| S1.4 | iOS a11y frame | 未开始 | 证据：`docs/gates/ios.md` item3 `fail` | — |
| S1.5 | Android a11y + 焦点环 | 未开始 | 证据：`docs/gates/android.md` item3/item7 | — |
| S1.6 | 桌面剪贴板 / a11y / file-picker | 未开始 | `crates/platform/floom-platform-desktop/src/{clipboard,accessibility}.rs` | — |
| S1.7 | ≥3 target tested（G1） | 未开始 | `docs/release-policy.md` targets 表 | — |
| S2.1 | 插件 `abi_version` | 未开始 | 全仓 0 命中 `abi_version` | — |
| S2.2 | 资源门禁（G12） | 未开始 | `gate.sh --list` 无 resources | — |
| S2.3 | 组件 token 清零 + 门禁 | 未开始 | 82 处（26/29/27） | — |
| S2.4 | 组件契约门禁 | 未开始 | `component_schema()` 无消费者 | — |
| S3.1 | CLI check/test/doctor | 未开始 | `crates/app/floom-cli/src/main.rs`（cargo 转发） | — |
| S3.2 | 热重载保留状态 | 未开始 | 文件级热重启 | — |
| S3.3 | DevTools 计算值 | 未开始 | Style 行无计算值 | — |
| S4.1 | agent 口进真实 app | 未开始 | Task 25.3 未做 | — |
| S4.2 | 稳定语义 id | 未开始 | 只有 `Label`/`Role` 选择器 | — |
| S4.3 | `floom mcp` | 未开始 | 无该子命令 | — |
| S4.4 | 失败工件成套 | 未开始 | snapshot 有 PNG，无 trace 绑定 | — |
| S4.5 | token 分层文档 | 未开始 | `llms.txt` 单档 | — |
| S5.1 | 兼容矩阵 / `flameo` / 命名 | 未开始 | `docs/roadmap.md` G10 | — |
| S5.2 | G2 版本承诺 | 未开始 | 0.2.0 | — |
| S5.3 | G9 像素测试进 CI | 未开始 | `#[ignore]` | — |
| S5.4 | 发布记录表 | 未开始 | 表空 | — |

## 5. 更新规则（这份文件怎么活）

1. **每次会话结束**更新一次：状态、证据、日期。没动就写"未开始"，不许把"想清楚了"记成"进行中"。
2. **`完成` 三条件**：验证命令跑过、退出码贴在同一行证据里、退出判据逐条对上。少一条就只能是 `待验证`。
3. **`阻塞` 必须写原因与解除条件**，例如 `阻塞(等 GitHub 镜像权限)`，不许只写"阻塞"。
4. 偏差发现时**先改本表再改代码**：把新认识写进对应步骤的退出判据，再动手——这样跑偏的是表，不是实现。
5. 阶段顺序只在一种情况下调整：后面的步骤变成前面步骤的**前置**（例如 S0.1 不通过，S1.7 的 `tested` 无法被外部验证）。调整要写一行理由。
