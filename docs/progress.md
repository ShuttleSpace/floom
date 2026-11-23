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
| S0.4 | **组织与约定对齐 tessera**：`mod.rs` → `name.rs`、`AGENTS.md`、依赖策略注释 | 仓库里不再出现 `mod.rs`；`AGENTS.md` 存在且指向 `docs/`；门禁能抓住回归 | 29 个 `mod.rs` 全量改名（`cargo check --workspace --all-targets` 四个 workspace 全清）；`scripts/check-module-paths.sh` + `just gate module-paths` + CI 一步，注入 `mod.rs` 实测会红；根 `AGENTS.md`（70 行）；`Cargo.toml` 补依赖策略注释 |
| S0.3 | **CI 有效性审计**：绿是真的验证过还是空转 | 每个 job 的「声称 vs 实际命令」有账本；注入缺陷能被抓到，或明确记录抓不到 | 变异测试 2 抓 1 漏（漏的是 CI 包清单缺 harmony/ios/embedded）；已补 3 个 crate 进清单 + macOS `--features desktop` 一步 + ubuntu `bundles` job；账本在 `developer-experience.md` 的「CI 到底证明了什么」 |
| S0.2 | 把 `deny`/`perf`/`size` 接进同一套门禁口径（`gate.sh --list` 与实际 CI 步骤一致） | `gate.sh --list` 与 CI 步骤集合相同，缺一即红 | `bash scripts/gate.sh --list`，逐条比对 workflow |

### S1 平台 binding 从"声明"到"实测"

| ID | 动作 | 退出判据 | 验证命令 |
| --- | --- | --- | --- |
| S1.1 | **OHOS 后端选型**：复活 `floom-winit-ohos`（有事件循环/输入/IME）或补齐 `floom-platform-harmony`（补输入导出） | `platforms.md` §4 写下一个决定 + 一条能收 touch 的路径；另一半代码删除或明确标注 deprecated | `grep -rn "floom-winit-ohos" crates/` 只剩一处实现路径 |
| S1.2 | OHOS 上屏 5 个缺陷：早退不 `AbortBuffer`、失败仍 `Flush`、三 atomic 非成组、尺寸精确相等、`stride` 单位 | 五个缺陷各自有注释指明修法；`stride` 单位有上机结论 | 上机：连续 60 s 无丢帧；日志无 `AbortBuffer missing` |
| S1.3 | iOS CJK：字体扫描/注册——**根因是 fontique 的 Apple 后端不扫 `/System/Library/Fonts`**，不是 iOS 缺汉字字体；现已由 `crates/widgets/floom-text/src/system_fonts.rs` 扫该系统目录按文件名前缀注册 CJK 面 | `你好世界` 首帧非 tofu | `idb screenshot` 与基线截图比对；宿主侧已有 `han_text_shapes_to_real_glyphs` |
| S1.4 | iOS a11y：把布局帧传给 `UIAccessibilityElement` | `idb ui describe-all` 的 frame 非零 | `idb ui describe-all`，与 `semantics` 树的 bounds 对齐 |
| S1.11 | **safe-area**：宿主把 `safeAreaInsets` 变成内容内边距（并顺手修"布局移动不刷新语义 bounds"与"事件处理器拿不到 host"） | 模拟器上内容避开状态栏/home indicator；示例 app 点击后可见变化 | `cargo test -p floom-app --test insets --test frame_callbacks`；`idb ui describe-all` 的 y 与屏幕像素一致 |
| S1.12 | **Android 宿主**：模板改用 `NativeActivity` + `android_main`（原模板声明了从未导出的 JNI 符号，装机即崩） | 生成的 app 在模拟器上出像素，`input tap` 能改计数 | `floom create` → `floom package --platform android` → `adb install` → `am start` + 截图 |
| S1.5 | Android：a11y `perform_action` 真实现 + 聚焦元素可见（焦点环） | `gates/android.md` item3/item7 由 `fail`/`partial` 转 `pass` | 设备上 TalkBack 能点击；截图见焦点环 |
| S1.6 | 桌面：剪贴板非 macOS 实现 + AT-SPI/UIA 桥 + `file-picker` trait | `gates/linux.md`/`windows.md` 首项转 `pass` | 三平台 `cargo test -p floom-platform-desktop` + 手工复制粘贴 |
| S1.7 | 平台状态表改真：把达标的 target 从 `device-gated` 改 `tested` | ≥3 target `tested` 含 ≥1 移动端（G1 达成） | `grep -c "tested" docs/release-policy.md`；`gates/*.md` 无 `fail` |
| S1.9 | **组件 golden 与宿主字体解耦（完成）**：3 个含文本 golden 改为在随库分发的 Ahem（`crates/devtools/floom-snapshot/tests/fixtures/ahem.ttf`，公有领域，每字形 1em 方块）下取景，`floom_text::register_font` + `WidgetHost::register_font` 把 `SansSerif` 指向它；golden 里的数字现在是 `168.0`（12 字 × 14px）这类可推算值，不再是 `78.5791` | 数字可由字体推算、与宿主无关 | 三系统 CI 上 `cargo test -p floom-snapshot --test widgets` 九项全 `ok` 且无 `ignored`；本机同哈希 |
| S1.8 | **Windows 文本半像素换行**（CI 首次真跑发现）：`a_label_that_fits_on_one_line_paints_on_one_line` 在 Windows 上实测 1.1499 行——同一字符串在 Segoe UI 下量得够宽，量/画对不齐的旧缺陷在 Windows 上仍在 | 该测试去掉 `cfg_attr(ignore)` 后在 Windows runner 上绿；文本测量改用 fixture 字体而非系统字体 | `gh run view <id> --job <windows 2.0 job>` 里该测试为 `ok` |

