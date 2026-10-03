# cargo-ohos-app macOS 跨平台补丁

让 `taoyuan11/harmony-app-rs`（`cargo-ohos-app`，pre-1.0）在 macOS / Linux 上可用。
原版在 `discover_hvigor` 里**强制要求 `tools/hvigor/bin/hvigorw.bat` 存在**，而
DevEco Studio 在 macOS/Linux 上的包装器是 `hvigorw`（无扩展名），`.bat` 仅 Windows 有，
因此原版实质是 Windows-only，macOS 上会在 SDK 检测阶段直接 `MissingFile` 失败。

## 改动内容

| 文件 | 改动 |
|---|---|
| `src/sdk.rs` | `HvigorInfo` 新增 `wrapper: PathBuf` 字段；`discover_hvigor` 按 `target_os` 选择 `hvigorw`（Unix）/ `hvigorw.bat`（Windows），并把 `hvigorw.bat` 从「全平台必填」降为「仅 Windows 必填」。 |
| `src/commands/package.rs` | hvigor 命令程序由 `app.hvigor.wrapper_bat` 改为 `app.hvigor.wrapper`（即用上面选出的正确包装器）；`ohpm install` 失败回退由写死的 `npm.cmd` 改为按平台选 `npm` / `npm.cmd`。 |
| `src/commands/mod.rs` | 链接器 `clang` / `clang++` / `llvm-ar` 原版硬编码 `.exe` 后缀（Windows-only），改为 `std::env::consts::EXE_SUFFIX`，macOS/Linux 上不再找 `clang.exe`。 |
| `src/commands/build.rs` + `package.rs` | staticlib 产物解析增加 `deps/` 回退：manifest 未声明 `staticlib` 时（如 floom 的 `examples/ohos` 是 cdylib），`cargo rustc -- --crate-type staticlib` 产出的 `.a` 只落在 `<profile>/deps/`，不会被 cargo 硬链到 `<profile>/` 根。 |
| `src/template.rs` | ① `build-profile.json5` 的 `compileSdkVersion` 等三个字段加引号（裸 `26.0.0` 不是合法 JSON 数字，hvigor 报 JSON5 Syntax Error）；② 工程级 `oh-package.json5` 与 `hvigor/hvigor-config.json5` 的 `modelVersion` 由 `5.0.0` 升到 `6.0.0`——API 26+ 要求 modelVersion ≥ 6.0.0 才接受字符串形式的 SDK 版本。 |

`src/runner.rs` 是平台无关的 `Command` 封装，无需改。

## SDK 布局注意（DevEco 安装的 SDK）

DevEco Studio 装到 `~/.local/share/ohos/<version>/` 的 OpenHarmony SDK **只含
`ets/js/previewer/toolchains`，没有 `native`**（clang 工具链）。而 command-line-tools
的 SDK（`~/.local/share/ohos/command-line-tools/sdk/default/openharmony/`）才有
`native`。cargo-ohos-app 需要 `<sdk_root>/<version>/native`，补一条软链即可：

```bash
ln -sfn ~/.local/share/ohos/command-line-tools/sdk/default/openharmony/native \
        ~/.local/share/ohos/26.0.0/native
```

另外 `discover_latest_sdk_version` 只认整数版本目录，`"26.0.0"` 必须显式传
`--sdk-version 26.0.0`。

## 应用方式

```bash
# 1. fork 并 clone
git clone https://github.com/<you>/harmony-app-rs.git
cd harmony-app-rs

# 2. 打补丁（在仓库根目录执行）
git apply /path/to/macos-crossplatform.patch

# 3. 本地安装 cargo 子命令
cargo install --path . --bin cargo-ohos-app
# 或开发模式：cargo run -- <args>
```

也可直接把 `sdk.rs` / `package.rs`（本目录提供的完整修改版）覆盖到对应路径。

## macOS 前置依赖（补丁不替代这些）

1. **DevEco Studio**（完整安装，非纯 CLI）—— 提供 `tools/hvigor/bin/hvigorw`。
   配置里的 `deveco_studio_dir` 应指向含 `tools/hvigor/bin/hvigorw` 的目录，
   典型为 `/Applications/DevEco-Studio.app/Contents`。
2. **OpenHarmony SDK** —— 配置 `sdk_root`（你机器上在 `/Users/ahs/Library/OpenHarmony`）。
3. **ohpm** —— 配置 `ohpm_path`（macOS 上是 `ohpm` 可执行文件，非 `ohpm.bat`）。
4. **联网** —— `ohpm install` 需要访问仓库。
5. **签名** —— 默认产出**未签名** HAP；真机安装需另配 `signingConfigs`。
   验证可先用 `--target x86_64` 出**模拟器** HAP，免真机。

## 验证建议

补丁应用后，在 floom 的 OHOS 集成完成前，可先用上游 `examples/winit-smoke` 验证整条
打包链路在 macOS 上是否通：能生成壳 → 编出 `lib{name}.a` → `ohpm install` →
`hvigorw` 调起出 HAP。能通过即说明本补丁生效、工具链可用。
