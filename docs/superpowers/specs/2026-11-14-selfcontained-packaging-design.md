# Self-contained platform packaging in `floom-cli`

- Date: 2026-11-14
- Status: draft (awaiting approval)
- Owner: floom-cli

## Intent and why

`floom package <DIR> --platform <X>` today only builds the host binary, then —
*only inside a Floom checkout* — delegates platform packaging to
`scripts/build.sh` → `build-apk.sh` (Android) / `ios-bundle.sh` (iOS). Those
scripts `die` when a required *cargo subcommand* (`cargo-ndk`, …) is missing and
tell the user to install it by hand. So a developer must (a) be inside the Floem
repo and (b) have pre-installed the cargo tooling.

The goal: a standalone `floom` can package a project created by `floom create`
for any platform, **without the user manually installing the cargo tooling**
(`cargo-ndk`, `cargo-ios`/`cargo-xcode`, `wasm-bindgen-cli`). Per the conversation:

- **Direction B**: the orchestration lives in `floom-cli`; it does **not** depend
  on the repo's shell scripts.
- **Provisioning**: **detect → show plan → ask (`y/N`) → install** (prompt-then-
  install; `--yes` / `CI` auto-confirms).
- **Scope**: **all platforms at once** (android, ios, web, ohos, windows, linux,
  macos).
- **Input**: the native projects that `floom create` already emits (Gradle,
  Xcode/Swift, CMake, DevEco, static web). `floom package` *drives* them.
- **iOS**: uses `cargo-ios` / `cargo-xcode` to generate the Xcode project, then
  `xcodebuild` (not the hand-assembled `.app` from `ios-bundle.sh`).

## Why drive the native projects (not port `build-apk.sh`)

`build-apk.sh` is an SDK-only pipeline (`# No Gradle, no AGP download`) written
for the repo's **example crates**: a `cdylib` `.so` + a root `native-activity`
`AndroidManifest.xml` (no resources, no Kotlin) fed to `aapt2`/`zipalign`/
`apksigner` directly.

`floom create` emits something **incompatible**: a **bin-only** Rust crate, an
Android **Gradle** project whose manifest references `@mipmap`/`@string`/
`@style` and ships a **Kotlin** `MainActivity`, and an iOS **Xcode/Swift**
project. There is no `cdylib` and no resource-free manifest, so `build-apk.sh`'s
`aapt2 link` would fail on the resource references and `javac`/`d8` cannot
compile Kotlin.

