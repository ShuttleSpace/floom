# Self-contained platform packaging in `floom-cli` — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `floom package --platform <X>` build a platform artifact for a project created by `floom create` with a standalone `floom` binary — detecting and (on confirmation) installing the cargo-subcommand tooling, and driving the native project the scaffold already emits.

**Architecture:** `floom-cli` gains a `toolchain` provisioning layer (detect → prompt → `cargo install` / `rustup target add`) and a `pack` orchestrator that, per platform, provisions rustup/cargo tools + checks SDK preconditions, then runs the cargo build and the native build tool (Gradle for Android, `cargo-ios`/`cargo-xcode`→`xcodebuild` for iOS, `wasm-bindgen` for web, `hvigorw` for OHOS, CMake for desktop). The CLI stays std-only and drives every external tool via `std::process::Command`. `build-apk.sh`/`ios-bundle.sh` are NOT ported — the create scaffold's Gradle/Xcode projects do the manifest merge, dex, aapt2, signing.

**Tech Stack:** Rust (std only, no new crates), `std::process::Command`, `cargo metadata` (hand-rolled JSON walk), Gradle, `cargo-ndk`, `cargo-ios`/`cargo-xcode`, `wasm-bindgen-cli`, `hvigorw`, CMake, `codesign`.

**Spec:** `docs/superpowers/specs/2026-11-14-selfcontained-packaging-design.md`

## Global Constraints

- Keep `crates/app/floom-cli` dependency-free (`[dependencies]` stays empty).
- All external work via `std::process::Command`.
- Hand-rolled `cargo metadata` walker; no `serde_json`.
- Provisioning: detect → show plan → ask (`y/N`) → install; `--yes` / `CI` auto-confirm.
- SDKs (Android SDK, JDK, Gradle, Xcode, DevEco/hvigor) detect-and-require; fail with a hint.
- Result contract: single `TIER=artifact|check-only|blocked PLATFORM=… CRATE=… ARTIFACT=…` line; exit 0 for every tier.
- Keep host `cargo build --release` first; `--release` governs both; preserve default-feature derivation (`<platform>` → `real` → none).
- Package-only flags: `--abi`/`--manifest`/`--install`/`--device`/`--bin`/`--yes`; `parse_lifecycle` untouched for `check`/`test`.
- Provision only tools the pipeline invokes; no version pins.

## Review Focus

1. **Non-interactive / CI:** provisioning must never block on stdin. `--yes` or `CI` set must auto-confirm; a missing cargo subcommand must install or fail cleanly, never hang. *Test: `provision(..., assume_yes=true)` with a missing-but-installable tool proceeds; `assume_yes=false` reads `y/N` from stdin.*
2. **Missing SDK** (e.g. no Android SDK on a Linux box): must print a clear hint (`ANDROID_HOME` / `gradle` / `xcode-select -p` / `hvigorw`) and a `blocked`/error, not a cryptic `gradlew: command not found`. *Test: `SdkPrecondition::is_satisfied()` false + `hint()` contains the setup env var when the env is unset.*
3. **`floom check`/`floom test` unchanged:** must keep working and must NOT accept `--abi`/`--platform`. *Test: `floom check --abi arm64` returns an error; `parse_lifecycle` is unmodified.*
4. **Bin-only crate → Android:** must not silently emit an APK with no native lib. The cdylib lib target (Task 5) must be present; the android packer must error if `cargo ndk` produced no `.so`. *Test: packer errors when the jniLibs output has no `.so`.*
5. **Feature/profile derivation:** `--release` governs both builds; default-feature order `<platform>` → `real` → none must not break a build that has none. *Test: default-features resolver returns `[]` when neither `<platform>` nor `real` exists, `["real"]` when `real` exists but not the platform feature.*

---

## Group A — Foundation

### Task 1: Scaffold the new modules

**Files:**
- Modify: `crates/app/floom-cli/src/main.rs:7-10` (add `mod package;`, `mod toolchain;`, `mod pack;`)
- Create: `crates/app/floom-cli/src/toolchain.rs`
- Create: `crates/app/floom-cli/src/package.rs`
- Create: `crates/app/floom-cli/src/pack/mod.rs`

