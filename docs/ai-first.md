<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# AI first

定义不是"能生成代码"，而是 agent 对一次改动能闭环 **读 → 改 → 跑 → 验** 四件事，且每一步都是机器可判定的事实，不是语言描述。

## 已经有的（比文档原来承认的多）

底子在 `crates/system/floom-accuracy/src/agent/`（Task 25.1/25.2 的产物）：

| 东西 | 内容 |
| --- | --- |
| 协议 | `AgentRequest`/`AgentResponse` + `ProtocolVersion` + `check_version`（`protocol.rs`），长度前缀 JSON |
| 命令 | `Tree` / `Semantics` / `Scene` / `Screenshot` / `Performance` / `Schema`；动作 `Tap{selector}` / `TypeText{selector,text}` / `Drag{from,to}` |
| 选择器 | 只有 `Selector::Label(String)` 与 `Selector::Role(String)`，**故意不暴露 `NodeId`**（内部句柄，猜了就会操作到失效节点）。屏幕按无障碍的方式寻址 |
| 组件 schema | `ComponentSchema { version, widgets }`：每个 kind 的 `slots` / `properties`（带 `style|layout|widget` 分类与封闭值域 `domain`）/ `events` / `tokens` / **一段可运行的 `example`**（`schema.rs`） |
| 传输 | `local_channel()`（进程内）+ `spawn_loopback()`：**只绑 127.0.0.1**，非回环直接拒绝、dev token 门禁。理由写在 `transport.rs`：无鉴权调试口就是 RCE |
| 决定性验证 | `floom-snapshot` 的 PNG 像素哈希 + 场景/语义 manifest + `TestApp` |
| 文档入口 | 仓库根 `llms.txt`（已有，之前没人发现） |

## 缺的（按"闭环断在哪"排）

| # | 缺口 | 为什么是硬缺口 | 证据 |
| --- | --- | --- | --- |
| A1 | ~~跑起来的 app 没有 agent 口~~ **已闭（2026-11-23）**：`AgentApp` 抽成 trait，`WindowApp` 让协议直接驱动真实 `WidgetHost`，`spawn_bridge` 把 loopback 请求交给持有窗口的线程；`floom-window-software --agent <port>` 在 CI 里被脚本问出树、点中按钮 | — | 见下节「已闭的缺口」 |
| A2 | ~~**无 `floom mcp`**~~ **已闭（2026-11-24）**：`floom-mcp` 把五个工具（树/语义/点/输入/截图）包成 MCP，stdio + JSON-RPC；它只附到已在跑的 app，不启动进程 | harness 里不用再写胶水 | `crates/app/floom-mcp`、`tests/protocol.rs` 8 passed |
| A3 | ~~**语义动作 ID 不稳定**~~ **已闭（2026-11-24）**：`Selector::Key("increment")` 按作者声明的名字定位，跨位置、跨文案、跨列表重排都不变；`Selector::Id` 仍可用但不持久 | 一次 i18n 改动不会再让脚本报废 | `Child::named` / `WidgetHost::set_agent_key` + `tests/agent_keys.rs` 4 passed |
| A4 | ~~失败工件不成套~~ **已闭（2026-11-24）**：拒绝的命令现在落一目录证据；像素 golden 失败会写出 `-actual.png` | 见「已闭的缺口」 | `floom-accuracy/src/agent/session.rs`、`floom-snapshot/src/lib.rs` |
| A5 | ~~**token 分层文档**~~ **已闭（2026-11-24）**：四层（全局主题 → 设计系统 → 组件表 → 实例 prop）、各层谁能写、改一层会动什么，以及门禁**不**检查什么 | agent 不必再猜"能不能直接写颜色" | [tokens.md](tokens.md)、`gate style` |
| A6 | ~~**CLI 没有 agent 子命令**~~ **已闭（2026-11-24）**：`floom verify`（fmt → check → test 一条命令一个退出码）与 `floom snapshot [--update]`（只跑快照测试） | agent 不必再手敲三条 cargo 命令 | `floom-cli/src/main.rs` 的 `VERIFY_STEPS`/`SNAPSHOT_STEPS` |

### A5 token 分层（2026-11-24）

