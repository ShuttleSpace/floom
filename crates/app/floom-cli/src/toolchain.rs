// SPDX-License-Identifier: MIT
//! Cargo-subcommand / rustup-target provisioning.
//!
//! `floom package` needs tools the user may not have installed (`cargo-ndk`,
//! `cargo-xcode`, `wasm-bindgen-cli`, cross-compilation targets). This module
//! detects them, prints exactly what it would run, and — only after a `y/N`
//! confirmation (or `--yes` / `CI`) — runs the install. Platform SDKs are a
//! separate `SdkPrecondition` check: they cannot be auto-installed, so they
//! fail with a hint instead.

use std::io::Write;
use std::process::{Command, Stdio};

/// A cargo subcommand installed via `cargo install <crate>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CargoTool {
    Ndk,
    CargoIosOrXcode,
    WasmBindgen,
}

impl CargoTool {
    /// The crate name `cargo install` takes (no version pin — matches the
    /// scripts, which pin none either).
    pub fn install_name(&self) -> &'static str {
        match self {
            CargoTool::Ndk => "cargo-ndk",
            CargoTool::CargoIosOrXcode => "cargo-xcode",
            CargoTool::WasmBindgen => "wasm-bindgen-cli",
        }
    }

    /// A cargo subcommand is present when `cargo <sub> --version` succeeds.
    /// `CargoIosOrXcode` accepts either spelling, since both generate an Xcode
    /// project and either may be the one installed.
    pub fn is_present(&self) -> bool {
        match self {
            CargoTool::Ndk => cargo_subcommand_present("ndk"),
            CargoTool::WasmBindgen => command_succeeds("wasm-bindgen", &["--version"]),
            CargoTool::CargoIosOrXcode => {
                cargo_subcommand_present("ios") || cargo_subcommand_present("xcode")
            }
        }
    }
}

/// A cross-compilation target installed via `rustup target add`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RustupTarget(pub &'static str);

impl RustupTarget {
    pub fn is_present(&self) -> bool {
        let Ok(output) = Command::new("rustup")
            .args(["target", "list", "--installed"])
            .stderr(Stdio::null())
            .output()
        else {
            return false;
        };
        output.status.success()
            && String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(|line| line.trim() == self.0)
    }
}

/// Anything the provisioning layer knows how to install.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Cargo(CargoTool),
    Rustup(RustupTarget),
}

impl Tool {
    pub fn is_present(&self) -> bool {
        match self {
            Tool::Cargo(t) => t.is_present(),
            Tool::Rustup(t) => t.is_present(),
        }
    }

    /// The exact line printed before installing.
    pub fn plan_line(&self) -> String {
        match self {
            Tool::Cargo(t) => format!(
                "missing: {} -> will run: cargo install {}",
                t.install_name(),
                t.install_name()
            ),
            Tool::Rustup(t) => format!(
                "missing: rustup target {} -> will run: rustup target add {}",
                t.0, t.0
            ),
        }
    }

    /// The exact argv used to install. Kept separate from `provision` so its
    /// value is testable without triggering a real install.
    pub fn install_argv(&self) -> Vec<String> {
        match self {
            Tool::Cargo(t) => vec!["cargo", "install", t.install_name()]
                .into_iter()
                .map(String::from)
                .collect(),
            Tool::Rustup(t) => vec!["rustup", "target", "add", t.0]
                .into_iter()
                .map(String::from)
                .collect(),
        }
    }
}

/// A platform SDK that cannot be auto-installed, only detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdkPrecondition {
    AndroidSdk,
    Jdk,
    Gradle,
    Xcode,
    Hvigor,
    Cmake,
}

impl SdkPrecondition {
    pub fn is_satisfied(&self) -> bool {
        match self {
            SdkPrecondition::AndroidSdk => crate::native_project::detect_android_sdk().is_some(),
            SdkPrecondition::Jdk => command_succeeds("java", &["-version"]),
            SdkPrecondition::Gradle => command_succeeds("gradle", &["--version"]),
            SdkPrecondition::Xcode => command_succeeds("xcode-select", &["-p"]),
            SdkPrecondition::Hvigor => {
                std::env::var_os("DEVECO_SDK_HOME").is_some()
                    || command_succeeds("hvigorw", &["--version"])
            }
            SdkPrecondition::Cmake => command_succeeds("cmake", &["--version"]),
        }
    }

    /// What a person must do to satisfy it. Printed verbatim when it is missing.
    pub fn hint(&self) -> &'static str {
        match self {
            SdkPrecondition::AndroidSdk => {
                "set ANDROID_HOME (or ANDROID_SDK_ROOT) to your Android SDK"
            }
            SdkPrecondition::Jdk => "install a JDK (needed for apksigner/keytool/Gradle)",
            SdkPrecondition::Gradle => {
                "install Gradle (`gradle` on PATH), or ensure ./gradlew exists in the android/ project"
            }
            SdkPrecondition::Xcode => "run: xcode-select --install (Xcode is required for iOS)",
            SdkPrecondition::Hvigor => {
                "install DevEco/hvigorw or set DEVECO_SDK_HOME (needed for the OHOS HAP)"
            }
            SdkPrecondition::Cmake => "install CMake (`cmake` on PATH), needed for desktop builds",
        }
    }
}

