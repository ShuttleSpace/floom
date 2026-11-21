// SPDX-License-Identifier: MIT
//! `floom doctor` — what this machine can build, before you find out the hard
//! way.
//!
//! Every row answers one question: can `floom package --platform X` even start?
//! That is decided by the target being installed, nothing more. SDKs, device
//! trust and linker settings are deliberately out of scope: checking them
//! needs each platform's own toolchain to be present, and a "ready" that is
//! occasionally wrong is worse than one that says what it measured.
//!
//! The CLI is dependency-free by design, so the probes here are `rustc -vV`,
//! `rustup target list --installed` and two small text scans.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The target each platform's `floom package` needs.
const PLATFORMS: [(&str, &str); 5] = [
    ("web", "wasm32-unknown-unknown"),
    ("android", "aarch64-linux-android"),
    ("ios", "aarch64-apple-ios"),
    ("ohos", "aarch64-unknown-linux-ohos"),
    ("windows", "x86_64-pc-windows-msvc"),
];

pub fn run(args: &[String]) -> Result<(), String> {
    let mut dir = PathBuf::from(".");
    let mut wanted: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--platform" => {
                i += 1;
                wanted.push(
                    args.get(i)
                        .ok_or("`--platform` requires a value")?
                        .to_string(),
                );
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unknown option `{flag}` for `doctor`"));
            }
            path => {
                if dir != Path::new(".") {
                    return Err("`floom doctor` takes one directory argument".into());
                }
                dir = PathBuf::from(path);
            }
        }
        i += 1;
    }

    let machine = Machine::probe();
    let project = read_project(&dir);
    let (text, blocked) = report(&machine, project.as_ref(), &dir, &wanted);
    print!("{text}");
    if blocked.is_empty() {
        return Ok(());
    }
    Err(format!(
        "not buildable here: {} — install the target(s) named above",
        blocked.join(", ")
    ))
}

/// Everything the report is built from, so the report can be tested without a
/// machine that happens to be missing a target.
struct Machine {
    rustc: Option<Rustc>,
    cargo: Option<String>,
    targets: Vec<String>,
}

impl Machine {
    fn probe() -> Self {
        Self {
            rustc: rustc_version(),
            cargo: cargo_version(),
            targets: installed_targets(),
        }
    }

    fn host(&self) -> String {
        self.rustc
            .as_ref()
            .and_then(|v| v.host.clone())
            .unwrap_or_else(|| "unknown".into())
    }
}

/// The report, and the requested platforms it says are not buildable here.
fn report(
    machine: &Machine,
    project: Option<&Project>,
    dir: &Path,
    wanted: &[String],
) -> (String, Vec<&'static str>) {
    let host = machine.host();
    let mut text = String::from("floom doctor — what this machine can build\n\n");
    match &machine.rustc {
        Some(v) => text.push_str(&format!("  rustc        {} ({})\n", v.release, v.date)),
        None => text.push_str("  rustc        not on PATH\n"),
    }
    match &machine.cargo {
        Some(version) => text.push_str(&format!("  cargo        {version}\n")),
        None => text.push_str("  cargo        not on PATH\n"),
    }
    text.push_str(&format!("  host         {host}\n"));
    match project {
        Some(p) => match &p.floom {
            FloomDep::Versioned(version) => text.push_str(&format!(
                "  project      {} (floom {version})\n",
                p.manifest.display()
            )),
            FloomDep::Unversioned => text.push_str(&format!(
                "  project      {} (floom from a path)\n",
                p.manifest.display()
            )),
            FloomDep::Absent => text.push_str(&format!(
                "  project      {} (no floom dependency)\n",
                p.manifest.display()
            )),
        },
        None => text.push_str(&format!(
            "  project      {} (no Cargo.toml)\n",
            dir.display()
        )),
    }
    text.push('\n');

    let mut blocked = Vec::new();
    for (platform, row) in rows(&host, &machine.targets) {
        match row {
            Ok(()) => text.push_str(&format!("  {platform:12} ready\n")),
            Err(reason) => {
                text.push_str(&format!("  {platform:12} missing     {reason}\n"));
                if wanted.iter().any(|w| w == platform) {
                    blocked.push(platform);
                }
            }
        }
    }
    text.push_str(
        "\n  desktop and tui build with the host target; the rows above are about\n  \
         targets only, not about SDKs, device trust or linker settings.\n",
    );
    (text, blocked)
}

