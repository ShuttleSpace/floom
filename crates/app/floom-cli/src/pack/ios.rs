// SPDX-License-Identifier: MIT
//! iOS packaging: the Rust static library, linked into a `.app` by `swiftc`.
//!
//! No `.xcodeproj` and no `cargo-xcode`: the host is two Swift files and an
//! `Info.plist` under `ios/`, and `swiftc` links them against the static
//! library directly. A generated project file would be one more artifact to
//! keep in step with the SDK, and the four commands it replaces are the ones
//! this module already has to run by hand to test them.

use crate::pack::shared::{self, Project};
use crate::pack::{PackOpts, PackResult, Tier};
use crate::toolchain::{RustupTarget, SdkPrecondition, Tool};

use std::path::PathBuf;

/// The rustup target and SDK a build uses. Simulator is the default; `--device`
/// selects the real-device target (which needs a signing identity the CLI
/// cannot provision).
pub struct IosPlan {
    pub rustup_target: &'static str,
    pub sdk: &'static str,
    pub device: bool,
}

/// How the app binary is built from the static library.
pub struct Link {
    pub sdk_path: String,
    pub target: String,
    pub sources: Vec<String>,
    pub library_dir: String,
    pub crate_name: String,
    pub out: String,
}

/// Pure resolution of the build's rustup target + SDK from options.
pub fn ios_plan(opts: &PackOpts) -> IosPlan {
    IosPlan {
        rustup_target: if opts.device {
            "aarch64-apple-ios"
        } else {
            "aarch64-apple-ios-sim"
        },
        sdk: if opts.device {
            "iphoneos"
        } else {
            "iphonesimulator"
        },
        device: opts.device,
    }
}

/// The `swiftc` argv that links the static library into an app binary.
///
/// `-parse-as-library` because the host uses `@main` rather than a `main.swift`
/// top level: without it `swiftc` looks for a top-level entry point, finds none,
/// and reports "expressions are not allowed at the top level" instead.
pub fn swiftc_argv(link: &Link) -> Vec<String> {
    let mut argv = vec![
        "swiftc".to_string(),
        "-sdk".to_string(),
        link.sdk_path.clone(),
        "-target".to_string(),
        link.target.clone(),
        "-parse-as-library".to_string(),
        "-Onone".to_string(),
        "-g".to_string(),
    ];
    argv.extend(link.sources.iter().cloned());
    argv.extend([
        "-L".to_string(),
        link.library_dir.clone(),
        format!("-l{}", link.crate_name),
        "-framework".to_string(),
        "UIKit".to_string(),
        "-framework".to_string(),
        "QuartzCore".to_string(),
        "-o".to_string(),
        link.out.clone(),
    ]);
    argv
}

/// Every `.swift` file under `ios/src`, sorted so the argv is stable.
fn swift_sources(project: &Project) -> Vec<String> {
    let dir = project.root.join("ios/src");
    let mut sources: Vec<String> = std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "swift"))
                .map(shared::arg)
                .collect()
        })
        .unwrap_or_default();
    sources.sort();
    sources
}

/// The app bundle the build writes into: `ios/build/<name>.app`.
fn app_bundle(project: &Project) -> PathBuf {
    project
        .root
        .join("ios/build")
        .join(format!("{}.app", project.package_name))
}