/// Whether to ask before installing: never on `--yes` or in CI. Pure so the
/// non-interactive rule is pinned by a test rather than by a hanging stdin read.
fn should_prompt(assume_yes: bool, ci: bool) -> bool {
    !assume_yes && !ci
}

/// Detect every tool, print the plan, confirm, then install the missing ones.
/// No tools missing (or none requested) returns immediately without touching
/// stdin — that is what keeps CI and unit tests from hanging.
pub fn provision(tools: &[Tool], assume_yes: bool) -> Result<(), String> {
    let missing: Vec<&Tool> = tools.iter().filter(|t| !t.is_present()).collect();
    if missing.is_empty() {
        return Ok(());
    }
    for t in &missing {
        eprintln!("{}", t.plan_line());
    }
    if should_prompt(assume_yes, std::env::var("CI").is_ok()) {
        eprint!("install the above? [y/N] ");
        let _ = std::io::stderr().flush();
        let mut answer = String::new();
        std::io::stdin()
            .read_line(&mut answer)
            .map_err(|e| format!("cannot read confirmation: {e}"))?;
        let answer = answer.trim().to_ascii_lowercase();
        if answer != "y" && answer != "yes" {
            return Err("installation declined; install the tools above and retry".into());
        }
    }
    for t in &missing {
        let argv = t.install_argv();
        let status = Command::new(&argv[0])
            .args(&argv[1..])
            .status()
            .map_err(|e| format!("cannot run `{}`: {e}", argv.join(" ")))?;
        if !status.success() {
            return Err(format!("`{}` failed ({status})", argv.join(" ")));
        }
    }
    Ok(())
}

fn cargo_subcommand_present(sub: &str) -> bool {
    command_succeeds("cargo", &[sub, "--version"])
}

/// Which of `cargo ios` / `cargo xcode` to invoke (both generate the Xcode
/// project from the `ios/` Swift + Info.plist); returns whichever is installed,
/// preferring `ios`.
pub fn ios_or_xcode_subcommand() -> &'static str {
    if cargo_subcommand_present("ios") {
        "ios"
    } else {
        "xcode"
    }
}

fn command_succeeds(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_ndk_plan_line_names_the_install() {
        assert_eq!(
            Tool::Cargo(CargoTool::Ndk).plan_line(),
            "missing: cargo-ndk -> will run: cargo install cargo-ndk"
        );
    }

    #[test]
    fn rustup_target_plan_line_names_add() {
        assert_eq!(
            Tool::Rustup(RustupTarget("aarch64-linux-android")).plan_line(),
            "missing: rustup target aarch64-linux-android -> will run: rustup target add aarch64-linux-android"
        );
    }

    #[test]
    fn cargo_install_argv_is_exact() {
        assert_eq!(
            Tool::Cargo(CargoTool::Ndk).install_argv(),
            vec!["cargo", "install", "cargo-ndk"]
        );
    }

    #[test]
    fn rustup_target_install_argv_is_exact() {
        assert_eq!(
            Tool::Rustup(RustupTarget("aarch64-linux-android")).install_argv(),
            vec!["rustup", "target", "add", "aarch64-linux-android"]
        );
    }

    #[test]
    fn sdk_hints_name_the_setup_action() {
        assert!(SdkPrecondition::AndroidSdk.hint().contains("ANDROID_HOME"));
        assert!(SdkPrecondition::Jdk.hint().to_lowercase().contains("jdk"));
        assert!(SdkPrecondition::Gradle.hint().contains("gradle"));
        assert!(SdkPrecondition::Xcode.hint().contains("xcode-select"));
        assert!(SdkPrecondition::Hvigor.hint().contains("DEVECO_SDK_HOME"));
        assert!(
            SdkPrecondition::Cmake
                .hint()
                .to_lowercase()
                .contains("cmake")
        );
    }

    #[test]
    fn bogus_rustup_target_is_not_present() {
        assert!(!RustupTarget("__definitely_not_a_real_target__").is_present());
    }

    #[test]
    fn assume_yes_and_ci_skip_the_prompt() {
        assert!(!should_prompt(true, false));
        assert!(!should_prompt(false, true));
        assert!(should_prompt(false, false));
    }

    #[test]
    fn provisioning_nothing_does_not_read_stdin() {
        // No missing tools => no prompt, no install, no hang on stdin.
        assert!(provision(&[], false).is_ok());
    }
}
