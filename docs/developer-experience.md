<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# 开发友好

判据只有一条：**改一行，两秒内看见结果；出错时知道下一步点哪里。**

## 单 CLI 全生命周期

`floom` CLI 在 `crates/app/floom-cli`。对标 nsh 的 `nsh test --trace`、tauri 的 `tauri dev`、makepad 的 `cargo-makepad`。
子命令在 `crates/app/floom-cli/src/main.rs`：`run` / `create` / `init` / `check` / `test` / `package` / `doctor`。

| 子命令 | 作用 | 状态 |
| --- | --- | --- |
| `floom create` / `init` | 从模板建工程（`--template app`或`plugin`、`--style rust`或`tsx`、`--platforms`） | 有，模板真实可用 |
| `floom run` | 构建 + 启动 | 有 |
| `floom run --watch` | 监听文件，改动后重建并重启 | 有（**文件级热重启**，非状态保留） |
| `floom check` | 先看目录里有没有工程，然后按 CI 的便宜门禁顺序跑：`cargo fmt --all --check` → `cargo check --all-targets`（`--features` 只加在后者） | 有（2026-11-25）；不再是 `cargo check` 的改名——绿了意味着 CI 的两个静态 job 也会绿 |
| `floom test` | 转发 `cargo test` | 有，但**只是转发**：不输出 ops/mount/nodes 级 trace |
| `floom verify` | `cargo fmt --all --check` → `cargo check --all-targets` → `cargo test --no-fail-fast`，一条命令一个退出码 | 有（2026-11-24）；便宜的先跑，测试最后 |
| `floom snapshot` | 只跑快照测试（`cargo test -- snapshot`）；`--update` 置 `UPDATE_SNAPSHOTS=1` 重写 golden | 有（2026-11-24） |
| `floom package` | 按平台打包（`--platform <target>`，`pack::pack`，自包含，不依赖仓内 `scripts/build.sh`） | 有，真实实现 |
| `floom doctor` | 报 rustc/cargo/host、每个平台所需 target 是否安装、工程钉住的 floom 版本；`--platform X` 让该平台缺 target 变成失败 | 有（2026-11-21）；**不查** GPU、字体、SDK、设备信任 |

所以 G5 的真实缺口不是"命令没有"：`doctor` 已经有了（只回答"这台机器能不能构建这个平台"，判据是 target 是否安装），`check` 会在 cargo 之前先把"这个目录里没有工程"说成人话。`check` 与 `test` 现在各自带着一条真实语义（静态门禁顺序 / 不允许顺手 bless golden），但仍**没有**把 `docs_parity`、`gate` 的其余子集（size、perf、style、module-paths）与 trace 串进来——那是"开发便利 3 分 → 5 分"剩下的距离。

## 热重载

- 现状：`floom run --watch` 是**文件级热重启**——进程重启、状态丢失。
- 为什么 TSX 路径上"原地重建"不值得做（2026-11-26 判定）：`floom run` 的 `.tsx` 路径会生成一个临时 crate 再编译，改一次源码本来就要重新编译；编译完再启动进程与"编译完在原进程里重建"耗时相同，而后者只能保住用户态的 widget 状态——一个刚被重新编译的二进制没有旧状态可保。真正需要原地重建的是**应用自己持有重建闭包**的场景（Rust 应用），所以入口做成 `floom::run_desktop_software_watched`，由应用决定"什么算源码变了"，CLI 不替它猜。
- `floom run <dir> --watch`（2026-11-26）：目录里有 `Cargo.toml` 时，CLI 用 `cargo run … -- --watch` 启动并把 `--watch` 交给应用，**重建由应用自己做**（`floom::run_desktop_software_watched`），CLI 不杀进程。
- 机制那一半是**做完了的**（2026-11-25）：`floom_hot_reload::watch::Watch` 轮询 mtime（忽略 `target/`、首次静默、删除也上报）、`WidgetHost::remount` 重建并按 authored key 路径搬状态、`Widget::carry_state` 由 `Text`/`Switch`/`TextInput`/`Scroll` 实现。接线也有了：`floom::run_desktop_software_watched(title, size, build, watch)`——`build` 是 `Fn`（不是 `FnOnce`），循环空闲时问一次 `watch()`，为真就 `remount`。`floom run --watch` 本身仍是进程重启（CLI 侧还没切）。
- 更早的事实，留着免得有人重查：`crates/devtools/floom-hot-reload/src/patch.rs:88` 的 `LiveTree::apply(&UiPatch, &mut DirtySet)` 事务式应用补丁（任一 op 失败则 `restore(checkpoint)` 并返回 `RolledBack { failed_at, reason }`），keys 与 `stash` 跨补丁保留，`tests/hot_reload.rs::ids_and_stashed_state_survive_a_compatible_patch` 证明。
- 卡住的不是补丁，是**两棵树**：`LiveTree` 是 `WidgetHost` 的只读镜像（`describe(&[HostedNode]) -> LiveTree`），补丁改的是镜像，像素由 `WidgetHost` 决定；把镜像的改动送回去，要么让 host 由文档构建（`floom-dsl` 的 `UiDocument` 直接驱动挂载），要么给 host 加一条按 key 重挂的入口。这是设计决策，不是接线——所以它停在未开始，而不是差一个 watcher。
- **决定（2026-11-25）：host 仍是真相，补丁路径是"按 key 重挂"**，不是"文档驱动挂载"。理由三条：① 这个库的公开 API 是 Rust 值（`Button::new("Save")`、`mount_root(style, widget)`），用户写的是代码不是 IR；② `Selector::Key`（agent 侧）已经建立在 `Child::named` + host 的 `agent_key` 之上，改文档驱动要迁移 agent 与 devtools 两处 key 语义；③ 文档驱动需要 `UiDocument` 覆盖每个 widget 的每个 prop，那是把 Rust 的类型检查换成运行时校验，与"开发友好"相反。
- 因此第一步是给 `WidgetHost` 一个按 key 重挂的入口（旧树按 `ChildKey`/`agent_key` 与新树对齐，能带的状态从旧 widget 搬到新 widget，带不了的如实丢弃），watcher 与补丁应用都排在它后面。
- 明确不做（1.0.x 之前）：进程内保留状态的热补丁。
- 延迟纪律：改一次文件到窗口重绘的时间必须**记录下来**，进 release notes；不测就不许说"快"。
- 已知限制：重启期间窗口会闪一下；watch 只覆盖 `src/`。