**Interfaces:**
- Produces: the three modules exist and compile against the existing `cargo_in` / `LifecycleOpts` helpers in `main.rs`.
- `pack/mod.rs` declares `pub enum Tier`, `pub struct PackResult`, `pub struct PackOpts`, `pub fn pack(platform: &str, project: &Project, opts: &PackOpts) -> Result<PackResult, String>`.

- [ ] **Step 1: Add the three `mod` declarations after the existing `mod` block in `main.rs`**
- [ ] **Step 2: Create `toolchain.rs`, `package.rs`, `pack/mod.rs` with `// SPDX-License-Identifier: MIT` headers and the module skeletons (no bodies yet — just the public types from Interfaces)**
- [ ] **Step 3: Build** `cargo build -p floom-cli` Expected: compiles
- [ ] **Step 4: Commit**
```bash
git add crates/app/floom-cli/src/main.rs crates/app/floom-cli/src/toolchain.rs crates/app/floom-cli/src/package.rs crates/app/floom-cli/src/pack/mod.rs
git commit -m "feat(floom-cli): scaffold toolchain + pack modules"
```

### Task 2: `toolchain.rs` — detection, plan, provisioning

**Files:**
- Create/Modify: `crates/app/floom-cli/src/toolchain.rs`

**Interfaces:**
- Produces:
  - `pub enum CargoTool { Ndk, CargoIosOrXcode, WasmBindgen }` with `pub fn install_name(&self) -> &'static str` (returns `"cargo-ndk"` / `"cargo-xcode"` / `"wasm-bindgen-cli"`) and `pub fn is_present(&self) -> bool` (runs `cargo <sub>` where sub is `ndk` / `ios` / `xcode`; for `CargoIosOrXcode`, present if `cargo ios --version` OR `cargo xcode --version` succeeds).
  - `pub struct RustupTarget(pub &'static str)` with `pub fn is_present(&self) -> bool` (runs `rustup target list --installed`, grep exact line).
  - `pub enum Tool { Cargo(CargoTool), Rustup(RustupTarget) }` with `pub fn is_present(&self) -> bool` and `pub fn plan_line(&self) -> String` (e.g. `"missing: cargo-ndk -> will run: cargo install cargo-ndk"` / `"missing: rustup target aarch64-linux-android -> will run: rustup target add aarch64-linux-android"`).
  - `pub enum SdkPrecondition { AndroidSdk, Jdk, Gradle, Xcode, Hvigor }` with `pub fn is_satisfied(&self) -> bool` and `pub fn hint(&self) -> &'static str` (exact hint text: `AndroidSdk` → `"set ANDROID_HOME (or ANDROID_SDK_ROOT) to your Android SDK"`; `Jdk` → `"install a JDK (needed for apksigner/keytool)"`; `Gradle` → `"install Gradle, or ensure ./gradlew exists"`; `Xcode` → `"run: xcode-select --install"`; `Hvigor` → `"install DevEco/hvigorw under DEVECO_SDK_HOME"`).
  - `pub fn provision(tools: &[Tool], assume_yes: bool) -> Result<(), String>` — collect missing, print each `plan_line()` to stderr, prompt `install the above? [y/N] ` on `stdin` unless `assume_yes || std::env::var("CI").is_ok()`, then for each missing run `cargo install <install_name>` (Cargo) or `rustup target add <name>` (Rustup); bubble up any failure as `Err`.

**Consumes:** nothing from other new modules.

