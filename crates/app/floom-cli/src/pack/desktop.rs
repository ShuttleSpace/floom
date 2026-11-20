// SPDX-License-Identifier: MIT
//! Desktop packaging: drive the CMake project for `windows`/`linux`/`macos`.
//!
//! `floom create` emits a CMake project under `<dir>/<platform>`; packaging
//! configures and builds it with `cmake`. CMake is a required SDK (detected,
//! never auto-installed). No extra rustup target is needed — CMake builds the
//! host binary via cargo.

use std::path::Path;

use crate::pack::shared::{self, Project};
use crate::pack::{PackOpts, PackResult, Tier};
use crate::toolchain::SdkPrecondition;

/// The `cmake -B <build>` configure argv. The build tree lives under the
/// platform dir so each platform stays isolated.
pub fn cmake_configure_argv(platform_dir: &Path) -> Vec<String> {
    vec!["-B".to_string(), shared::arg(platform_dir.join("build"))]
}

/// The `cmake --build <build>` argv. On multi-config generators (Visual Studio,
/// Xcode) the configuration is passed with `--config`.
pub fn cmake_build_argv(platform_dir: &Path, opts: &PackOpts) -> Vec<String> {
    let mut argv = vec![
        "--build".to_string(),
        shared::arg(platform_dir.join("build")),
    ];
    if opts.release {
        argv.push("--config".to_string());
        argv.push("Release".to_string());
    }
    argv
}

/// Recursively find the built binary: the file whose stem equals the package
/// name (covers `<name>`, `<name>.exe`, `<name>.app`).
fn find_artifact(build: &Path, name: &str) -> Option<std::path::PathBuf> {
    let Ok(walker) = std::fs::read_dir(build) else {
        return None;
    };
    for entry in walker.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_artifact(&path, name) {
                return Some(found);
            }
            continue;
        }
        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string());
        if stem.as_deref() == Some(name) {
            return Some(path);
        }
    }
    None
}

/// Package a desktop platform by configuring and building its CMake project.
/// Missing CMake → `blocked`; the produced binary is the artifact.
pub fn pack_desktop(
    platform: &str,
    project: &Project,
    opts: &PackOpts,
) -> Result<PackResult, String> {
    if let Err(blocked) = shared::require_sdks_blocked(&[SdkPrecondition::Cmake]) {
        return Ok(blocked);
    }

    let platform_dir = project.root.join(platform);
    let build = platform_dir.join("build");

    let cfg = cmake_configure_argv(&platform_dir);
    let cfg_ref: Vec<&str> = cfg.iter().map(String::as_str).collect();
    shared::run("cmake", &cfg_ref, &platform_dir)?;

    let b = cmake_build_argv(&platform_dir, opts);
    let b_ref: Vec<&str> = b.iter().map(String::as_str).collect();
    shared::run("cmake", &b_ref, &platform_dir)?;

    let artifact = find_artifact(&build, &project.package_name).ok_or_else(|| {
        format!(
            "cmake produced no `{}` artifact in {}",
            project.package_name,
            build.display()
        )
    })?;
    Ok(PackResult {
        tier: Tier::Artifact,
        artifact: Some(artifact),
        note: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::PackOpts;
    use std::path::PathBuf;

    fn opts(release: bool) -> PackOpts {
        PackOpts {
            release,
            features: vec![],
            abi: None,
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
    fn configure_points_at_the_platform_build_dir() {
        let argv = cmake_configure_argv(&project().root.join("linux"));
        assert_eq!(argv, vec!["-B", "/p/linux/build"]);
    }

    #[test]
    fn build_passes_release_config_only_when_requested() {
        let linux = project().root.join("linux");
        assert_eq!(
            cmake_build_argv(&linux, &opts(true)),
            vec!["--build", "/p/linux/build", "--config", "Release"]
        );
        assert_eq!(
            cmake_build_argv(&linux, &opts(false)),
            vec!["--build", "/p/linux/build"]
        );
    }

    #[test]
    fn find_artifact_matches_the_binary_by_name() {
        let dir = std::env::temp_dir().join(format!("floom-cli-desktop-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("sub/demo"), b"x").unwrap();
        let found = find_artifact(&dir, "demo").unwrap();
        assert_eq!(found.file_name().unwrap(), "demo");
        assert!(find_artifact(&dir, "nope").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