## Devtools

`floom-devtools` 的检查器是 Chrome 布局的：树 + 标签栏 + picker（鼠标点选元素、高亮盒子、看到自己那一行 style）。

- 已有：树、picker、逐帧 `budget_exceeded`、分配计数、`PerformanceSnapshot`、语义树视图。
- 缺口 G7：Style 行只显示**声明值**，没有计算值；缺 hover 高亮。
- 缺口（接真实窗口）：性能画像停在 headless 协议，`app wiring pending`。
- 快捷键接线在事件循环里，不在树里——这是有意的（面板不能吃掉应用的按键）。

## 模板与示例

- `floom new` 模板走 `crates/app/floom-cli`，模板仓库里带最小脚手架 + 一个按钮 + 一个 `cargo test`。
- golden examples 见 [examples.md](examples.md)（8 个，其清单被 `docs_parity.rs` 与 `scripts/run-golden-examples.sh` 双向校验）。
- `floom-showcase` 是 `/components` 页挂载全部 Tier-0 组件的**唯一**"设计回归"入口。

## 日志（Rust → logcat / os_log）

Android 上 `log` 门面的记录默认**一条都不出现**：`android_logger` 的级别跟着 `log::max_level()`，而它初始是 `Off`。开关是系统属性，不需要重新编译：

```sh
adb shell setprop debug.floom.log debug     # off | 1(debug 档以上见下) | info | debug | trace | warn | error
adb shell am force-stop <pkg> && adb shell am start -n <pkg>/.<Class>Activity
adb logcat -s floom
```

值为空、`0`、`off` 时什么都不装（每条记录要一次 JNI 调用，没人要就不付这个成本）；无法识别的值也当关闭——拼错不会猜档位。开关必须在进程启动时读：属性是启动时快照，改完要重启 app。

证据（模拟器 API 34）：`debug.floom.log debug` → logcat 出现 `I floom : floom_platform_android::logging: logging to logcat at DEBUG; debug.floom.log set the level` 与 `I floom : ...: android backend starting`（还有 `android_activity` 自己的 `D floom :` 行——`set_max_level` 是全局的）；设回 `0` 重启 → `adb logcat -s floom` 零行。

平台另一半：**iOS 上 `log` 门面没有后端**，`os_log`/`NSLog` 只吃 Swift 侧的输出；Rust 的 `println!`/`eprintln!` 在两端都不进系统日志（Android 的 stdout 会以 `RustStdoutStderr` 标签出现，但不保证被 flush）。桌面走 `RUST_LOG`，那是 `tracing` 生态的约定，与这里无关。

## 错误信息

要求（当前部分满足）：

1. 编译期：feature 组合错误给出"启用哪个 feature"的建议（`feature_graph.rs` 的报错已做到）。
2. 运行期：能力缺失报"这个平台支持什么"，不是 `Option::None`。
3. 平台侧：`PlatformCapabilities` 声明与实现不符是**故意可见的 bug**，不允许用 `unwrap_or_default()` 掩盖（见 [platforms.md](platforms.md)）。
4. 测试失败：`just gate` 的 skip 必须打印原因，`SKIP ≠ PASS`。

## TSX / DSL / 编辑器

