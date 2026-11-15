// SPDX-License-Identifier: MIT
//! iOS packaging: `cargo ios`/`cargo xcode` → `.xcodeproj`, then `xcodebuild`.
//!
//! `floom create` emits an Xcode/Swift project under `ios/` with no `.xcodeproj`
//! — that is exactly what `cargo-ios`/`cargo-xcode` generates from the
//! `Info.plist` + Swift sources. Packaging drives that project instead of
//! hand-assembling an `.app` like the old `ios-bundle.sh`.

use std::path::Path;

use crate::pack::shared::{self, Project};
use crate::pack::{PackOpts, PackResult, Tier};
use crate::toolchain::{CargoTool, RustupTarget, SdkPrecondition, Tool};

/// The rustup target and xcodebuild configuration an iOS build uses. Simulator
/// is the default; `--device` selects the real-device target (which needs a
/// signing identity the CLI cannot provision).
pub struct IosPlan {
    pub rustup_target: &'static str,
    pub configuration: &'static str,
    pub device: bool,
}

/// Pure resolution of the build's rustup target + configuration from options.
pub fn ios_plan(opts: &PackOpts) -> IosPlan {
    IosPlan {
        rustup_target: if opts.device {
            "aarch64-apple-ios"
        } else {
            "aarch64-apple-ios-sim"
        },
        configuration: if opts.release { "Release" } else { "Debug" },
        device: opts.device,
    }
}

/// The `xcodebuild` argv. The scheme is the crate name (what cargo-ios/Xcode
/// generates), and the derived data tree is kept under `ios/build`.
pub fn xcodebuild_argv(project: &Project, opts: &PackOpts) -> Vec<String> {
    let plan = ios_plan(opts);
    vec![
        "xcodebuild".to_string(),
        "-scheme".to_string(),
        project.package_name.clone(),
        "-configuration".to_string(),
        plan.configuration.to_string(),
        "-derivedDataPath".to_string(),
        project
            .root
            .join("ios/build")
            .to_string_lossy()
            .into_owned(),
        "build".to_string(),
    ]
}

/// Package the iOS scaffold. Provisions `cargo-ios`/`cargo-xcode` + the rustup
/// target, requires Xcode, generates the project, builds it, and — for the
/// simulator — ad-hoc codesigns the `.app`. A device build is reported as
/// `blocked` because installing it needs a real signing identity.
pub fn pack_ios(project: &Project, opts: &PackOpts) -> Result<PackResult, String> {
    let plan = ios_plan(opts);
    crate::toolchain::provision(
        &[
            Tool::Cargo(CargoTool::CargoIosOrXcode),
            Tool::Rustup(RustupTarget(plan.rustup_target)),
        ],
        opts.yes,
    )?;
    if let Err(blocked) = shared::require_sdks_blocked(&[SdkPrecondition::Xcode]) {
        return Ok(blocked);
    }

    let sub = crate::toolchain::ios_or_xcode_subcommand();
    let ios_dir = project.root.join("ios");
    shared::run("cargo", &[sub], &ios_dir)?;

    let xb = xcodebuild_argv(project, opts);
    let xb_ref: Vec<&str> = xb.iter().map(String::as_str).collect();
    shared::run("xcodebuild", &xb_ref, &ios_dir)?;

    let app = shared::newest_artifact(&ios_dir, "app")
        .ok_or_else(|| "xcodebuild produced no .app".to_string())?;

    if plan.device {
        Ok(PackResult {
            tier: Tier::Blocked,
            artifact: Some(app),
            note: Some(
                "unsigned: a real signing identity is required to install on a device".into(),
            ),
        })
    } else {
        let app_str = app.to_string_lossy().into_owned();
        shared::run(
            "codesign",
            &["--force", "--deep", "--sign", "-", &app_str],
            &ios_dir,
        )?;
        Ok(PackResult {
            tier: Tier::Artifact,
            artifact: Some(app),
            note: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::PackOpts;
    use std::path::PathBuf;

    fn opts(device: bool, release: bool) -> PackOpts {
        PackOpts {
            release,
            features: vec![],
            abi: None,
            manifest: None,
            install: false,
            device,
            bin: None,
            yes: false,
            dir: PathBuf::from("/p"),
        }
    }

    fn project() -> Project {
        Project {
            root: PathBuf::from("/p"),
            target_dir: PathBuf::from("/p/target"),
            package_name: "demo".into(),
            features: vec!["real".into()],
        }
    }

    #[test]
    fn simulator_is_the_default_target_release_builds() {
        let plan = ios_plan(&opts(false, true));
        assert_eq!(plan.rustup_target, "aarch64-apple-ios-sim");
        assert_eq!(plan.configuration, "Release");
        assert!(!plan.device);
    }

    #[test]
    fn device_selects_the_real_target() {
        let plan = ios_plan(&opts(true, false));
        assert_eq!(plan.rustup_target, "aarch64-apple-ios");
        assert_eq!(plan.configuration, "Debug");
        assert!(plan.device);
    }

    #[test]
    fn xcodebuild_argv_uses_scheme_and_derived_data() {
        let argv = xcodebuild_argv(&project(), &opts(false, true));
        assert_eq!(argv[0], "xcodebuild");
        assert_eq!(argv[1], "-scheme");
        assert_eq!(argv[2], "demo");
        assert_eq!(argv[3], "-configuration");
        assert_eq!(argv[4], "Release");
        assert_eq!(argv[5], "-derivedDataPath");
        assert_eq!(argv[6], "/p/ios/build");
        assert_eq!(argv[7], "build");
    }
}