### S2 契约与门禁（锁住已有优势）

| ID | 动作 | 退出判据 | 验证命令 |
| --- | --- | --- | --- |
| S2.1 | 插件 `abi_version`（核实为**早已成立**，本步降级为核验台账） | 每条台账前提都被命令验证过；假缺口删除 | `crates/native/floom-native-ffi/tests/c_vtable.rs` 的旧 ABI 负例 |
| S2.2 | 资源门禁（RSS / GPU 时间；逐帧分配、峰值内存、首帧、输入延迟已由测试落地） | `just gate resources` 能绿能红 | 故意把阈值调小 → 红；恢复 → 绿 |
| S2.3 | **完成**：82 处字面量清零——36 处 `rgba(0,0,0,0)` → `Color::TRANSPARENT`（主题自己的 `Color` 类型补了该常量）、22 处 `rgba(0,0,0,255)` → `Color::BLACK`（scrim/shadow 的 alpha 本来就叠在它上面）、14 处状态色（info/success/warning、cupertino 的 rating 黄、三个 color-picker 默认色）移进各 crate 的 `src/tokens.rs`；顺带统一 fluent banner 的 Warning（原 `#F7CE00`，与 alert 的 `#FFB900` 不一致） | 三个 `*/src/widgets/` 0 处 `Color::rgb*`；`gate.sh style` 绿，注入一个字面量即红 | `bash scripts/gate.sh style`；变异验证：往 `floom-material/src/widgets/badge.rs` 加一行 `Color::rgb(1,2,3)` → 退出码 1 |
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
| S0.1 | CI 真的会跑 | 完成 | 23 个 job 全绿：https://github.com/ShuttleSpace/floom/actions/runs/44806694423（CI core，含 Rust on macOS/ubuntu/windows 三系统 `cargo test` + Floom 2.0 ×3 + clippy ×3 + MSRV + fmt + typos + deny ×2 + arch）、…/runs/44806694390（plugins）、…/runs/44806694366（platform: android/iOS 交叉检查 + device probe）、…/runs/44806694311（docs → Pages）。共修既有缺陷 21 类，详见 git log 2026-11-20 | 2026-11-20 |
| S0.2 | 门禁口径一致 | 未开始 | — | — |
| S0.3 | CI 有效性审计 | 完成 | 变异测试：text pen 累加与 no-backend 报错两个注入缺陷被抓到；harmony 的 `stride_bytes` 单测存在但 CI 包清单里没有该 crate。已补 `floom-platform-{harmony,ios,embedded}` 进 `FLOOM2_PACKAGES`、macOS 加 `cargo test -p floom --features desktop`、ubuntu 新增 `bundles` job（desktop/tui/full） | `gh run view <id>` 里 `Floom bundles on ubuntu-24.04` 与 macOS 那一步为 success；另新增 `gpu-pixels` job（macos-latest, Metal）跑 11 个像素测试，并用「不推裁剪层」的变异确认它会红（`a_clip_crops_the_ink_at_its_own_edge`）；账本见 `developer-experience.md`；另修两处依赖环境/时钟的测试：剪贴板往返按守护进程探测、`VelocityTracker` 加 `record_at`/`velocity_at` 去掉 `thread::sleep` |
| S1.1 | OHOS 后端选型（A：复活 `floom-winit-ohos`） | 待验证（设备） | 驱动 `crates/app/floom/src/platform/ohos_winit.rs`：Rust 拥有 surface/loop/tree/blit，导出 `ohos_winit_runtime_*`；`floom::ohos_app!(build)` 是应用唯一入口；`floom::run` 在这条路径上返回明确错误。CI 已 **check + build + link**（`#[link(name="native_window")]` 真链通过）。**未上机**：`tests/floom-ohos-demo` 的 C++ 桥仍走旧三函数 | 绿：https://github.com/ShuttleSpace/floom/actions/runs/44863051614 （job `Floom 2.0 for aarch64-unknown-linux-ohos`）；上机待 `S1.1b` |
| S1.1b | 把 `tests/floom-ohos-demo` 的 C++ 桥从旧三函数切到 `ohos_winit_runtime_*`（surface/touch/frame） | 设备上出现真实 floom 树（不是渐变），点击 +/- 改值 | 上机；`hilog` 无 `surface_created` 之外的告警 |
| S1.3 | iOS CJK 豆腐 | 待验证（设备） | `crates/widgets/floom-text/src/system_fonts.rs`：扫 `/System/Library/Fonts`(+`Supplemental`) 按前缀挑 CJK 面注册进 `FontContext`，`TextShaper::new()` 与编辑器线程 `FONT_CONTEXT` 各调一次。宿主实测 bare `FontContext` 查不到 `PingFang SC`（能查到 `Arial Unicode MS`，所以 macOS 一直没暴露） | `cargo test -p floom-text` 28 passed（含 3 个新测）；真机待 `gates/ios.md` item 2 |
| S1.2 | OHOS 上屏 5 缺陷 | 待验证（设备） | 五条全部改完并各有单元测试：AbortBuffer 归还、失败不 Flush、单一 `Mutex<Option<Bound>>` 跨 blit 持锁、帧自己的尺寸、`stride_bytes` 双读法。**未上机**：连续 60 s 无丢帧与 hilog 无 Abort 告警待真机 | `cargo test -p floom-platform-harmony`（6 passed）；上机待验 |
| S1.4 | iOS a11y frame + 动作回传 | 完成（UIKit 腿待真机手动验） | 读数：`SemanticsNode::bounds` + `Platform::accessibility` 门 + `deliver_semantics` → `floom-platform-ios/src/accessibility.rs` 展平成 `A11yNode`（阅读顺序、点坐标）→ Swift 造 `UIAccessibilityElement`。动作：`WidgetHost::activate(id)`（当作元素中心点击派发）← `UIAccessibilityElement` 子类 → `floom_ios_a11y_activate(index)` → 适配器把下标换回 `ElementId` → `ios_host` 注册的处理器。能力位含 `ACTION_INVOKE` | `idb ui describe-all` → `StaticText \"Count: 0\" {0,0,402,14}`、`Button \"Increment\" {0,22,78,22}`；`cargo test -p floom-app --test semantics` 3 passed、`-p floom-platform-ios` 11 passed；**UIKit 腿未机验**（`idb` 不能触发 VoiceOver 双击） |
| S1.10 | iOS 宿主运行时（原先不存在：`ios/` 是占位工程，`enum Floom` 四个方法体全空，没有 `.xcodeproj`） | 完成 | `floom::ios_app!(build)` 生成 `floom_ios_app_{start,pump,touch}`；`ios/src/{AppDelegate,ViewController}.swift` 换成真宿主（scene 生命周期 + `CGBitmapContext` + display link + 触摸转发）；`floom package --platform ios` 改成 staticlib + `swiftc` 链接 + 临时签名，不再依赖 `cargo-xcode`；`src/app.rs` 抽出共用 widget 树，`src/lib.rs` 是 iOS 入口 | 端到端跑过：`floom create --platforms ios` → `floom package --platform ios` → `simctl install/launch` → 模拟器上出现 floom 像素（日志 `floom: bound=true started=true size=1206x2622 scale=3.0`）；`cargo test -p floom-cli` 112 passed |
| S1.11 | safe-area 进布局（含 `on_frame` 与语义 bounds 刷新） | 完成 | `WidgetHost::set_content_insets(Insets)` 作为根内边距（每方向 `max`，不写回节点样式，背景仍整屏）；`ios_host` 在 `start`/每帧 `pump` 同步 UIKit 的 `safeAreaInsets`；`touch` 除以 `displayScale`；`WidgetHost::refresh_semantics()` 供宿主在 safe-area/尺寸变化时重建一次语义（不做逐帧移动检测：那会让每个移动节点产生一次分配，`--test allocation` 会红）；新增 `WidgetHost::on_frame` 让事件处理器能经帧回调改树 | 模拟器（iPhone 17 / iOS 27.0）：内容 `y0` 从 0 变 62（= UIKit 报的 `{62,0,34,0}`），截图深色文字带落在 62.7–73 pt；`idb ui tap` 后 `StaticText "Count: 1"`（此前模板的标签是死字符串）；`cargo test -p floom-app --test insets` 2 passed、`--test frame_callbacks` 1 passed、`-p floom-cli` 112 passed | 2026-11-22；绿（含 `Floom 2.0` 三系统与 clippy）：https://github.com/ShuttleSpace/floom/commit/60ca5523 |
| S1.12 | Android 宿主运行时 | 完成（状态栏遮挡待解） | 三处根因一起修：①模板 Kotlin 声明了 `FloomNative.start/frame/touch/…` 一批 `external fun`，而 Rust 侧从未导出（首调 `UnsatisfiedLinkError`）；②`floom-platform-android::run` 要 winit 的 `AndroidApp`，它只出现在 `android_main` 里，而模板没有 `android_main`；③manifest 写的是自定义 meta-data `floom.nativeLibraryDir`，`NativeActivity` 只认 `android.app.lib_name`，且该 meta-data 必须在 `<activity>` 内（放 `<application>` 里会报 `Unable to find native library main`）。改法：`floom::android_app!` 宏（生成 `android_main` → `set_android_app` → `run`，与 `ios_app!`/`ohos_app!` 同形）+ `floom::AndroidApp` 再导出（免得每个 app 去钉 winit fork 的 rev）+ template `[lib] crate-type=["cdylib"]` 与 `[lib] name`/manifest `lib_name` 用同一个值 + Kotlin 只留 `NativeActivity` 子类 + scaffold 写 `android.newDsl=false`（AGP 9 的新 DSL 与 KGP 不兼容）+ `floom package` 在缺 `gradlew` 时按 `gradle-wrapper.properties` 引导 wrapper | 模拟器 floem2api34（arm64-v8a，API 34）：`Count: 0` + 蓝色 `Increment` 首次在 Android 上由 Floom 画出（此前只有闪屏图标）；触摸已证到达 Rust（`PointerButton Pressed/Released`，逻辑点 = 像素 ÷ 2.625）；**遗留**：模板内容从 y=0 开始，按钮落在状态栏带内，`input tap` 点它会被系统吞掉。safe-area：`packages::safe_area::safe_area_insets()`（读 activity 的 `WindowInsets`，系统栏与刘海取并集，设备像素 ÷ density）此前**没有调用方**，现在由 Android 后端在窗口尺寸变化时读一次并交给 `WidgetHost::set_content_insets` | 模拟器（floem2api34）：内容从状态栏下开始（`Count: 0` 的 y 从 0 变为状态栏高度）；`input tap` 在按钮上 → `Count: 1` | 2026-11-22；绿：https://github.com/ShuttleSpace/floom/commit/653fbf0a |
| S1.5 | Android a11y + 焦点环 | 完成 | 证据：`docs/gates/android.md` item3/item7 | A：`floom-platform-android/src/accessibility.rs` 重写为真适配器——按 `bounds` 的 y 再 x 排序发布（阅读顺序，map 顺序是任意的）、跳过 `role == None` 与 `HIDDEN`、`NATIVE_BRIDGE` 只在有 publisher 时置位、`perform_action` 只认 `Activate` 且无 publisher 时一律 false；`AndroidPlatform::accessibility()` 已把它交给 `AppRunner::deliver_semantics`（此前返回 `None`，树根本到不了平台）。4 个单测：`cargo test -p floom-platform-android` 9 passed。B：**已完成**。Rust 侧把树编码成"每节点一行、制表符分隔"（`accessibility::encode`，纯函数有单测），存在 `static PUBLISHED: Mutex<String>`；两个手写 mangled 的 JNI 入口 `Java_dev_floom_android_FloomA11y_pull`（返回 jstring；首参必须是 `jni::EnvUnowned`——`jni::Env` 不是 FFI safe，按值收会解引用野指针并 SIGABRT）与 `_activate`（← `perform_action(ACTION_CLICK)`）。JVM 半边由脚手架生成 `dev/floom/android/FloomA11y.java`：静态 `System.loadLibrary`、`attach(View)` 把 `AccessibilityNodeProvider` 装在 `window.decorView` 的 delegate 上（不叠一层 View，否则会吃掉 native surface 的触摸），把 payload 解析成虚拟 `AccessibilityNodeInfo`（class 名按 role 映射、`setClickable(true)` + `ACTION_CLICK`、bounds = 逻辑像素 × density + `getLocationOnScreen`）。证据（模拟器 floem2api34 + API 34）：`uiautomator dump` 出现 `text="Count: 0" class="android.widget.TextView" bounds="[0,137][945,179]"` 与 `text="Increment" class="android.widget.Button" clickable="true" bounds="[0,200][207,266]"`；开启 TalkBack 后点击 → `Count: 0` 变 `Count: 3`。C：**已完成**。焦点环由宿主绘制（`WidgetHost::paint_focus_ring` + `FocusRing::DEFAULT`，四条填充矩形——每个后端都会画矩形，只有部分支持描边；一次铸 id 长期复用，静止帧零分配零损伤；`set_focus_ring(None)` 供设计系统自绘）。真机上键 `KeyCode(3)` 到达后环随遍历移动：第一次 Tab 文字带 8931 个环色像素、按钮带 0；第二次 Tab 文字带 0、按钮带 1106。`set_focus`/`move_focus` 现在会点亮脏标记（环是宿主画的，焦点移动即使 widget 无反应也要一帧）。另：Rust → logcat 的日志开关已加（`debug.floom.log` = `off|1|info|debug|trace|warn|error`，`crates/platform/floom-platform-android/src/logging.rs`；未设置时一条不装，因为每条记录是一次 JNI 调用）。证据：`debug.floom.log debug` → `I floom : ... android backend starting`；设回 `0` 重启 → `adb logcat -s floom` 零行。 |
| S1.6 | 桌面剪贴板 / a11y / file-picker | 进行中 | `crates/platform/floom-platform-desktop/src/{clipboard,accessibility}.rs` | 剪贴板三平台齐了：macOS `NSPasteboard`、Windows `CF_UNICODETEXT`（CI runner 上真往返，含 CJK）、Linux `wl-copy`/`xclip`/`xsel`（解析有单测，会话里未验证——ubuntu runner 无 display，日志打印 `no clipboard in this session` 后跳过）；a11y 半桥已就位（2026-11-23）：`DesktopAccessibilityAdapter::tree_update()` 把保留帧转成 `accesskit::TreeUpdate`（角色映射、label/value/hint、bounds、`Click`/`SetValue` 动作、children 过滤到已发布的节点、焦点缺省落根、空帧返回 `None`），6 个新测试；macOS 桥已装（2026-11-23）：`crates/platform/floom-platform-desktop/src/accessibility/macos.rs` 在 `create_window_now` 里把 `accesskit_macos::Adapter` 挂到 winit 的 `NSView`，`poll_events` 里推 `TreeUpdate`（按 `version()` 去重）并把 AppKit 的 `Click` 转成 `SemanticsAction::Activate` 送回适配器；`Capability::ACCESSIBILITY` 随桥出现而不是随配置。运行时冒烟：`examples/floom-macos --features real` 带桥跑起来、照常聚焦与剪贴板。**未验：辅助技术实际读到什么**（没有 Accessibility Inspector / VoiceOver 的自动断言）。`accesskit_windows`（HWND）与 `accesskit_unix`（AT-SPI 会话）未装。`file-picker` trait 已核实为不该做（`floom-plugin-file-dialog` 已是完整实现） |
| S1.7 | ≥3 target tested（G1） | 进行中（2/3，含移动端） | `docs/release-policy.md` targets 表；`.github/workflows/ci-platform.yml` 的 `iOS app on a simulator` job | `aarch64-apple-ios-sim` 已 **tested (CI)**：`scripts/ios-smoke.sh` 走 `floom create` → `floom package --platform ios` → `simctl boot/install/launch` → 截图 → 数像素，断言蓝色按钮真的画出来了（本机实测 `1320x2868: 23867 inked pixels, 13853 blue`）；顺带修掉一个真 bug：`pack/ios.rs` 用包名 `-lsmoke` 去链接，而脚手架生成的 staticlib 叫 `libfloom_smoke.a`（`[lib] name = floom_<package>`，为 Android 的 `android.app.lib_name` 而改）——iOS 链接因此在我们改完 lib 名之后一直失败，直到这个脚本跑起来才暴露。`aarch64-apple-darwin` 也已 **tested (CI)**：新 job `macOS app in a window` 用 `scripts/macos-smoke.sh` 让闸探针开出真窗口，断言三行（`window surface 480x320 at scale 2`、剪贴板 `matched=true`、`focus None -> TextInput@2`）；为此在探针的 `window_created` 里加了一行 surface 报告。仍缺 1 个：Windows 或 Linux（runner 没有显示会话，需要 Xvfb/llvmpipe，或把措辞改成"跑了测试但没有窗口"）|
| S1.8 | Windows 文本半像素换行（新发现） | 未开始 | `crates/app/floom-app/tests/text_weight.rs` 的 `cfg_attr(ignore)` 与理由 | 2026-11-20 |
| S1.9 | Windows/Linux 组件 golden | 完成 | `crates/devtools/floom-snapshot/tests/widgets.rs`（Ahem 注册 + 重生成 golden）；证据 = 三系统 CI 无 `ignored` | 2026-11-21 |
| S2.1 | 插件 `abi_version` | 完成（核验） | `crates/native/floom-native-ffi/src/abi.rs:46` ABI_VERSION=1、`provider.rs:196` 注册校验、`tests/c_vtable.rs:317` 旧版本被拒 | 2026-11-21 核实，原台账是 grep 误报 |
| S2.2 | 资源门禁（G12） | 进行中 | P6 峰值 RSS、P8 逐帧分配、P10 首帧、P11 输入延迟均已有门禁（`tests/memory.rs`、`tests/allocation.rs`、`tests/latency.rs`）；只剩 GPU/VRAM 无 | 顺带修掉 S1.3 引入的 236 MiB 字体拷贝 |
| S2.3 | 组件 token 清零 + 门禁 | 完成 | 82 处清零（36 透明 / 22 黑 / 14 状态色进 tokens.rs）；`scripts/check-design-tokens.sh` + `gate.sh style`；fluent banner Warning 与 alert 统一为 `#FFB900` | 2026-11-21 |
| S2.4 | 组件契约门禁 | 未开始 | `component_schema()` 无消费者 | — |
| S3.1 | CLI check/test/doctor | 进行中 | `doctor` 已落地（`crates/app/floom-cli/src/doctor.rs`，`cargo test -p floom-cli` 110 passed）；`check` 加了前置校验（无 `Cargo.toml` 时给出下一步，单测 `check_explains_a_directory_that_holds_no_project`）；`test` 仍是裸转发 | `bash scripts/gate.sh` |
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