/// One row per platform: `Ok` when the target is installed, `Err` with what to
/// run when it is not.
fn rows(host: &str, targets: &[String]) -> Vec<(&'static str, Result<(), String>)> {
    let has = |t: &str| targets.iter().any(|i| i == t);
    let mut out: Vec<(&'static str, Result<(), String>)> = Vec::new();
    for platform in ["desktop", "tui"] {
        out.push((platform, install_hint(host, has(host))));
    }
    for (platform, target) in PLATFORMS {
        out.push((platform, install_hint(target, has(target))));
    }
    out
}

fn install_hint(target: &str, installed: bool) -> Result<(), String> {
    if installed {
        Ok(())
    } else {
        Err(format!("rustup target add {target}"))
    }
}

struct Project {
    manifest: PathBuf,
    floom: FloomDep,
}

/// How the project depends on floom. A path dependency has no version to print,
/// but it is still a dependency — reporting it as absent was wrong.
#[derive(Debug, PartialEq)]
enum FloomDep {
    Absent,
    Unversioned,
    Versioned(String),
}

/// The project's `Cargo.toml`, plus the `floom` version its `Cargo.lock` pinned.
///
/// Read as text rather than parsed: a doctor that refuses to run because the
/// manifest uses a syntax the CLI's hand-rolled reader does not know would be
/// useless exactly when it is needed.
fn read_project(dir: &Path) -> Option<Project> {
    let manifest = dir.join("Cargo.toml");
    if !manifest.is_file() {
        return None;
    }
    let text = std::fs::read_to_string(&manifest).unwrap_or_default();
    let declared = declared_floom(&text);
    let pinned = dir
        .join("Cargo.lock")
        .is_file()
        .then(|| pinned_floom(&std::fs::read_to_string(dir.join("Cargo.lock")).unwrap_or_default()))
        .flatten();
    let floom = match (declared, pinned) {
        (FloomDep::Absent, _) => FloomDep::Absent,
        (_, Some(version)) => FloomDep::Versioned(version),
        (declared, None) => declared,
    };
    Some(Project { manifest, floom })
}

/// `version = "0.2.0"` from the `name = "floom"` block of a `Cargo.lock`.
fn pinned_floom(lock: &str) -> Option<String> {
    let mut name_matched = false;
    for line in lock.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("name = ") {
            name_matched = rest.trim_matches('"') == "floom";
        } else if name_matched && let Some(rest) = line.strip_prefix("version = ") {
            return Some(rest.trim_matches('"').to_string());
        }
    }
    None
}

/// The requirement from `floom = { version = … }` / `floom = "…"`.
fn declared_floom(manifest: &str) -> FloomDep {
    for line in manifest.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("floom") else {
            continue;
        };
        let rest = rest.trim_start();
        if let Some(spec) = rest.strip_prefix('=') {
            let spec = spec.trim();
            if let Some(start) = spec.find("version") {
                let after = &spec[start..];
                if let Some(open) = after.find('"')
                    && let Some(close) = after[open + 1..].find('"')
                {
                    return FloomDep::Versioned(after[open + 1..open + 1 + close].to_string());
                }
            }
            // A bare string is a version requirement; a table without `version`
            // is a path dependency, which is a dependency with no version.
            return match spec.strip_prefix('"').and_then(|r| r.split('"').next()) {
                Some(version) => FloomDep::Versioned(version.to_string()),
                None => FloomDep::Unversioned,
            };
        }
    }
    FloomDep::Absent
}

struct Rustc {
    release: String,
    date: String,
    host: Option<String>,
}

fn rustc_version() -> Option<Rustc> {
    let out = Command::new("rustc").arg("-vV").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let field = |key: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(key).map(|v| v.trim().to_string()))
    };
    Some(Rustc {
        release: field("release:")?,
        date: field("commit-date:").unwrap_or_else(|| "unknown date".into()),
        host: field("host:"),
    })
}

fn cargo_version() -> Option<String> {
    let out = Command::new("cargo").arg("--version").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    Some(text.trim().trim_start_matches("cargo ").to_string())
}

fn installed_targets() -> Vec<String> {
    let Ok(out) = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn targets(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn an_installed_target_is_the_only_thing_a_row_reads() {
        let rows = rows(
            "aarch64-apple-darwin",
            &targets(&["aarch64-apple-darwin", "wasm32-unknown-unknown"]),
        );
        let by_name = |name: &str| {
            rows.iter()
                .find(|(p, _)| *p == name)
                .map(|(_, r)| r.clone())
                .expect("the row exists")
        };
        assert_eq!(by_name("desktop"), Ok(()));
        assert_eq!(by_name("web"), Ok(()));
        assert_eq!(
            by_name("ios"),
            Err("rustup target add aarch64-apple-ios".to_string())
        );
    }

    /// The report is a pure function of the machine, so the "missing" branch is
    /// reachable in a test even on a machine where every target is installed.
    #[test]
    fn a_requested_platform_whose_target_is_absent_is_named() {
        let machine = Machine {
            rustc: Some(Rustc {
                release: "1.91.0".into(),
                date: "2026-01-01".into(),
                host: Some("aarch64-apple-darwin".into()),
            }),
            cargo: Some("1.91.0".into()),
            targets: targets(&["aarch64-apple-darwin"]),
        };
        let (text, blocked) = report(&machine, None, Path::new("."), &["ios".to_string()]);
        assert_eq!(blocked, vec!["ios"]);
        assert!(
            text.contains("ios          missing     rustup target add aarch64-apple-ios"),
            "{text}"
        );
        assert!(text.contains("desktop      ready"), "{text}");

        // Not asked for, so it is reported and does not fail the command.
        let (_, blocked) = report(&machine, None, Path::new("."), &[]);
        assert!(blocked.is_empty());
    }

    #[test]
    fn a_path_dependency_is_a_dependency_without_a_version() {
        assert_eq!(
            declared_floom("floom = { path = \"../floom\" }"),
            FloomDep::Unversioned
        );
        assert_eq!(
            declared_floom("floom = { version = \"0.2\", features = [\"desktop\"] }"),
            FloomDep::Versioned("0.2".to_string())
        );
        assert_eq!(
            declared_floom("floom = \"0.2.0\""),
            FloomDep::Versioned("0.2.0".to_string())
        );
        assert_eq!(
            declared_floom("[package]\nname = \"demo\"\n"),
            FloomDep::Absent
        );
        assert_eq!(declared_floom("floomify = \"1\"\n"), FloomDep::Absent);
    }

    #[test]
    fn the_lock_file_names_the_version_that_is_actually_built() {
        let lock = r#"
[[package]]
name = "floom"
version = "0.2.0"

[[package]]
name = "floom-cli"
version = "0.2.0"
"#;
        assert_eq!(pinned_floom(lock), Some("0.2.0".to_string()));
        assert_eq!(pinned_floom(""), None);
    }
}