/// Package the iOS scaffold. Builds the static library for the target, links
/// it with macc `swiftc`, and for the simulator ad-hoc signs the bundle. A
/// device build is reported as `blocked`: installing it needs a signing
/// identity this CLI cannot provision.
pub fn pack_ios(project: &Project, opts: &PackOpts) -> Result<PackResult, String> {
    let plan = ios_plan(opts);
    crate::toolchain::provision(&[Tool::Rustup(RustupTarget(plan.rustup_target))], opts.yes)?;
    if let Err(blocked) = shared::require_sdks_blocked(&[SdkPrecondition::Xcode]) {
        return Ok(blocked);
    }

    let root = project.root.clone();
    let mut cargo_argv: Vec<String> =
        vec!["build".into(), "--target".into(), plan.rustup_target.into()];
    if opts.release {
        cargo_argv.push("--release".into());
    }
    shared::append_features(&mut cargo_argv, &opts.features);
    let cargo_ref: Vec<&str> = cargo_argv.iter().map(String::as_str).collect();
    shared::run("cargo", &cargo_ref, &root)?;

    let library_dir = project
        .target_dir
        .join(plan.rustup_target)
        .join(if opts.release { "release" } else { "debug" });
    let app = app_bundle(project);
    std::fs::create_dir_all(&app)
        .map_err(|e| format!("cannot create {}: {e}", shared::arg(&app)))?;
    std::fs::copy(root.join("ios/Info.plist"), app.join("Info.plist"))
        .map_err(|e| format!("cannot write the bundle manifest: {e}"))?;

    let sdk_path = shared::capture("xcrun", &["--sdk", plan.sdk, "--show-sdk-path"], &root)?;
    let version = shared::capture("xcrun", &["--sdk", plan.sdk, "--show-sdk-version"], &root)?;
    let target = if plan.device {
        format!("arm64-apple-ios{version}")
    } else {
        format!("arm64-apple-ios{version}-simulator")
    };
    let link = Link {
        sdk_path,
        target,
        sources: swift_sources(project),
        library_dir: shared::arg(&library_dir),
        crate_name: project.package_name.replace('-', "_"),
        out: shared::arg(app.join(&project.package_name)),
    };
    if link.sources.is_empty() {
        return Err(
            "ios/src holds no Swift sources; run `floom create` to restore the host".into(),
        );
    }
    let argv = swiftc_argv(&link);
    // `swiftc_argv` names the program first (so tests read like the command);
    // `shared::run` takes it separately.
    let args: Vec<&str> = argv[1..].iter().map(String::as_str).collect();
    shared::run("swiftc", &args, &root)?;

    if plan.device {
        return Ok(PackResult {
            tier: Tier::Blocked,
            artifact: Some(app),
            note: Some(
                "unsigned: a real signing identity is required to install on a device".into(),
            ),
        });
    }
    let app_str = shared::arg(&app);
    shared::run("codesign", &["--force", "--sign", "-", &app_str], &root)?;
    Ok(PackResult {
        tier: Tier::Artifact,
        artifact: Some(app),
        note: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::PackOpts;
    use std::path::{Path, PathBuf};

    fn opts(device: bool, release: bool) -> PackOpts {
        PackOpts {
            release,
            features: vec![],
            abi: None,
            install: false,
            device,
            yes: false,
        }
    }

    fn project() -> Project {
        Project {
            root: PathBuf::from("/p"),
            target_dir: PathBuf::from("/p/target"),
            package_name: "my-app".into(),
            features: vec!["real".into()],
        }
    }

    fn link() -> Link {
        Link {
            sdk_path: "/sdk".into(),
            target: "arm64-apple-ios27.2-simulator".into(),
            sources: vec!["/p/ios/src/AppDelegate.swift".into()],
            library_dir: "/p/target/aarch64-apple-ios-sim/debug".into(),
            crate_name: "my_app".into(),
            out: "/p/ios/build/my-app.app/my-app".into(),
        }
    }

    #[test]
    fn simulator_is_the_default_target() {
        let plan = ios_plan(&opts(false, true));
        assert_eq!(plan.rustup_target, "aarch64-apple-ios-sim");
        assert_eq!(plan.sdk, "iphonesimulator");
        assert!(!plan.device);
    }

    #[test]
    fn device_selects_the_real_target_and_sdk() {
        let plan = ios_plan(&opts(true, false));
        assert_eq!(plan.rustup_target, "aarch64-apple-ios");
        assert_eq!(plan.sdk, "iphoneos");
        assert!(plan.device);
    }

    #[test]
    fn swiftc_links_the_static_library_into_the_bundle() {
        let argv = swiftc_argv(&link());
        assert_eq!(argv[0], "swiftc");
        assert!(argv.contains(&"-parse-as-library".to_string()));
        assert!(argv.contains(&"/p/ios/src/AppDelegate.swift".to_string()));
        assert!(argv.contains(&"-L".to_string()));
        assert!(argv.contains(&"/p/target/aarch64-apple-ios-sim/debug".to_string()));
        // The crate name is the package name with `-` folded to `_`: that is
        // what cargo names `libmy_app.a`.
        assert!(argv.contains(&"-lmy_app".to_string()));
        assert!(argv.contains(&"UIKit".to_string()));
        assert_eq!(
            argv.last().map(String::as_str),
            Some("/p/ios/build/my-app.app/my-app")
        );
    }

    #[test]
    fn a_missing_swift_source_directory_is_reported_not_ignored() {
        let empty = Project {
            root: PathBuf::from("/definitely/not/here"),
            target_dir: PathBuf::from("/definitely/not/here/target"),
            package_name: "demo".into(),
            features: vec![],
        };
        assert!(swift_sources(&empty).is_empty());
        assert_eq!(
            app_bundle(&empty),
            Path::new("/definitely/not/here/ios/build/demo.app")
        );
    }
}