Driving the native project is therefore the coherent path, and it is also the
smaller one: Gradle/AGP already does the manifest merge, Kotlin/Java → dex,
aapt2, zipalign and signing internally. The CLI's contribution is the
**toolchain provisioning** (the user's stated pain — "不需要再单独安装") plus
**orchestration** (run `cargo ndk` → `gradlew`, `cargo-ios` → `xcodebuild`).

## Goals

- `floom package --platform <X>` works on any directory created by `floom create`,
  with no Floom checkout required.
- Before building, provision the platform's **cargo subcommands** and **rustup
  targets**; if missing, print exactly what will run and ask, then install.
- **SDKs** (Android SDK + build-tools, a JDK, Gradle, Xcode, DevEco/hvigor) are
  *detected and required*: if absent, fail with a clear setup hint. They are not
  auto-installed (cannot be, cross-platform).
- Keep the same result contract as `scripts/build.sh`: a single
  `TIER=artifact|check-only|blocked PLATFORM=… CRATE=… ARTIFACT=…` line, exit 0
  for every tier.

## Non-goals / anti-goals

- **Not** porting `build-apk.sh`'s aapt2/zipalign/apksigner/d8 pipeline, nor
  `android-manifest-merge.py` / `android-plugin-modules.py` — Gradle/AGP do that
  for the create scaffold. (Those scripts remain in the repo for the example
  crates / CI and are **kept, not deleted**.)
- **Not** adding crate dependencies. The CLI stays buildable from `std` alone;
  provisioning is a runtime `cargo install`, not a build-time dep; `cargo
  metadata` JSON is parsed with a tiny hand-rolled walker (no `serde_json`).
- **Not** changing `floom create`'s output contract beyond what packaging needs
  (see Risks: the Android project currently has no `gradlew` script).
- **Not** changing `floom run` / `check` / `test`.

## Constraints

- Keep `crates/app/floom-cli` dependency-free (`[dependencies]` stays empty).
- All external work via `std::process::Command` (cargo, cargo-ndk, gradle,
  xcodebuild, wasm-bindgen, hvigorw, codesign, …).
- `floom create`'s generated native projects (Gradle/CMake/Xcode/DevEco) must
  remain openable in their IDEs; the packaging CLI must not rewrite them.

## Approach

### Module layout (new files in `crates/app/floom-cli/src/`)

- `toolchain.rs` — provisioning layer.
  - `enum CargoTool { Ndk, CargoIosOrXcode, WasmBindgen }` and
    `RustupTarget(&'static str)`; `is_present()` (e.g. `cargo ndk --version`,
    `rustup target list --installed` grep); `cargo_install_args()` (exact
    `cargo install …` argv, no version pins — matches the scripts); a
    `SdkPrecondition` set (AndroidSdk, Jdk, Gradle, Xcode, Hvigor) checked up
    front, each failing with a hint (`ANDROID_HOME`, `gradle`/`gradlew` path,
    `xcode-select -p`, `hvigorw`, …) reusing the existing SDK discovery from
    `build-apk.sh` / `native_project::detect_android_sdk`.
  - `provision(&[Tool]) -> Result<()>`: collect missing, print the plan
    (`missing: cargo-ndk → will run: cargo install cargo-ndk`), read `y/N` from
    stdin (auto-yes on `--yes` or `CI`), then run installs.
- `package.rs` — replaces `package()` in `main.rs`. Still builds the host binary
  first (matches current behavior; `--release` governs both builds), then
  dispatches to a `pack/<platform>` packer by `--platform`. No `--platform` →
  host binary + hint (today's behavior).
- `pack/mod.rs` — `pub struct PackResult { tier, artifact: Option<PathBuf>, note:
  Option<String> }` and the per-platform dispatch.
- `pack/android.rs` — provision `[Ndk, RustupTarget("aarch64-linux-android")]`
  + SDK preconditions; run `cargo ndk -t <abis> -P 24 -o <dir>/android/app/src/
  main/jniLibs build --release`; then run Gradle
  (`gradlew` if present else `gradle` on PATH) `assembleRelease` in
  `<dir>/android`, parse the produced APK path.
- `pack/ios.rs` — provision `[CargoIosOrXcode, RustupTarget("aarch64-apple-ios-
  sim")]` + Xcode; run `cargo ios`/`cargo xcode` to generate/sync the Xcode
  project from `<dir>/ios`, then `xcodebuild -scheme … -configuration Release`,
  ad-hoc `codesign`. Device SDK (`aarch64-apple-ios`) → tier `blocked` with a
  note (needs a real identity), but the build still proceeds where possible.
- `pack/web.rs` — `cargo build --target wasm32-unknown-unknown` +
  provision `WasmBindgen`; run `wasm-bindgen --target web`; no bindings →
  `check-only` tier reporting the raw `.wasm`.
- `pack/ohos.rs` — provision RustupTarget("aarch64-unknown-linux-ohos"); `cargo
  build --target …`; stage `.so` into the hvigor project; `hvigorw assembleHap`
  (SDK precondition). Missing hvigor → `blocked`.
- `pack/desktop.rs` — `windows`/`linux`/`macos`: provision cmake (SDK
  precondition) + rustup targets as needed; `cmake -B build && cmake --build
  build` from `<dir>/<platform>`.

### Result contract

`pack()` returns `PackResult { tier, artifact, note }`; `floom package` prints the
single `TIER=… PLATFORM=… CRATE=… ARTIFACT=…` line (mirroring `build.sh:353`) and
exits 0 for every tier, including `check-only`/`blocked`. Crate name/workspace
root come from `cargo metadata` at `<DIR>`; artifacts land in the metadata
`target_directory`. `crate_name`/`find_build_sh` are retired from the platform
path (their tests updated).

### Tests

Unit-test the pure-Rust pieces, mirroring the existing `#[cfg(test)]` style:
tool-present parsing, ABI↔target mapping, provisioning plan text,
gradle/xcodebuild argv construction, tier selection. Builds that need an SDK are
not unit-tested (external tools); they are covered by doc comments.

## Risks / gaps in the create scaffold the CLI must bridge

- **Android has no `gradlew` script** (only `gradle-wrapper.properties`). The
  packer must use `gradle` from PATH (detect-require) or generate the wrapper
  (`gradle wrapper`) where Gradle is installed. This is a scaffolding gap, not a
  CLI rewrite of the project.
- **iOS has no `.xcodeproj`** — that is exactly what `cargo-ios`/`cargo-xcode`
  generates from the `Info.plist` + Swift sources; the packer relies on it.
- The create scaffold's Rust core is **bin-only**; for Android it must be a
  `cdylib` so `cargo ndk` produces a `.so`. This either needs `floom create` to
  also emit a `cdylib` lib target for mobile platforms, or the packer to pass a
  lib build. **Open:** confirm how the `.so` is produced from the bin-only
  scaffold (likely: add a `cdylib` `[lib]` to the create template for mobile
  targets). This is the one change to `floom create` the packaging path needs.
- Gradle/AGP first build resolves dependencies over the network; that is expected
  and out of scope to automate.

## Decisions locked by the conversation

- Direction **B**, prompt-then-install, all platforms at once, target the
  `floom create` scaffold, drive its native projects, iOS via cargo-ios/xcode,
  SDKs detect-and-require.

## Builder-check residue (settled as my calls)

- `pack()` returns a struct, not a bare `PathBuf`; `TIER=…` line printed, exit 0
  for all tiers.
- Keep the host `cargo build --release` first; `--release` governs both; preserve
  `build.sh`'s default-feature derivation (`<platform>` → `real` → none).
- Package-only flag parser for `--abi/--manifest/--install/--device/--bin/--yes`;
  `parse_lifecycle` stays untouched for `check`/`test`.
- Provision only tools the pipeline invokes; no version pins.
- CLI stays std-only; hand-rolled `cargo metadata` walker.
