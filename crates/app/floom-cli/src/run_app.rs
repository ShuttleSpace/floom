// SPDX-License-Identifier: MIT
//! `floom run <app.tsx>` — compile a TSX application and launch it.
//!
//! The command is the "no Rust at all" path: the author writes one `.tsx` file
//! whose root is an `<App>` wrapper, and `floom` generates a throwaway Cargo
//! project around it, builds it, and runs the binary.
//!
//! # Why a generated crate instead of embedding the compiler
//!
//! `floom-cli` is deliberately dependency-free: it must build from std alone on
//! a machine that is only about to scaffold or run an app. The TSX front end
//! (`swc` + `floom-tsx`) is heavy, so it lives in the generated crate's
//! `build-dependencies` instead. Cargo then caches it like any other build
//! script, and the CLI itself stays small.
//!
//! The generated crate is written under `<app dir>/.floom/run`, so re-running
//! only rebuilds what changed.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_FEATURES: &str = "desktop,renderer-gpu";

#[derive(Debug)]
struct RunOpts {
    app: PathBuf,
    release: bool,
    build_only: bool,
    features: String,
    floom_path: Option<String>,
}

/// Entry point for `floom run`.
pub fn run(args: &[String]) -> Result<(), String> {
    let opts = parse(args)?;
    let app = opts
        .app
        .canonicalize()
        .map_err(|e| format!("cannot open {}: {e}", opts.app.display()))?;
    if !app.is_file() {
        return Err(format!("{} is not a file", app.display()));
    }
    match app.extension().and_then(|e| e.to_str()) {
        Some("tsx") | Some("ts") => {}
        _ => {
            return Err(format!(
                "{} is not a TSX application; expected a `.tsx` file",
                app.display()
            ));
        }
    }

    let floom_root = resolve_floom_path(opts.floom_path.as_deref(), &app)?;
    let app_dir = app.parent().unwrap_or(Path::new("."));
    let crate_dir = app_dir.join(".floom").join("run");

    write_project(&crate_dir, &app, &floom_root, &opts.features)?;

    let mut cmd = Command::new("cargo");
    cmd.arg(if opts.build_only { "build" } else { "run" })
        .arg("--manifest-path")
        .arg(crate_dir.join("Cargo.toml"));
    if opts.release {
        cmd.arg("--release");
    }
    // Reuse the enclosing workspace's `target/` when there is one, so the first
    // run does not have to rebuild the whole Floom stack from scratch.
    if let Some(workspace) = workspace_root(app_dir) {
        cmd.env("CARGO_TARGET_DIR", workspace.join("target"));
    }
    let status = cmd.status().map_err(|e| format!("cannot run cargo: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("`floom run` failed ({status})"))
    }
}

fn parse(args: &[String]) -> Result<RunOpts, String> {
    let mut app = None;
    let mut release = false;
    let mut build_only = false;
    let mut features = DEFAULT_FEATURES.to_string();
    let mut floom_path = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--release" => release = true,
            "--no-run" | "--build-only" => build_only = true,
            "--features" => {
                i += 1;
                features = args
                    .get(i)
                    .ok_or("`--features` requires a value")?
                    .to_string();
            }
            "--floom-path" => {
                i += 1;
                floom_path = Some(
                    args.get(i)
                        .ok_or("`--floom-path` requires a value")?
                        .to_string(),
                );
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unknown option `{flag}` for `run`"));
            }
            path => {
                if app.replace(PathBuf::from(path)).is_some() {
                    return Err("`run` takes one TSX app path".into());
                }
            }
        }
        i += 1;
    }

    Ok(RunOpts {
        app: app.ok_or("`run` requires a TSX app path, e.g. `floom run app.tsx`")?,
        release,
        build_only,
        features,
        floom_path,
    })
}

/// Where the Floom crates live: an explicit path, `FLOOM_PATH`, the checkout
/// that contains the app, or — as a last resort — the published version.
fn resolve_floom_path(explicit: Option<&str>, app: &Path) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        return check_floom_dir(Path::new(path));
    }
    if let Ok(path) = std::env::var("FLOOM_PATH")
        && !path.is_empty()
    {
        return check_floom_dir(Path::new(&path));
    }
    // Running `floom run` from inside a Floom checkout is the common case while
    // the toolchain is pre-release; find it by walking up from the app.
    if let Some(root) = workspace_root(app.parent().unwrap_or(Path::new("."))) {
        let candidate = root.join("crates").join("floom");
        if candidate.is_dir() {
            return Ok(root.join("crates"));
        }
    }
    Err(
        "cannot find the Floom crates: pass `--floom-path <checkout>` or set FLOOM_PATH"
            .to_string(),
    )
}

