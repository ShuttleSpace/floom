// SPDX-License-Identifier: MIT
//! OpenHarmony packaging: build the cdylib `.so`, stage it, then `hvigorw`.
//!
//! `floom create` emits a DevEco/hvigor project whose `EntryAbility` imports
//! `libfloom.so`; packaging builds the cdylib with the OHOS rustup target,
//! stages it into the HAP module's `libs`, and runs `hvigorw assembleHap`.
//! The DevEco SDK cannot be auto-installed, so its absence yields `blocked`.

use std::path::Path;

use crate::pack::shared::{self, Project};
use crate::pack::{PackOpts, PackResult, Tier};
use crate::toolchain::{RustupTarget, SdkPrecondition, Tool};

/// The `cargo build` argv for the OHOS target.
pub fn ohos_build_argv(opts: &PackOpts) -> Vec<String> {
    let mut argv = vec![
        "build".to_string(),
        "--target".to_string(),
        "aarch64-unknown-linux-ohos".to_string(),
    ];
    if opts.release {
        argv.push("--release".to_string());
    }
    argv
}

/// Which hvigor to invoke and with what task. `hvigorw` wins when the scaffold
/// shipped one; otherwise `hvigor` from PATH.
pub fn assemble_argv(project: &Project, _opts: &PackOpts) -> (String, Vec<String>) {
    let ohos_dir = project.root.join("ohos");
    let hvigor = if ohos_dir.join("hvigorw").exists() {
        "./hvigorw".to_string()
    } else {
        "hvigor".to_string()
    };
    (hvigor, vec!["assembleHap".to_string()])
}

/// Package the OHOS scaffold. Missing DevEco/hvigor → `blocked` (cannot install
/// the SDK); otherwise provision the rustup target, build the `.so`, stage it
/// into the HAP module, and assemble the HAP.
pub fn pack_ohos(project: &Project, opts: &PackOpts) -> Result<PackResult, String> {
    if let Err(blocked) = shared::require_sdks_blocked(&[SdkPrecondition::Hvigor]) {
        return Ok(blocked);
    }

    crate::toolchain::provision(
        &[Tool::Rustup(RustupTarget("aarch64-unknown-linux-ohos"))],
        opts.yes,
    )?;

    let build = ohos_build_argv(opts);
    let build_ref: Vec<&str> = build.iter().map(String::as_str).collect();
    shared::run("cargo", &build_ref, &project.root)?;

    let so = shared::newest_artifact(&project.target_dir, "so")
        .ok_or_else(|| "cargo build produced no .so".to_string())?;
    let libs = project.root.join("ohos/entry/libs/arm64-v8a");
    std::fs::create_dir_all(&libs)
        .map_err(|e| format!("cannot create {}: {e}", libs.display()))?;
    let dest = libs.join(so.file_name().unwrap());
    std::fs::copy(&so, &dest).map_err(|e| format!("cannot stage .so into hvigor project: {e}"))?;

    let (hvigor, hargs) = assemble_argv(project, opts);
    let hvigor_ref = hvigor.as_str();
    let hargs_ref: Vec<&str> = hargs.iter().map(String::as_str).collect();
    shared::run(hvigor_ref, &hargs_ref, &project.root.join("ohos"))?;

    let hap = shared::newest_artifact(&project.root.join("ohos"), "hap")
        .ok_or_else(|| "hvigorw produced no .hap".to_string())?;
    Ok(PackResult {
        tier: Tier::Artifact,
        artifact: Some(hap),
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
            manifest: None,
            install: false,
            device: false,
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
    fn ohos_build_argv_targets_the_ohos_rustup_target() {
        assert_eq!(
            ohos_build_argv(&opts(true)),
            vec![
                "build",
                "--target",
                "aarch64-unknown-linux-ohos",
                "--release"
            ]
        );
        assert!(!ohos_build_argv(&opts(false)).contains(&"--release".to_string()));
    }

    #[test]
    fn assemble_uses_hvigorw_or_path_binary() {
        let (prog, args) = assemble_argv(&project(), &opts(true));
        assert_eq!(prog, "hvigor"); // no hvigorw in the test path
        assert_eq!(args, vec!["assembleHap".to_string()]);
    }
}