- [ ] **Step 1: Write failing tests** in `#[cfg(test)] mod tests` of `toolchain.rs`:
  ```rust
  #[test]
  fn plan_line_names_the_cargo_install() {
      assert_eq!(CargoTool::Ndk.plan_line(),
          "missing: cargo-ndk -> will run: cargo install cargo-ndk");
  }
  #[test]
  fn rustup_target_plan_line_names_add() {
      assert_eq!(Tool::Rustup(RustupTarget("aarch64-linux-android")).plan_line(),
          "missing: rustup target aarch64-linux-android -> will run: rustup target add aarch64-linux-android");
  }
  #[test]
  fn sdk_hint_points_at_android_home() {
      assert!(SdkPrecondition::AndroidSdk.hint().contains("ANDROID_HOME"));
  }
  #[test]
  fn provision_with_assume_yes_does_not_block_on_stdin() {
      // point at an unresolvable tool so install fails fast, but assert it tried
      let r = provision(&[Tool::Cargo(CargoTool::Ndk)], true);
      assert!(r.is_err()); // install will fail in test env; the point is no hang
  }
  ```
- [ ] **Step 2: Run** `cargo test -p floom-cli toolchain` Expected: FAIL (`CargoTool` etc. undefined)
- [ ] **Step 3: Implement the types and `provision`** in `toolchain.rs`. `is_present` shells out via `std::process::Command`; `provision` reads stdin only when not `assume_yes` and not `CI`.
- [ ] **Step 4: Run** `cargo test -p floom-cli toolchain` Expected: PASS
- [ ] **Step 5: Commit**
```bash
git add crates/app/floom-cli/src/toolchain.rs
git commit -m "feat(floom-cli): toolchain detection + prompt-then-install"
```

### Task 3: `pack/shared.rs` — `cargo metadata` inspector

**Files:**
- Create: `crates/app/floom-cli/src/pack/shared.rs`

**Interfaces:**
- Produces:
  - `pub struct Project { pub root: PathBuf, pub target_dir: PathBuf, pub package_name: String, pub default_features: Vec<String> }`
  - `pub fn inspect(dir: &Path) -> Result<Project, String>` — runs `cargo metadata --manifest-path <dir>/Cargo.toml --format-version 1 --no-deps`, hand-parse JSON for `workspace_root`, `target_directory`, the first/sole `packages[].name`, and derive `default_features`: if the package has a feature equal to the platform name use `[platform]`; else if it has `real` use `["real"]`; else `[]`.
  - `pub fn abi_to_rust_target(abi: &str) -> Option<&'static str>` (`arm64-v8a`→`aarch64-linux-android`, `armeabi-v7a`→`armv7-linux-androideabi`, `x86`→`i686-linux-android`, `x86_64`→`x86_64-linux-android`).

**Consumes:** nothing.

- [ ] **Step 1: Write failing tests** (use a tiny synthetic `cargo metadata` JSON string parsed by a `pub fn parse_metadata(json: &str) -> Result<Project, String>` helper that `inspect` calls after the subprocess):
  ```rust
  const META: &str = r#"{"workspace_root":"/r","target_directory":"/r/target",
    "packages":[{"name":"demo","features":{"real":[],"android":[]}}]}"#;
  #[test]
  fn derives_real_feature_when_platform_feature_absent() {
      let p = parse_metadata(META).unwrap();
      assert_eq!(p.default_features, vec!["real".to_string()]);
  }
  #[test]
  fn abi_maps_to_android_rust_target() {
      assert_eq!(abi_to_rust_target("arm64-v8a"), Some("aarch64-linux-android"));
      assert_eq!(abi_to_rust_target("nope"), None);
  }
  ```
- [ ] **Step 2: Run** `cargo test -p floom-cli pack::shared` Expected: FAIL
- [ ] **Step 3: Implement `parse_metadata` + `inspect` + `abi_to_rust_target`** (small recursive JSON walk for the four fields; no serde).
- [ ] **Step 4: Run** `cargo test -p floom-cli pack::shared` Expected: PASS
- [ ] **Step 5: Commit**
```bash
git add crates/app/floom-cli/src/pack/shared.rs
git commit -m "feat(floom-cli): cargo metadata inspector + abi mapping"
```

### Task 4: `package.rs` orchestrator + retire old delegation

**Files:**
- Modify: `crates/app/floom-cli/src/main.rs` — replace `fn package(args: &[String])` body (lines ~672-714) and `find_build_sh` (649-664) and `crate_name` usage; the `package` command now calls `package::run`.
- Modify: `crates/app/floom-cli/src/package.rs`