fn check_floom_dir(path: &Path) -> Result<PathBuf, String> {
    if !path.is_dir() {
        return Err(format!("{} is not a directory", path.display()));
    }
    // Accept either the checkout root or its `crates` directory.
    let crates = if path.join("crates").join("floom").is_dir() {
        path.join("crates")
    } else {
        path.to_path_buf()
    };
    if !crates.join("floom").is_dir() {
        return Err(format!(
            "{} does not contain a `floom` crate",
            crates.display()
        ));
    }
    Ok(crates)
}

/// The nearest ancestor holding a `Cargo.toml` with a `[workspace]` table.
fn workspace_root(start: &Path) -> Option<PathBuf> {
    let mut dir = start.canonicalize().ok()?;
    loop {
        let manifest = dir.join("Cargo.toml");
        if let Ok(text) = fs::read_to_string(&manifest)
            && text.lines().any(|l| l.trim() == "[workspace]")
        {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Write the generated crate, touching only files whose content changed so
/// cargo's fingerprints stay valid across runs.
fn write_project(
    dir: &Path,
    app: &Path,
    floom_crates: &Path,
    features: &str,
) -> Result<(), String> {
    fs::create_dir_all(dir.join("src"))
        .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;

    let feature_list = features
        .split(',')
        .map(str::trim)
        .filter(|f| !f.is_empty())
        .map(|f| format!("{f:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    let dep = |name: &str| {
        format!(
            "{name} = {{ path = {path:?} }}\n",
            path = floom_crates.join(name).display().to_string()
        )
    };
    // `floom-ui-composites` lives under `crates/ecosystem/floom-ui/crates`, not
    // directly under `crates`, so the generic `dep` join cannot find it.
    let composites = format!(
        "floom-ui-composites = {{ path = {:?} }}\n",
        floom_crates
            .join("ecosystem/floom-ui/crates/floom-ui-composites")
            .display()
            .to_string()
    );
    let manifest = format!(
        "# @generated by `floom run`; the app itself is {app}.\n\
         [package]\n\
         name = \"floom-app-run\"\n\
         version = \"0.0.0\"\n\
         edition = \"2021\"\n\
         publish = false\n\
         \n\
         # Its own workspace root: the app may live inside another workspace.\n\
         [workspace]\n\
         \n\
         [[bin]]\n\
         name = \"app\"\n\
         path = \"src/main.rs\"\n\
         \n\
         [dependencies]\n\
         floom = {{ path = {floom:?}, features = [{feature_list}] }}\n\
         {core}{types}{widgets}{composites}{signal}\
         \n\
         [build-dependencies]\n\
         {tsx}{dsl}",
        app = app.display(),
        floom = floom_crates.join("floom").display().to_string(),
        core = dep("floom-core"),
        types = dep("floom-types"),
        widgets = dep("floom-widgets"),
        signal = dep("floom-signal"),
        tsx = dep("floom-tsx"),
        dsl = dep("floom-dsl"),
    );

    let build_rs = format!(
        "// @generated by `floom run`.\n\
         fn main() {{\n\
         \x20   let app = {app:?};\n\
         \x20   println!(\"cargo:rerun-if-changed={{app}}\");\n\
         \x20   println!(\"cargo:rerun-if-changed=build.rs\");\n\
         \x20   let source = std::fs::read_to_string(&app).expect(\"read the TSX app\");\n\
         \x20   let doc = floom_tsx::compile(&source)\n\
         \x20       .unwrap_or_else(|e| panic!(\"{{app}}: {{e}}\"));\n\
         \x20   let dts = std::path::Path::new(&app).with_extension(\"d.ts\");\n\
         \x20   std::fs::write(&dts, floom_tsx::generate_dts(&doc))\n\
         \x20       .expect(\"write the .d.ts\");\n\
         \x20   let generated = floom_tsx::generate(&source)\n\
         \x20       .unwrap_or_else(|e| panic!(\"{{app}}: {{e}}\"));\n\
         \x20   let out = std::env::var(\"OUT_DIR\").expect(\"OUT_DIR\");\n\
         \x20   std::fs::write(format!(\"{{out}}/ui_gen.rs\"), generated.ui_gen).expect(\"write ui_gen.rs\");\n\
         \x20   std::fs::write(format!(\"{{out}}/main_gen.rs\"), generated.main).expect(\"write main_gen.rs\");\n\
         }}\n",
        app = app.display().to_string(),
    );

    let main_rs = "// @generated by `floom run`: the real entry is generated from the TSX app.\n\
                   include!(concat!(env!(\"OUT_DIR\"), \"/main_gen.rs\"));\n";

    write_if_changed(&dir.join("Cargo.toml"), &manifest)?;
    write_if_changed(&dir.join("build.rs"), &build_rs)?;
    write_if_changed(&dir.join("src").join("main.rs"), main_rs)?;
    Ok(())
}

fn write_if_changed(path: &Path, content: &str) -> Result<(), String> {
    if let Ok(existing) = fs::read_to_string(path)
        && existing == content
    {
        return Ok(());
    }
    fs::write(path, content).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn run_requires_an_app_path() {
        let error = parse(&args(&[])).expect_err("no app");
        assert!(error.contains("floom run app.tsx"), "{error}");
    }

    #[test]
    fn run_rejects_two_app_paths() {
        let error = parse(&args(&["a.tsx", "b.tsx"])).expect_err("two apps");
        assert!(error.contains("one TSX app path"), "{error}");
    }

    #[test]
    fn run_reads_the_flags_it_documents() {
        let opts = parse(&args(&[
            "app.tsx",
            "--release",
            "--no-run",
            "--features",
            "desktop",
            "--floom-path",
            "/tmp/floom",
        ]))
        .expect("parse");
        assert!(opts.release);
        assert!(opts.build_only);
        assert_eq!(opts.features, "desktop");
        assert_eq!(opts.floom_path.as_deref(), Some("/tmp/floom"));
    }

    #[test]
    fn empty_feature_entries_are_dropped() {
        let dir = std::env::temp_dir().join(format!("floom-run-test-{}", std::process::id()));
        let crates = dir.join("crates");
        for name in [
            "floom",
            "floom-core",
            "floom-types",
            "floom-widgets",
            "floom-signal",
            "floom-tsx",
            "floom-dsl",
        ] {
            fs::create_dir_all(crates.join(name)).expect("mkdir");
        }
        write_project(
            &dir.join("run"),
            Path::new("/tmp/app.tsx"),
            &crates,
            "desktop,, renderer-gpu",
        )
        .expect("write");
        let manifest = fs::read_to_string(dir.join("run").join("Cargo.toml")).expect("manifest");
        assert!(
            manifest.contains(r#"features = ["desktop", "renderer-gpu"]"#),
            "{manifest}"
        );
        assert!(!manifest.contains(r#""""#), "{manifest}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_generated_crate_is_its_own_workspace_root() {
        let dir = std::env::temp_dir().join(format!("floom-run-ws-{}", std::process::id()));
        let crates = dir.join("crates");
        for name in [
            "floom",
            "floom-core",
            "floom-types",
            "floom-widgets",
            "floom-signal",
            "floom-tsx",
            "floom-dsl",
        ] {
            fs::create_dir_all(crates.join(name)).expect("mkdir");
        }
        write_project(&dir.join("run"), Path::new("/tmp/app.tsx"), &crates, "").expect("write");
        let manifest = fs::read_to_string(dir.join("run").join("Cargo.toml")).expect("manifest");
        assert!(manifest.contains("[workspace]"), "{manifest}");
        assert!(manifest.contains("features = []"), "{manifest}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_generated_build_script_emits_editor_declarations() {
        // The TSX app is the source of truth, and the declarations are what an
        // editor completes against. They are written beside the app so the
        // editor finds them without a path to configure.
        let dir = std::env::temp_dir().join(format!("floom-run-dts-{}", std::process::id()));
        let crates = dir.join("crates");
        for name in [
            "floom",
            "floom-core",
            "floom-types",
            "floom-widgets",
            "floom-signal",
            "floom-tsx",
            "floom-dsl",
        ] {
            fs::create_dir_all(crates.join(name)).expect("mkdir");
        }
        write_project(&dir.join("run"), Path::new("/tmp/app.tsx"), &crates, "").expect("write");
        let build_rs = fs::read_to_string(dir.join("run").join("build.rs")).expect("build.rs");
        assert!(
            build_rs.contains("floom_tsx::generate_dts"),
            "the build script must emit the declarations:\n{build_rs}"
        );
        assert!(
            build_rs.contains(r#"with_extension("d.ts")"#),
            "the declarations belong beside the app:\n{build_rs}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
