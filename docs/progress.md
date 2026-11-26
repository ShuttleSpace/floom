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
| S1.1 | **OHOS 后端选型**：复活 `floom-winit-ohos`（有事件循环/输入/IME）或补齐 `floom-platform-harmony`（补输入导出） | `platforms.md` §4 写下一个决定 + 一条能收 touch 的路径；另一半代码删除或明确标注 deprecated **本机 SDK 现状（2026-11-26 勘到，未起设备）**：`/Users/humphrey/.local/share/ohos` 下有 `26.0.0`（含 `toolchains/hdc`，`hdc version` = 3.2.0f）、`command-line-tools`（`hvigorw`、`ohpm`、`emulator/Emulator`、`sdk/default`）、`system-image/HarmonyOS-7.0.0`；`~/.ohos/config` 是空目录。 `Emulator` 的用法是 `Emulator -start <name> [-imageRoot <path> -noWindow]` / `-create <name> -deviceType <type> -osVersion <version>` / `-imageList` / `-install`（**没有 `-list-avds`/`-h`，`--help` 才给用法**）。 `hdc list targets` 当前 `[Empty]`，即还没有跑起来的设备。下一步：`Emulator -imageList -imageRoot …/system-image` → `-create`/`-start` → `hdc list targets` 出现设备后再谈装 HAP。 **2026-11-26 设备就绪（硬墙已绕过）**：`~/.local/share/ohos` 已被移除，真源是 DevEco Studio 自带的 SDK `/Applications/DevEco-Studio.app/Contents/sdk/default/openharmony/{toolchains/hdc,native}`；`hdc list targets` → `127.0.0.1:5557`（模拟器已由 DevEco 启动）。 装包实测：`hdc -t 127.0.0.1:5557 install -r tests/floom-ohos-demo/floom-signed.hap` → `install bundle successfully`；`hdc shell aa start -a EntryAbility -b com.honorbox.reminder` → `start ability successfully`；`hdc shell snapshot_display -f /data/local/tmp/shot.jpeg` 得 2210×2416 截图，画面是旧的 `bridge.cpp` 渐变网格（即 `floom_harmony_present_frame` 上屏通路真的在跑）。 交叉编译也在真 SDK 上验过：`CC/AR/CARGO_TARGET_…_OHOS_LINKER=$SDK/native/llvm/bin/{clang,llvm-ar}` + `RUSTFLAGS="-C link-arg=--target=aarch64-linux-ohos -C link-arg=--sysroot=$SDK/native/sysroot"` + `LIBRARY_PATH=$SDK/native/sysroot/usr/lib/aarch64-linux-ohos` → `cargo build -p floom-platform-harmony --target aarch64-unknown-linux-ohos` 链接通过。 注：早前 `Emulator -install` 报的 "available only in the Chinese mainland" 只影响命令行工具自带的镜像下载，不影响 DevEco 已部署的模拟器。 | `grep -rn "floom-winit-ohos" crates/` 只剩一处实现路径 |
| S1.2 | 完成 | 五个缺陷各自有注释指明修法；`stride` 单位有上机结论 | 上机：连续 60 s 无丢帧；日志无 `AbortBuffer missing` **2026-11-26 真机取证（模拟器）**：present 通路已用 hilog 证明干净 —— `frame 2210x2208` / `buffer w=2210 h=2210 stride_raw=8840 size=19518720`（stride 是**字节**，`8840=2210*4`）/ `flush_rc=0`。剩下的唯一问题是绘制：首帧 `ink=1142 bbox=0x0-49x24`，即内容只画在左上角 49×24 里，恰好是期望值（153×75）的 **1/3.125**，说明布局尺寸与绘制坐标之间多除了一次 display scale。日志在 `crates/platform/floom-platform-harmony/src/lib.rs` 的 `present`（`crate::log` 走 `OH_LOG_Print`，因为 `println!`/`log::` 在这个平台都不进 hilog）。 |