- `floom-tsx`：TSX→Rust 前端。**注意**：`llms.txt` 曾声称发布了 `npm/@types/floom/index.d.ts`，仓库里没有 `npm/` 目录 —— 要么补，要么删，不允许文档先说。
- `floom-dsl`：`codegen` + `ui_ir` 两个模块；IR 是 agent 与热重载的共同底座。
- `floom-editor`：卫星 crate，富文本/代码编辑，不在 1.0 锁定面。

## CI 到底证明了什么

绿不等于验证过。下面是 2026-11-21 的实测账本：每条"CI 声称的"后面跟的是它**实际**执行的命令，以及命令覆盖不到的东西。

| CI 里看到的名字 | 实际命令 | 覆盖不到 |
| --- | --- | --- |
| `Floom 2.0 on {macOS,ubuntu-24.04,windows}` | `cargo test $FLOOM2_PACKAGES` —— 门面 `floom` 的 `default = []`，所以**没有任何后端被编译** | 门面 8 个 bundle（`desktop`/`tui`/`web`/`devtools`/`testkit`/`snapshot`/`renderer-gpu`/`full`）的运行时接线。2026-11-21 起补了 macOS 的 `--features desktop` 一步与 ubuntu 的 `bundles` job |
| `build` | `cargo build` + `cargo nextest run --workspace` | 同上（default features）；`crates/ecosystem/*` 是独立 workspace，不在此列 |
| MSRV | `cargo check --workspace --all-features`（ubuntu，1.91） | 只**编译**不运行。全 cfg 的编译覆盖靠它，运行时覆盖靠不住它 |
| `CI platform`（android/iOS/OHOS） | `cargo check --target ...` / `cargo build`+`link`（OHOS） | 交叉分支从不**执行**：android/ios 的错误处理、JNI 调用、a11y 适配逻辑一个断言都没跑 |
| `CI plugins` | 该 workspace 的 `cargo test` + `fmt` | 插件的原生半只在"能编译能链接"这一层被证明 |
| 像素/渲染 | 2026-11-21 前：**无**。现在两套后端：`gpu-pixels` 在 `macos-latest` 的 Metal 上、`GPU pixels on lavapipe` 在 Mesa 软件 Vulkan 上，各跑同一批 11 个像素测试 | 组件 golden：3 个含文本项改用随库分发的 Ahem（`floom-snapshot/tests/fixtures/ahem.ttf`）取景，已在三系统真跑；大小/拖影类像素仍在 macOS 才验（ubuntu/windows runner 无适配器） |
| `just gate size` / `perf` 的等价物 | **无** | 体积与帧预算只在本地跑 |
| 体积/性能/资源（G11/G12/G13） | **无** | `just gate size` / `scripts/check-perf-budgets.sh` 只在本地跑 |

### 变异测试：测试是真的会红，还是只是跑过

只读命令列表说明不了问题，所以往代码里注入了三个真实缺陷，跑一遍 CI 等价命令：

| 注入的缺陷 | 捕获情况 |
| --- | --- |
| `crates/widgets/floom-text/src/layout.rs`：丢掉 pen 累加（所有 glyph 叠在原点） | **抓到**，`tests/shaping.rs::glyph_positions_advance_across_the_run` 红 |
| `crates/app/floom/src/platform.rs`：无后端时 `run()` 返回 `Ok(())` 而不是解释缺什么 | **抓到**，`platform::tests::a_run_without_a_backend_explains_what_to_enable` 红 |
| `crates/platform/floom-platform-harmony/src/stride_bytes`：永远返回行宽（像素/字节读法失效） | 单测**存在且会红**，但 CI 的包清单里**没有**这个 crate（`FLOOM2_PACKAGES` 只列了 desktop/android/tui/web）→ CI 里永远跑不到。2026-11-21 起已把 harmony / ios / embedded 三个 crate 加进清单 |

结论：单元层是真的（几千个测试，注入的缺陷会红），**薄在运行时的平台接缝**——交叉编译分支、后端 bundle 的级联、渲染像素、以及自己承诺的预算门槛。

### 为什么这些洞会一直存在

- 交叉目标只能 `check`：一台 ubuntu runner 跑不了 Android 的 JNI、iOS 的 UIKit、OHOS 的 NAPI。**这不可修**，只能靠 `gates/*.md` 的设备账本诚实标注 device-gated。
- GPU 像素测试需要 GPU：`macos-latest` 有 Metal，实测 11 个测试 25 s 跑完（2026-11-21 起在 CI 里），ubuntu/windows runner 没有适配器。
- 门槛脚本（size/perf）依赖具体宿主，异构 runner 上会假红——`1.0-launch-milestone.md` 里有明确记录，所以它们留在本地 `just gate`。

## 目标数字（要写进 release notes 的）

| 项 | 目标 |
| --- | --- |
| 文件改动 → 窗口重绘 | ≤ 2 s（`--watch`，中位） |
| 冷启动首帧 | 记录并逐版对比（当前未测） |
| `floom check` 全绿时间 | ≤ 60 s（中等工程） |
| 从零到跑起来的命令数 | 1（`floom new x && cd x && floom run`） |
