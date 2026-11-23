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
| A2 | **无 `floom mcp`**：没有任何 MCP server 把上面 6 个只读命令 + 3 个动作暴露出去 | 每个 harness 都要自己写胶水 | 全仓无 `mcp` |
| A3 | **语义动作 ID 不稳定**：`Tap{Label}` 在文案变更即失效，没有"稳定 id + label 兜底" | 一次 i18n 改动就让 agent 的全部脚本报废 | `Selector` 只有两种（**2026-11-23 起有 `Selector::Id`，但 id 是挂载次序，sibling 挂载会让它移位**） |
| A4 | **失败工件不成套**：`assert` 失败只报哈希不符，没有"期望/实际 PNG + 场景 diff + 那一刻的树"三件套 | agent 拿到哈希没法自己修 | `floom-snapshot/src/lib.rs:72-88` |
| A5 | **token 分层文档**：schema 有 token 关系表，但没有"这个组件允许改哪些 token、改了会不会破一致性"的成文约束 | agent 会随手硬编码颜色（组件里已有 26/29/27 处先例） | [components.md](components.md) |
| A6 | **CLI 没有 agent 子命令**：`floom verify` / `floom snapshot` 未暴露 | agent 只能写 Rust 测试来验 | `floom-cli/src/main.rs:103-109` 无 |

## 已闭的缺口

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