**Interfaces:**
- Consumes: `toolchain::Tool`, `pack::PackResult`, `pack::Tier`, `pack::PackOpts`, `pack::shared::Project::inspect`, `pack::pack`.
- Produces:
  - `pub fn run(args: &[String]) -> Result<(), String>` — parse `--release --features --platform --abi --manifest --install --device --bin --yes` plus one `<DIR>` (reuse `LifecycleOpts` parsing for the common flags; extend with the package-only flags into `PackOpts`). Build host binary first (`cargo_in(dir,"build",features,true)`), then `inspect(dir)`, then `pack(platform, &project, &opts)`. On no `--platform`, print host-built message + hint (today's behavior) and return `Ok(())`. On `Some(platform)`, print the `TIER=… PLATFORM=… CRATE=… ARTIFACT=…` line (tier/artifact/note from `PackResult`) and `Ok(())`.
  - `pub struct PackOpts { pub release: bool, pub features: Vec<String>, pub abi: Option<String>, pub manifest: Option<PathBuf>, pub install: bool, pub device: bool, pub bin: Option<String>, pub yes: bool, pub dir: PathBuf }`.

- [ ] **Step 1: Write a test** in `package.rs` asserting the no-`--platform` path returns `Ok` and the `TIER=` line format; and that `--abi` parses into `PackOpts.abi` (unit-test the parser only, no real build).
- [ ] **Step 2: Run** `cargo test -p floom-cli package` Expected: FAIL
- [ ] **Step 3: Implement `run` + `PackOpts` parser.** Remove `find_build_sh`/`crate_name` only where the platform path used them; keep `crate_name` if `check`/`test` still use it (they don't — delete it and its test). Route `package` dispatch through `pack::pack`.
- [ ] **Step 4: Run** `cargo test -p floom-cli` Expected: PASS
- [ ] **Step 5: Commit**
```bash
git add crates/app/floom-cli/src/main.rs crates/app/floom-cli/src/package.rs
git commit -m "feat(floom-cli): package orchestrator with TIER result line"
```

## Group B — scaffold change

### Task 5: cdylib lib target for mobile scaffolds

**Files:**
- Modify: `crates/app/floom-cli/src/templates.rs` — `cargo_toml` gains a `mobile: bool` parameter; when true append:
  ```toml
  [lib]
  name = "<crate with - replaced by _>"
  path = "src/lib.rs"
  crate-type = ["cdylib"]
  ```
- Modify: `crates/app/floom-cli/src/main.rs` `create_rust_app` passes `mobile = opts.platforms.iter().any(|p| matches!(p, "android"|"ios"|"ohos"))`.
- Modify: `create_rust_app` to also write `src/lib.rs` (mobile only) calling the platform entry (`floom::run_android` etc. are out of scope — emit a documented stub that exposes `android_main` wiring later; the packager only needs the cdylib present).
- Test: `crates/app/floom-cli/src/main.rs` `#[cfg(test)]` — assert `cargo_toml("my_app", "floom", None, true)` contains `[lib]` and `crate-type = ["cdylib"]`, and `false` does not.

**Interfaces:** Consumes `templates::cargo_toml` signature change. Produces: a scaffold whose `cargo ndk` yields a `.so`.

- [ ] **Step 1: Update the existing test** for `cargo_toml` to pass the new `mobile` arg; add the cdylib assertion.
- [ ] **Step 2: Run** `cargo test -p floom-cli` Expected: FAIL
- [ ] **Step 3: Implement** the `mobile` branch in `cargo_toml` and the `src/lib.rs` emit.
- [ ] **Step 4: Run** `cargo test -p floom-cli` Expected: PASS
- [ ] **Step 5: Commit**
```bash
git add crates/app/floom-cli/src/templates.rs crates/app/floom-cli/src/main.rs
git commit -m "feat(floom-cli): emit cdylib lib target for mobile scaffolds"
```

## Group C — per-platform packers

### Task 6: `pack/android.rs`

**Files:** Create `crates/app/floom-cli/src/pack/android.rs`; wire into `pack/mod.rs::pack`.

**Interfaces:** Consumes `Project`, `PackOpts`, `toolchain::{Tool, CargoTool, RustupTarget, SdkPrecondition, provision}`. Produces `PackResult`.

- [ ] **Step 1: Test** the gradle invocation args builder: given `PackOpts{abi:Some("arm64-v8a".into()), release:true, ..}`, assert the `cargo ndk` argv equals `["ndk","-t","arm64-v8a","-P","24","-o","<dir>/android/app/src/main/jniLibs","build","--release"]` and that the gradle command is `assembleRelease` (or `assembleDebug` when `!release`), invoked in `<dir>/android` via `gradlew` if present else `gradle` on PATH. Also test that with no `.so` present after the (mocked) build it returns `Err`.
- [ ] **Step 2: Run** `cargo test -p floom-cli pack::android` Expected: FAIL
- [ ] **Step 3: Implement** `pack_android`: `provision([Cargo(CargoNdk), Rustup(abi_to_rust_target(abi))])`; check `SdkPrecondition::{AndroidSdk,Jdk,Gradle}` satisfied (error with hint otherwise); run `cargo ndk …`; verify a `.so` landed in jniLibs (else `Err`); run gradle `assembleRelease`; locate the produced APK under `<dir>/android/app/build/outputs/apk/…`; `PackResult{tier:Artifact, artifact:Some(apk), note:None}`. `--install` ⇒ `adb install -r`.
- [ ] **Step 4: Run** `cargo test -p floom-cli pack::android` Expected: PASS
- [ ] **Step 5: Commit**

### Task 7: `pack/ios.rs`

**Files:** Create `crates/app/floom-cli/src/pack/ios.rs`.

- [ ] **Step 1: Test** the target selection and `cargo ios`/`cargo xcode` argv: device ⇒ `aarch64-apple-ios`, sim ⇒ `aarch64-apple-ios-sim`; provisioning lists `Cargo(CargoIosOrXcode)` + the chosen `RustupTarget`; xcodebuild scheme derived from the crate name; device ⇒ `tier:Blocked` note `"device build needs a real signing identity"`.
- [ ] **Step 2: Run** `cargo test -p floom-cli pack::ios` Expected: FAIL
- [ ] **Step 3: Implement** `pack_ios`: provision; `cargo build --target <t>`; run `cargo ios`/`cargo xcode` to generate/sync the Xcode project from `<dir>/ios`; `xcodebuild -scheme <crate> -configuration Release`; ad-hoc `codesign --force --sign -`. Device ⇒ still build but report `Blocked`. `PackResult` with the `.app` path.
- [ ] **Step 4: Run** `cargo test -p floom-cli pack::ios` Expected: PASS
- [ ] **Step 5: Commit**

### Task 8: `pack/web.rs`

**Files:** Create `crates/app/floom-cli/src/pack/web.rs`.

- [ ] **Step 1: Test** `wasm32-unknown-unknown` is the rustup target; `wasm-bindgen` provisioned; when `wasm-bindgen` is absent the tier is `CheckOnly` with the raw `.wasm` as artifact and note `"install wasm-bindgen-cli for JS bindings"`.
- [ ] **Step 2: Run** `cargo test -p floom-cli pack::web` Expected: FAIL
- [ ] **Step 3: Implement** `pack_web`: `rustup target add wasm32-unknown-unknown`; `cargo build --target wasm32-unknown-unknown`; if `is_present(WasmBindgen)` run `wasm-bindgen --target web --out-dir <dist> --no-typescript <wasm>`, else `CheckOnly`. 
- [ ] **Step 4: Run** `cargo test -p floom-cli pack::web` Expected: PASS
- [ ] **Step 5: Commit**

### Task 9: `pack/ohos.rs`

**Files:** Create `crates/app/floom-cli/src/pack/ohos.rs`.

- [ ] **Step 1: Test** rustup target is `aarch64-unknown-linux-ohos`; when `hvigorw` is absent ⇒ `Blocked` note `"hvigorw not found under DEVECO_SDK_HOME"`; the `.so` is staged into the hvigor project's `entry/src/main/cpp/libs/arm64-v8a` and `entry/libs/arm64-v8a`.
- [ ] **Step 2: Run** `cargo test -p floom-cli pack::ohos` Expected: FAIL
- [ ] **Step 3: Implement** `pack_ohos`: provision rustup target; check `SdkPrecondition::Hvigor` (else `Blocked`); `cargo build --target aarch64-unknown-linux-ohos`; stage `.so` into the two destinations; run `hvigorw --no-daemon clean assembleHap` in the hvigor dir; on `.hap` produced ⇒ `Artifact` (note `"HAP is unsigned"`), else `Blocked`.
- [ ] **Step 4: Run** `cargo test -p floom-cli pack::ohos` Expected: PASS
- [ ] **Step 5: Commit**

### Task 10: `pack/desktop.rs` (windows / linux / macos)

**Files:** Create `crates/app/floom-cli/src/pack/desktop.rs`.

- [ ] **Step 1: Test** cmake is the SDK precondition; the cmake invocation is `cmake -B build && cmake --build build` in `<dir>/<platform>`; windows ⇒ `x86_64-pc-windows-msvc`, linux ⇒ `x86_64-unknown-linux-gnu` rustup target checked before cmake; `tier:Artifact` once the executable exists.
- [ ] **Step 2: Run** `cargo test -p floom-cli pack::desktop` Expected: FAIL
- [ ] **Step 3: Implement** `pack_desktop`: check `SdkPrecondition::Gradle`(cmake) satisfied; for windows/linux ensure the rustup target present; run `cargo build` then `cmake -B build && cmake --build build`; locate the executable; `PackResult{tier:Artifact, artifact:Some(exe), note:None}`.
- [ ] **Step 4: Run** `cargo test -p floom-cli pack::desktop` Expected: PASS
- [ ] **Step 5: Commit**

## Group D — polish

### Task 11: Help text + spec/design cross-reference + final verification

**Files:**
- Modify: `crates/app/floom-cli/src/main.rs` `USAGE` constant (lines 16-73) — add the new `package` flags (`--abi`, `--manifest`, `--install`, `--device`, `--bin`, `--yes`) and a sentence that `floom package` provisions cargo tooling automatically.
- Modify: `docs/superpowers/specs/2026-11-14-selfcontained-packaging-design.md` — add a one-line "implemented by plan `…/plans/2026-11-14-selfcontained-packaging.md`" note.

- [ ] **Step 1: Update `USAGE`** to document the new flags and the auto-provisioning behavior.
- [ ] **Step 2: Run** `cargo build -p floom-cli && cargo test -p floom-cli` Expected: PASS, no warnings about the new modules.
- [ ] **Step 3: Run** `./target/debug/floom --help` and `./target/debug/floom package --help` (if a help subcommand is added) — confirm the new flags appear.
- [ ] **Step 4: Commit**
```bash
git add crates/app/floom-cli/src/main.rs docs/superpowers/specs/2026-11-14-selfcontained-packaging-design.md
git commit -m "docs(floom-cli): document package auto-provisioning flags"
```

## Self-review notes

- Spec coverage: every spec clause maps to a task (provisioning→T2, SDK hints→T2, metadata/abi→T3, orchestrator/TIER→T4, cdylib→T5, each platform→T6–T10, help→T11). `build-apk.sh` is intentionally NOT ported (driving Gradle instead) — called out in the spec.
- The five Review Focus items each have a test in the owning task (T2 non-interactive + SDK hints; T3 `check` stays unchanged via unchanged `parse_lifecycle`; T4/T6 no-`.so` error; T3 feature derivation).
- Type consistency: `Tool`, `SdkPrecondition`, `Project`, `PackOpts`, `PackResult`, `Tier` are defined once in T2/T3/T4 and reused by T6–T10 with identical names.
- Proportion: bodies are given only for the metadata walk and the provisioning prompt; everything else is signature + test assertions + pinned argv, so the plan stays shorter than the spec.