[`docs/tokens.md`](tokens.md) 把"值可以从哪来"写成四层：`floom-theme` 的七个颜色角色 → 各设计系统 `tokens.rs` 的额外角色（Material 十九个、`INFO`/`SUCCESS`/`WARNING`）→ `component_tokens/<widget>.rs` 的每组件度量 → 实例 prop。规则一句话：**组件可读任一层，不得自己发明值；单点改动是 prop，不是 token。**

同一节里写了改哪一层会动什么（全局 → 三个系统都跟；设计系统 → 只有它自己变，于是差异在 diff 里看得见；组件表 → 该组件的全部实例；widget 里的字面量 → 什么都不跟，下一次换主题就漏掉），以及门禁的边界：`gate style` 只拦三个系统的 `src/widgets/**` 里的颜色字面量（注入验证过），**不**检查组件是否绕过组件表直接读设计系统层，也**不**检查 `tokens.rs` 里有没有没人用的常量——两者都需要真正的 token 图。

### A4 失败工件成套（2026-11-24）

两条路各补一半，都不改变失败本身：

- **命令被拒**（agent 侧）：`AgentSession::with_artifact_dir(dir)` 后，每条失败的请求写 `dir/failure-NN/`，内含 `command.json`（请求 + 拒绝原因）、`tree.json`、`semantics.json`、`screenshot.png`（真实像素，`PixelSnapshot` → PNG）、`README.md`（说明每个文件是什么）。编号取自目录（不是计数器），因为桥每条连接一个 session，两个不能互相覆盖。写失败只在 stderr 说一句，原错误原样返回——工件不能掩盖失败。
- **像素 golden 变了**（测试侧）：`floom_snapshot::assert_pixels(&snapshot, manifest_dir, name)` 把 golden 存成同名 `.png`，不符时把失败的那张写成 `<name>-actual.png`，panic 消息直接说"golden 在 X，失败的那张在 Y"。文本 golden 本来就有行 diff，像素没有，所以证据就是两张图。

证据：`crates/system/floom-accuracy/tests/agent_artifacts.rs`（3 个测试：拒绝留下五件、成功什么都不写、两次失败各有编号）、`crates/devtools/floom-snapshot/tests/pixel_golden.rs`（2 个：bless 后同图通过且不留文件；变了的图 panic 消息点名 `-actual.png` 且文件是 PNG）。

未做：把 agent 工件接进 `floom test`/`floom run` 的默认行为（要定"默认目录"与清理策略），以及 trace（现在只有树/语义/像素，没有请求时间线）。

## 已闭的缺口

**A3：作者声明的 key。** `ChildKey` 是有哈希的、读不回来，所以名字按原样存在节点上（`Child::named("increment")` 或 `WidgetHost::set_agent_key`），`Tree` 每条节点带 `key`，`Selector::Key("increment")` 就能点到它。`tests/agent_keys.rs` 证明四件事：命名的子节点在树里报出自己的 key、按 key 能点中、**同一个 key 在"第一个"与"第二个"位置都解析成功**（位置无关）、未知 key 被拒并列出屏上 label。位置型 `Selector::Id` 保留但不再被文档推荐为持久句柄。

### A2 MCP server（2026-11-24）

`crates/app/floom-mcp` 是**适配器**，不是启动器：它接一个已经在跑 agent 协议的 app（`floom-window-software --agent 4877`），把 `Tree`/`Semantics`/`Tap`/`TypeText`/`Screenshot` 包成五个 MCP 工具（`floom_tree`、`floom_semantics`、`floom_tap`、`floom_type`、`floom_screenshot`），传输是 stdio + 换行分隔的 JSON-RPC 2.0。

三条边界写进了代码与文档：① 它不启动 app（否则要回答"谁持有窗口、进程退出怎么办、stderr 去哪"）；② 工具调用失败是 `isError: true` 的**结果**而不是 JSON-RPC error（模型看得见、能改）；③ `floom_screenshot` 只报尺寸与哈希——协议报的是"这一帧变了"，不是"长什么样"，把它说成截图会是假的。

