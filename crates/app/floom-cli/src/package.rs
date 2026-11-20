// SPDX-License-Identifier: MIT
//! `floom package` orchestration.
//!
//! Builds the host release binary, resolves the project via `cargo metadata`,
//! then hands a requested platform to `pack::pack`. Per-platform artifacts are
//! opt-in via `--platform`; packaging is self-contained (no Floom checkout /
//! `scripts/build.sh` needed).

use std::path::Path;

use crate::pack;

/// Flags that belong to `package` only. `parse_lifecycle` (shared with
/// `check`/`test`) must never see them, so they are split out first.
#[derive(Default)]
struct PackageFlags {
    abi: Option<String>,
    install: bool,
    device: bool,
    yes: bool,
}

/// Split `package` args into the shared lifecycle options and the
/// package-only flags, then parse the shared ones with `parse_lifecycle`.
fn parse_package(args: &[String]) -> Result<(crate::LifecycleOpts, PackageFlags), String> {
    let mut shared: Vec<String> = Vec::new();
    let mut flags = PackageFlags::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--abi" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "`--abi` requires a value".to_string())?
                    .clone();
                flags.abi = Some(v);
                i += 1;
            }
            "--install" => flags.install = true,
            "--device" => flags.device = true,
            "--yes" => flags.yes = true,
            other => shared.push(other.to_string()),
        }
        i += 1;
    }
    let lc = crate::parse_lifecycle(&shared)?;
    Ok((lc, flags))
}

/// `floom package <DIR> [flags]`.
pub fn run(args: &[String]) -> Result<(), String> {
    let (lc, flags) = parse_package(args)?;

    // The host build is always release (matches the previous `package`).
    crate::cargo_in(&lc.dir, "build", &lc.features, true)?;

    let manifest = lc.dir.join("Cargo.toml");
    if !manifest.is_file() {
        return Err(format!("{} has no Cargo.toml", lc.dir.display()));
    }
    let project = crate::pack::shared::inspect(&lc.dir)?;

    let Some(platform) = lc.platform.clone() else {
        println!(
            "built release binary for `{}` (host).",
            project.package_name
        );
        println!(
            "  for a platform artifact, run `floom package --platform <macos|windows|linux|android|ios|ohos|web>`"
        );
        return Ok(());
    };

    // Explicit `--features` wins; else derive `<platform>` → `real` → none,
    // exactly as `scripts/build.sh` did.
    let features = if lc.features.is_empty() {
        crate::pack::shared::default_features(&project.features, &platform)
    } else {
        lc.features.clone()
    };

    let opts = pack::PackOpts {
        release: lc.release,
        features,
        abi: flags.abi,
        install: flags.install,
        device: flags.device,
        yes: flags.yes,
    };

    let result = pack::pack(&platform, &project, &opts)?;
    if let Some(note) = &result.note {
        eprintln!("warning: {note}");
    }
    println!("{}", tier_line(&platform, &project.package_name, &result));
    Ok(())
}

/// The single result line `scripts/build.sh` printed, kept verbatim so tooling
/// that parsed it keeps working. Pure so its format is pinned by a test.
fn tier_line(platform: &str, crate_name: &str, result: &pack::PackResult) -> String {
    let tier = match result.tier {
        pack::Tier::Artifact => "artifact",
        pack::Tier::CheckOnly => "check-only",
        pack::Tier::Blocked => "blocked",
    };
    let artifact = result
        .artifact
        .as_deref()
        .map(|p: &Path| p.display().to_string())
        .unwrap_or_else(|| "-".to_string());
    format!("TIER={tier} PLATFORM={platform} CRATE={crate_name} ARTIFACT={artifact}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_flags_split_off_before_lifecycle_parsing() {
        let (lc, flags) = parse_package(&[
            "my_app".into(),
            "--platform".into(),
            "android".into(),
            "--abi".into(),
            "arm64-v8a".into(),
            "--device".into(),
            "--yes".into(),
        ])
        .unwrap();
        assert_eq!(lc.platform.as_deref(), Some("android"));
        assert_eq!(lc.dir, std::path::Path::new("my_app"));
        assert_eq!(flags.abi.as_deref(), Some("arm64-v8a"));
        assert!(flags.device);
        assert!(flags.yes);
        assert!(!flags.install);
    }

    #[test]
    fn unknown_flags_are_still_rejected() {
        // `check`/`test` share parse_lifecycle; --abi must not leak into them.
        assert!(crate::parse_lifecycle(&["--abi".into(), "x".into()]).is_err());
        assert!(parse_package(&["--bogus".into()]).is_err());
    }

    #[test]
    fn tier_line_matches_build_sh_contract() {
        let artifact = pack::PackResult {
            tier: pack::Tier::Artifact,
            artifact: Some(std::path::PathBuf::from("/r/target/app.apk")),
            note: Some("HAP is unsigned".into()),
        };
        assert_eq!(
            tier_line("android", "demo", &artifact),
            "TIER=artifact PLATFORM=android CRATE=demo ARTIFACT=/r/target/app.apk"
        );
        let blocked = pack::PackResult {
            tier: pack::Tier::Blocked,
            artifact: None,
            note: Some("no SDK".into()),
        };
        assert_eq!(
            tier_line("ios", "demo", &blocked),
            "TIER=blocked PLATFORM=ios CRATE=demo ARTIFACT=-"
        );
        let check = pack::PackResult {
            tier: pack::Tier::CheckOnly,
            artifact: Some(std::path::PathBuf::from("/r/a.wasm")),
            note: None,
        };
        assert_eq!(
            tier_line("web", "demo", &check),
            "TIER=check-only PLATFORM=web CRATE=demo ARTIFACT=/r/a.wasm"
        );
    }
}
