// SPDX-License-Identifier: MIT
//! Per-platform packaging orchestration.

use std::path::PathBuf;

use crate::pack::shared::Project;

/// Whether a platform build produced a real artifact, only compiled, or was
/// blocked (e.g. missing SDK or device signing).
pub enum Tier {
    Artifact,
    CheckOnly,
    Blocked,
}

/// What `pack` returns: the tier, the produced artifact (if any), and a human
/// note (e.g. why it was blocked or that the artifact is unsigned). The string
/// form is `TIER=… PLATFORM=… CRATE=… ARTIFACT=…`, printed by `package::run`.
pub struct PackResult {
    pub tier: Tier,
    pub artifact: Option<PathBuf>,
    pub note: Option<String>,
}

/// Options parsed from `floom package` for a single platform build.
pub struct PackOpts {
    pub release: bool,
    pub features: Vec<String>,
    pub abi: Option<String>,
    pub install: bool,
    pub device: bool,
    pub yes: bool,
}

pub fn pack(platform: &str, project: &Project, opts: &PackOpts) -> Result<PackResult, String> {
    match platform {
        "android" => crate::pack::android::pack_android(project, opts),
        "ios" => crate::pack::ios::pack_ios(project, opts),
        "web" => crate::pack::web::pack_web(project, opts),
        "ohos" => crate::pack::ohos::pack_ohos(project, opts),
        "windows" | "linux" | "macos" => {
            crate::pack::desktop::pack_desktop(platform, project, opts)
        }
        other => Err(format!("platform `{other}` is not implemented yet")),
    }
}

pub mod android;
pub mod desktop;
pub mod ios;
pub mod ohos;
pub mod shared;
pub mod web;