`crates/app/floom-cli` 刻意保持零依赖（只为 `floom create` 能在裸机器上跑起来），所以 MCP 是独立 crate。实测：启动示例 app → `floom-mcp` 收 `initialize` 与两次 `tools/call` → 读回 `button "Increment" 160x40`、点它得 `Done{repainted:true}`。

**A2 的一半：`Tree` 给的 id 现在能用了。** 协议里 `Selector` 原有 `Label`/`Role` 两种，而 `Tree` 每条节点都带 `id` —— 读得到、点不到，等于给 agent 一个假手柄。现在 `Selector::Id(u64)` 会解析成本次的节点并走同一条点击路径（`AgentApp::element_of_id` + `tap_element`，默认实现基于 `host.node_at` / `host.node_id` / `host.activate`），越界或已离屏的 id 报 `SelectorNotFound` 并附上"现在屏幕上有什么"。

id 是什么、不是什么，写进了 `protocol.rs`：它是**挂载次序**，节点在屏期间稳定，sibling 在它前面挂载会让它移位；跨 i18n 文案变更仍要靠 `Label`。要真正稳定的身份，得给节点一个作者声明的 key —— 那是 A3 的后半，没做。

证据（`crates/system/floom-accuracy/tests/agent_ids.rs`，3 passed）：读树拿到 `Increment` 的 id 后 `Tap{Id}` 回 `Done{repainted:true}`；`Id(9999)` 被拒且回执里列出屏上的 label；同一个 app 建两次，`Tree` 与 `Semantics` 的 JSON 逐字节相同（这台机器上 id、次序、几何都是确定的）。

**A1：跑起来的 app 有了 agent 口。** 三块拼起来，都在 `floom-accuracy::agent`：

- `AgentApp` trait（`session.rs`）：`host` / `size` / `scene_nodes` / `labels` / `has` / `tap` / `type_text` / `drag`。`TestApp` 与 `WindowApp` 各实现一次，会话代码只有一份 —— 所以 agent 的点击与测试的点击走同一条事件路径，不是两条。
- `WindowApp`（`window.rs`）：对真实 `WidgetHost` 实现它；`tap` 落成 `host.activate(element)`（元素中心按下+抬起），`has` 按 label/role 在语义树里找。`TypeText` 与 `Drag` **明确返回 false 而不是假装成功**：前者要输入法会话、后者要带时间的手势路径，凭空造一个只会教 agent 一个 app 收不到的手势。
- `spawn_bridge`（`transport.rs`）：TCP 连接线程把请求塞进 channel，窗口线程每帧（以及 `about_to_wait`，因为静止的树不再给帧）取出并回答。请求是"交给窗口线程的值"，不是跨线程调用 —— `WidgetHost` 是 `&mut` 且只属于事件循环。

证据（`scripts/desktop-smoke.sh`，Linux Xvfb 与 Windows 各一次）：

```
floom-software: agent listening on 127.0.0.1:4877
agent: tree has button Increment at 0,0 160x40
agent: tap Increment repainted=true
```

仍是 loopback-only（`bind` 拒绝非回环地址、`is_loopback_peer` 丢非回环对端）；示例不设 token 是因为它只在 CI 的本地回环上活两秒，`AgentSession::with_token` 给真实开发服务器留着。

## 判据（写进 1.0 门槛的东西）

1. `floom mcp` 一条命令起服务；`tools/list` 至少含 `ui_tree`、`ui_semantics`、`ui_schema`、`ui_screenshot`、`ui_tap`、`ui_type`、`perf_summary`。
2. 任何一次 `ui_tap` 的返回里带 `expected/actual` 的可比数字（节点 id、矩形、hash），而不是"ok"。
3. 视觉断言失败时自动落盘三件套到 `target/floom-artifacts/<test>/`。
4. `llms.txt` 里的每条链接都指向存在的文件（现在有一条指向不存在的 `npm/@types/floom/index.d.ts`）。
5. 只绑定回环 + token，**永不**监听 `0.0.0.0`。

## 明确不做

- 不做"自然语言直接生成界面"的托管服务。schema 和协议是给 agent 用的，不是给模板引擎用的。
- 不做云端回放/遥测。工件留在本地 `target/`。
- 不让 agent 通过猜 `NodeId` 操作界面（同 `Selector` 的既有决定）。
