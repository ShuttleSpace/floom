// SPDX-License-Identifier: MIT
//! Web packaging: compile to `wasm32-unknown-unknown`, then run `wasm-bindgen`.
//!
//! When the crate exports no `#[wasm_bindgen]` bindings `wasm-bindgen` produces
//! no JS, so the build is reported as `check-only` and the raw `.wasm` is the
//! artifact — there is nothing runnable to hand back.

use std::path::Path;

use crate::pack::shared::{self, Project};
use crate::pack::{PackOpts, PackResult, Tier};
use crate::toolchain::{CargoTool, RustupTarget, Tool};

/// The `cargo build` argv for the wasm32 target.
pub fn wasm_build_argv(opts: &PackOpts) -> Vec<String> {
    let mut argv = vec![
        "build".to_string(),
        "--target".to_string(),
        "wasm32-unknown-unknown".to_string(),
    ];
    if opts.release {
        argv.push("--release".to_string());
    }
    shared::append_features(&mut argv, &opts.features);
    argv
}

/// The `wasm-bindgen --target web` argv for a built `.wasm`, emitting JS/glue
/// into the project's `web/pkg`.
pub fn wasm_bindgen_argv(project: &Project, wasm: &Path, _opts: &PackOpts) -> Vec<String> {
    let out = project.root.join("web/pkg").to_string_lossy().into_owned();
    vec![
        "--target".to_string(),
        "web".to_string(),
        "--out-dir".to_string(),
        out,
        wasm.to_string_lossy().into_owned(),
    ]
}

/// Package the web scaffold: provision the wasm32 target + `wasm-bindgen`, build
/// the `.wasm`, then bind it. No JS bindings → `check-only` with the raw wasm.
pub fn pack_web(project: &Project, opts: &PackOpts) -> Result<PackResult, String> {
    crate::toolchain::provision(
        &[
            Tool::Rustup(RustupTarget("wasm32-unknown-unknown")),
            Tool::Cargo(CargoTool::WasmBindgen),
        ],
        opts.yes,
    )?;

    let build = wasm_build_argv(opts);
    let build_ref: Vec<&str> = build.iter().map(String::as_str).collect();
    shared::run("cargo", &build_ref, &project.root)?;

    let wasm = shared::newest_artifact(&project.target_dir, "wasm")
        .ok_or_else(|| "cargo build produced no .wasm".to_string())?;

    let out = project.root.join("web/pkg");
    let _ = std::fs::create_dir_all(&out);
    let wb = wasm_bindgen_argv(project, &wasm, opts);
    let wb_ref: Vec<&str> = wb.iter().map(String::as_str).collect();
    let _ = shared::run("wasm-bindgen", &wb_ref, &project.root);

    match shared::newest_artifact(&out, "js") {
        Some(js) => Ok(PackResult {
            tier: Tier::Artifact,
            artifact: Some(js),
            note: None,
        }),
        None => Ok(PackResult {
            tier: Tier::CheckOnly,
            artifact: Some(wasm),
            note: Some("no JS bindings generated; reporting raw .wasm".into()),
        }),
    }
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
    fn wasm_build_argv_targets_wasm32_and_release_flag() {
        assert_eq!(
            wasm_build_argv(&opts(true)),
            vec!["build", "--target", "wasm32-unknown-unknown", "--release"]
        );
        let debug = wasm_build_argv(&opts(false));
        assert!(!debug.contains(&"--release".to_string()));
    }

    #[test]
    fn wasm_bindgen_argv_targets_web_with_out_dir() {
        let wasm = Path::new("/p/target/wasm32/demo.wasm");
        let argv = wasm_bindgen_argv(&project(), wasm, &opts(true));
        assert_eq!(argv[0], "--target");
        assert_eq!(argv[1], "web");
        assert_eq!(argv[2], "--out-dir");
        assert_eq!(argv[3], "/p/web/pkg");
        assert_eq!(argv[4], "/p/target/wasm32/demo.wasm");
    }
}
