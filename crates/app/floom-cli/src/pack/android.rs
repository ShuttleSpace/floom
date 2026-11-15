// SPDX-License-Identifier: MIT
//! Android packaging: `cargo ndk` → `.so`, then Gradle `assembleRelease`.
//!
//! `floom create` emits an Android **Gradle** project (manifest with resources,
//! Kotlin `MainActivity`), so packaging drives Gradle/AGP rather than porting
//! `scripts/build-apk.sh`'s aapt2/d8 pipeline. `floom-cli` contributes the
//! toolchain provisioning (`cargo-ndk`, the rustup target) and orchestration.

use std::path::Path;

use crate::pack::shared::{self, Project};
use crate::pack::{PackOpts, PackResult, Tier};
use crate::toolchain::{CargoTool, RustupTarget, SdkPrecondition, Tool};

/// Default NDK ABI when the user passes none. `arm64-v8a` covers modern phones.
const DEFAULT_ABI: &str = "arm64-v8a";

/// The `adb install` argv for a built APK (`-r` keeps the data of an existing
/// install). Pure so the command is pinned without touching a device.
pub fn adb_install_argv(apk: &Path) -> Vec<String> {
    vec!["install".to_string(), "-r".to_string(), apk.to_string_lossy().into_owned()]
}

/// The exact `cargo ndk` argv. Pure so its flags are pinned without a real
/// build. `-P 24` is the minimum SDK (matches the create scaffold); the `.so`
/// lands in the Gradle project's `jniLibs` dir where AGP picks it up.
pub fn ndk_argv(project: &Project, opts: &PackOpts) -> Vec<String> {
    let abi = opts.abi.clone().unwrap_or_else(|| DEFAULT_ABI.to_string());
    let out = project
        .root
        .join("android/app/src/main/jniLibs")
        .to_string_lossy()
        .into_owned();
    let mut argv = vec![
        "ndk".to_string(),
        "-t".to_string(),
        abi,
        "-P".to_string(),
        "24".to_string(),
        "-o".to_string(),
        out,
        "build".to_string(),
    ];
    if opts.release {
        argv.push("--release".to_string());
    }
    shared::append_features(&mut argv, &opts.features);
    argv
}

/// Which Gradle to invoke and with what task. `gradlew` wins when the scaffold
/// shipped one; otherwise `gradle` from PATH (the create scaffold currently has
/// only `gradle-wrapper.properties`, so this is the common path).
pub fn gradle_argv(project: &Project, opts: &PackOpts) -> (String, Vec<String>) {
    let android_dir = project.root.join("android");
    let gradle = if android_dir.join("gradlew").exists() {
        "./gradlew".to_string()
    } else {
        "gradle".to_string()
    };
    let task = if opts.release {
        "assembleRelease"
    } else {
        "assembleDebug"
    }
    .to_string();
    (gradle, vec![task])
}

/// Package the Android scaffold: provision `cargo-ndk` + the rustup target,
/// require the SDKs, run `cargo ndk`, then Gradle. Returns the produced APK.
pub fn pack_android(project: &Project, opts: &PackOpts) -> Result<PackResult, String> {
    crate::toolchain::provision(
        &[
            Tool::Cargo(CargoTool::Ndk),
            Tool::Rustup(RustupTarget("aarch64-linux-android")),
        ],
        opts.yes,
    )?;

    // SDKs cannot be auto-installed; a missing one is a `blocked` tier, not a
    // hard error.
    if let Err(blocked) = shared::require_sdks_blocked(&[
        SdkPrecondition::AndroidSdk,
        SdkPrecondition::Jdk,
        SdkPrecondition::Gradle,
    ]) {
        return Ok(blocked);
    }

    let ndk = ndk_argv(project, opts);
    let ndk_ref: Vec<&str> = ndk.iter().map(String::as_str).collect();
    shared::run("cargo", &ndk_ref, &project.root)?;

    let jni_libs = project.root.join("android/app/src/main/jniLibs");
    if shared::newest_artifact(&jni_libs, "so").is_none() {
        return Err(format!(
            "cargo ndk produced no .so under {}",
            jni_libs.display()
        ));
    }

    let (gradle, gargs) = gradle_argv(project, opts);
    let android_dir = project.root.join("android");
    let gradle_ref = gradle.as_str();
    let gargs_ref: Vec<&str> = gargs.iter().map(String::as_str).collect();
    shared::run(gradle_ref, &gargs_ref, &android_dir)?;

    let apk = shared::newest_artifact(&android_dir, "apk")
        .ok_or_else(|| "Gradle produced no .apk".to_string())?;

    if opts.install {
        // `adb` ships with the Android SDK platform-tools; if it is missing the
        // install fails with a clear error rather than silently skipping.
        let adb = adb_install_argv(&apk);
        let adb_ref: Vec<&str> = adb.iter().map(String::as_str).collect();
        shared::run("adb", &adb_ref, &android_dir)?;
    }

    Ok(PackResult {
        tier: Tier::Artifact,
        artifact: Some(apk),
        note: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::PackOpts;
    use std::path::PathBuf;

    fn opts_with(abi: Option<&str>, release: bool) -> PackOpts {
        PackOpts {
            release,
            features: vec![],
            abi: abi.map(str::to_string),
            install: false,
            device: false,
            yes: false,
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
    fn ndk_argv_uses_default_abi_and_release_flag() {
        let argv = ndk_argv(&project(), &opts_with(None, true));
        assert_eq!(
            argv,
            vec![
                "ndk",
                "-t",
                "arm64-v8a",
                "-P",
                "24",
                "-o",
                "/p/android/app/src/main/jniLibs",
                "build",
                "--release",
            ]
        );
    }

    #[test]
    fn ndk_argv_honors_user_abi_and_omits_release_when_debug() {
        let argv = ndk_argv(&project(), &opts_with(Some("x86_64"), false));
        assert_eq!(argv[2], "x86_64");
        assert!(!argv.contains(&"--release".to_string()));
    }

    #[test]
    fn gradle_release_and_debug_tasks() {
        // No gradlew shipped in the test path, so the PATH `gradle` is chosen.
        let release = gradle_argv(&project(), &opts_with(None, true));
        assert_eq!(release.0, "gradle");
        assert_eq!(release.1, vec!["assembleRelease".to_string()]);
        let debug = gradle_argv(&project(), &opts_with(None, false));
        assert_eq!(debug.1, vec!["assembleDebug".to_string()]);
    }

    #[test]
    fn adb_install_argv_uses_replace_flag() {
        let apk = Path::new("/p/android/app-release.apk");
        assert_eq!(
            adb_install_argv(apk),
            vec!["install", "-r", "/p/android/app-release.apk"]
        );
    }
}
