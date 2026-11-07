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

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_FEATURES: &str = "desktop,renderer-gpu";

/// Every crate the generated app depends on, by package name. `floom run`
/// looks each one up under the checkout's `crates/` directory, so the crate
/// layout can move around without this list changing.
const CRATE_NAMES: &[&str] = &[
    "floom",
    "floom-core",
    "floom-types",
    "floom-widgets",
    "floom-ui-composites",
    "floom-signal",
    // The TSX front end is a build-dependency of the generated crate.
    "floom-tsx",
    "floom-dsl",
];

/// Directories that cannot contain a crate and must not be searched.
const SKIPPED_DIRS: &[&str] = &["target", ".git", "node_modules", "vendor", ".floom"];

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
    let crates = locate_crates(&floom_root)?;
    let patches = forwarded_patches(&floom_root);
    let app_dir = app.parent().unwrap_or(Path::new("."));
    let crate_dir = app_dir.join(".floom").join("run");

    write_project(&crate_dir, &app, &crates, &opts.features, &patches)?;

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

/// Where the Floom checkout lives: an explicit path, `FLOOM_PATH`, or the
/// checkout that contains the app. Answers with the checkout root, whose
/// `crates/` directory holds the crates.
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
    if let Some(root) = workspace_root(app.parent().unwrap_or(Path::new(".")))
        && root.join("crates").is_dir()
    {
        return Ok(root);
    }
    Err(
        "cannot find the Floom checkout: pass `--floom-path <checkout>` or set FLOOM_PATH"
            .to_string(),
    )
}

/// Accept either the checkout root or its `crates` directory, and answer with
/// the checkout root.
fn check_floom_dir(path: &Path) -> Result<PathBuf, String> {
    if !path.is_dir() {
        return Err(format!("{} is not a directory", path.display()));
    }
    if path.join("crates").is_dir() {
        return Ok(path.to_path_buf());
    }
    if path.file_name().is_some_and(|name| name == "crates") && path.parent().is_some() {
        return Ok(path.parent().unwrap().to_path_buf());
    }
    Err(format!(
        "{} is not a Floom checkout: it has no `crates/` directory",
        path.display()
    ))
}

/// Find every crate `CRATE_NAMES` names under `<root>/crates/`, wherever the
/// layout puts it, and answer with package name -> directory. The crate
/// directories have been reorganised more than once and each holds a
/// `Cargo.toml` whose `[package].name` is what a path dependency has to point
/// at, so the layout itself is the thing to search rather than hard-code.
fn locate_crates(root: &Path) -> Result<BTreeMap<String, PathBuf>, String> {
    let crates = root.join("crates");
    if !crates.is_dir() {
        return Err(format!("{} has no `crates/` directory", root.display()));
    }
    let mut found: BTreeMap<String, PathBuf> = BTreeMap::new();
    let mut stack = vec![crates];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name();
            if SKIPPED_DIRS.contains(&name.to_string_lossy().as_ref()) {
                continue;
            }
            if let Some(pkg) = package_name(&path.join("Cargo.toml"))
                && !found.contains_key(&pkg)
                && CRATE_NAMES.contains(&pkg.as_str())
            {
                found.insert(pkg, path.clone());
            }
            stack.push(path);
        }
    }
    let missing: Vec<&str> = CRATE_NAMES
        .iter()
        .copied()
        .filter(|name| !found.contains_key(*name))
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "{} does not look like a Floom checkout: it is missing {missing:?}",
            root.display()
        ));
    }
    Ok(found)
}

/// The package name in a manifest, read without a TOML parser: the `name` key
/// of the `[package]` table is enough for a path dependency.
fn package_name(manifest: &Path) -> Option<String> {
    let text = fs::read_to_string(manifest).ok()?;
    let mut in_package = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if in_package
            && let Some(value) = line.strip_prefix("name")
            && let Some(value) = value.trim_start().strip_prefix('=')
        {
            return Some(value.trim().trim_matches('"').to_string());
        }
    }
    None
}

/// The `[patch.*]` tables of the checkout manifest, with their paths made
/// absolute. The generated crate is its own workspace, so it does not inherit
/// the checkout's patches — and the desktop backend depends on a vendored fork
/// of `winit` that crates.io does not serve.
fn forwarded_patches(root: &Path) -> String {
    let Ok(text) = fs::read_to_string(root.join("Cargo.toml")) else {
        return String::new();
    };
    let mut sections: Vec<(String, Vec<String>)> = Vec::new();
    let mut current: Option<(String, Vec<String>)> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if let Some(section) = current.take() {
                sections.push(section);
            }
            if trimmed.starts_with("[patch") {
                current = Some((trimmed.to_string(), Vec::new()));
            }
            continue;
        }
        if let Some((_, entries)) = current.as_mut()
            && let Some((name, rest)) = trimmed.split_once('=')
            && let Some(path) = patch_path(rest)
        {
            let absolute = root.join(path).display().to_string();
            entries.push(format!("{} = {{ path = {absolute:?} }}", name.trim()));
        }
    }
    if let Some(section) = current.take() {
        sections.push(section);
    }

    let mut out = String::new();
    for (header, entries) in sections {
        if entries.is_empty() {
            continue;
        }
        out.push_str(&header);
        out.push('\n');
        for entry in entries {
            out.push_str(&entry);
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

/// The quoted value of the `path` key inside one patch entry:
/// `{ path = "crates/vendor/…" }`.
fn patch_path(rest: &str) -> Option<&str> {
    let after_key = rest.split_once("path")?.1;
    let after_eq = after_key.split_once('=')?.1;
    let start = after_eq.find('"')? + 1;
    let tail = &after_eq[start..];
    Some(&tail[..tail.find('"')?])
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

/// TypeScript configuration for the generated project. `lib` excludes the DOM
/// on purpose: the widget globals `Text` / `Image` would collide with the DOM
/// constructors of the same name.
const TS_CONFIG: &str = r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "jsx": "preserve",
    "lib": ["ES2020"],
    "strict": true,
    "noEmit": true
  },
  "include": ["*.tsx", "*.ts"]
}
"#;

/// Write the generated crate, touching only files whose content changed so
/// cargo's fingerprints stay valid across runs.
fn write_project(
    dir: &Path,
    app: &Path,
    crates: &BTreeMap<String, PathBuf>,
    features: &str,
    patches: &str,
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
    let path_of = |name: &str| {
        // `locate_crates` found every name in `CRATE_NAMES` before we got here,
        // and every name below is in that list.
        crates
            .get(name)
            .unwrap_or_else(|| panic!("the `{name}` crate was not located"))
            .display()
            .to_string()
    };
    let dep = |name: &str| format!("{name} = {{ path = {path:?} }}\n", path = path_of(name));
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
         floom = {{ path = {floom_dep:?}, features = [{feature_list}] }}\n\
         {core}{types}{widgets}{composites}{signal}\
         \n\
         [build-dependencies]\n\
         {tsx}{dsl}\n\
         {patches}",
        app = app.display(),
        floom_dep = path_of("floom"),
        core = dep("floom-core"),
        types = dep("floom-types"),
        widgets = dep("floom-widgets"),
        composites = dep("floom-ui-composites"),
        signal = dep("floom-signal"),
        tsx = dep("floom-tsx"),
        dsl = dep("floom-dsl"),
    );

    // Only write tsconfig.json if it doesn't already exist
    let app_dir = std::path::Path::new(&app)
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let tsconfig = app_dir.join("tsconfig.json");
    if !tsconfig.exists() {
        write_if_changed(&tsconfig, TS_CONFIG)?;
    }

    let build_rs = format!(
        "// @generated by `floom run`.\n\
         fn main() {{\n\
         \x20   let app = {app:?};\n\
         \x20   println!(\"cargo:rerun-if-changed={{app}}\");\n\
         \x20   println!(\"cargo:rerun-if-changed=build.rs\");\n\
         \x20   let source = std::fs::read_to_string(&app).expect(\"read the TSX app\");\n\
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

    /// A crate layout shaped like the checkout's: each package name lives in
    /// its own subtree under `crates/`, and each has a `[package].name`.
    fn fake_crates(dir: &Path) -> BTreeMap<String, PathBuf> {
        let layout = [
            ("floom", "app/floom"),
            ("floom-core", "core/floom-core"),
            ("floom-types", "core/floom-types"),
            ("floom-widgets", "widgets/floom-widgets"),
            (
                "floom-ui-composites",
                "ecosystem/floom-ui/crates/floom-ui-composites",
            ),
            ("floom-signal", "core/floom-signal"),
            ("floom-tsx", "frontend/floom-tsx"),
            ("floom-dsl", "frontend/floom-dsl"),
        ];
        let mut map = BTreeMap::new();
        for (pkg, rel) in layout {
            let path = dir.join("crates").join(rel);
            fs::create_dir_all(&path).expect("mkdir");
            fs::write(
                path.join("Cargo.toml"),
                format!("[package]\nname = \"{pkg}\"\n"),
            )
            .expect("write manifest");
            map.insert(pkg.to_string(), path);
        }
        map
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
        let app = dir.join("app.tsx");
        let crates = fake_crates(&dir);
        write_project(
            &dir.join("run"),
            &app,
            &crates,
            "desktop,, renderer-gpu",
            "",
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
        let app = dir.join("app.tsx");
        let crates = fake_crates(&dir);
        write_project(&dir.join("run"), &app, &crates, "", "").expect("write");
        let manifest = fs::read_to_string(dir.join("run").join("Cargo.toml")).expect("manifest");
        assert!(manifest.contains("[workspace]"), "{manifest}");
        assert!(manifest.contains("features = []"), "{manifest}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn declarations_are_not_generated_at_run_time() {
        // Editor declarations ship as the published `@types/floom` package and
        // are generated ahead of time; `floom run` must not regenerate them in
        // the build script.
        let dir = std::env::temp_dir().join(format!("floom-run-dts-{}", std::process::id()));
        let app = dir.join("app.tsx");
        let crates = fake_crates(&dir);
        write_project(&dir.join("run"), &app, &crates, "", "").expect("write");
        let build_rs = fs::read_to_string(dir.join("run").join("build.rs")).expect("build.rs");
        assert!(
            build_rs.contains("floom_tsx::generate"),
            "the build script must still compile the app:\n{build_rs}"
        );
        assert!(
            !build_rs.contains("generate_dts"),
            "declarations belong to the @types/floom package, not `floom run`:\n{build_rs}"
        );
        let tsconfig = fs::read_to_string(dir.join("tsconfig.json")).expect("tsconfig.json");
        assert!(
            tsconfig.contains(r#""lib": ["ES2020"]"#),
            "the DOM lib would collide with the Text/Image widget globals:\n{tsconfig}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn tsconfig_is_written_once_and_not_overwritten() {
        // `floom run` writes a tsconfig next to the app so an editor picks up
        // the `@types/floom` declarations without configuration. It must not
        // clobber a tsconfig the author already owns.
        let dir = std::env::temp_dir().join(format!("floom-run-ts-{}", std::process::id()));
        let app = dir.join("app.tsx");
        let crates = fake_crates(&dir);
        // First run creates it; the author then owns it.
        write_project(&dir.join("run"), &app, &crates, "", "").expect("first write");
        let tsconfig = dir.join("tsconfig.json");
        assert!(tsconfig.exists(), "tsconfig.json must be created: {dir:?}");
        let owned = "{\"compilerOptions\": {\"strict\": false}}";
        fs::write(&tsconfig, owned).expect("seed author tsconfig");

        // Second run must leave the author's tsconfig untouched.
        write_project(&dir.join("run"), &app, &crates, "", "").expect("second write");
        let after = fs::read_to_string(&tsconfig).expect("tsconfig");
        assert_eq!(
            after, owned,
            "an existing tsconfig.json must not be overwritten"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_generated_manifest_points_at_where_crates_actually_are() {
        // The crate layout keeps the facade in `crates/app/`, the core in
        // `crates/core/` and the composites under `crates/ecosystem/`; a
        // manifest written from the old flat layout would not resolve.
        let dir = std::env::temp_dir().join(format!("floom-run-paths-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let expected = fake_crates(&dir);
        let found = locate_crates(&dir).expect("locate crates in the nested layout");
        assert_eq!(found, expected);

        let app = dir.join("app.tsx");
        fs::write(&app, "").expect("app");
        write_project(&dir.join("run"), &app, &found, "", "").expect("write");
        let manifest = fs::read_to_string(dir.join("run").join("Cargo.toml")).expect("manifest");
        assert!(
            manifest.contains(&format!(
                "floom = {{ path = {:?}, features = [] }}",
                expected["floom"].display().to_string()
            )),
            "{manifest}"
        );
        assert!(manifest.contains("crates/core/floom-core"), "{manifest}");
        assert!(
            manifest.contains("crates/ecosystem/floom-ui/crates/floom-ui-composites"),
            "{manifest}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn locate_crates_names_the_missing_packages() {
        let dir = std::env::temp_dir().join(format!("floom-run-missing-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let only = dir.join("crates").join("app").join("floom");
        fs::create_dir_all(&only).expect("mkdir");
        fs::write(only.join("Cargo.toml"), "[package]\nname = \"floom\"\n").expect("manifest");

        let error = locate_crates(&dir).expect_err("incomplete checkout");
        assert!(error.contains("floom-core"), "{error}");
        assert!(error.contains("floom-dsl"), "{error}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_floom_path_may_be_the_checkout_root_or_its_crates_dir() {
        let dir = std::env::temp_dir().join(format!("floom-run-root-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("crates")).expect("mkdir");
        assert_eq!(check_floom_dir(&dir).expect("root"), dir);
        assert_eq!(
            check_floom_dir(&dir.join("crates")).expect("crates dir"),
            dir
        );

        let unrelated = dir.join("unrelated");
        fs::create_dir_all(&unrelated).expect("mkdir");
        let error = check_floom_dir(&unrelated).expect_err("not a checkout");
        assert!(error.contains("no `crates/` directory"), "{error}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_checkout_patches_are_forwarded_with_absolute_paths() {
        let dir = std::env::temp_dir().join(format!("floom-run-patch-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(
            dir.join("Cargo.toml"),
            "[workspace]\n\
             members = []\n\
             \n\
             [patch.\"https://github.com/lapce/winit\"]\n\
             floem-winit = { path = \"crates/vendor/floem-winit/winit\" }\n\
             floem-winit-core = { path = \"crates/vendor/floem-winit/winit-core\" }\n\
             dpi = { path = \"crates/vendor/floem-winit/dpi\" }\n",
        )
        .expect("manifest");

        let patches = forwarded_patches(&dir);
        assert!(
            patches
                .matches("[patch.\"https://github.com/lapce/winit\"]")
                .count()
                == 1,
            "each patch table must be emitted once:\n{patches}"
        );
        assert!(
            patches.contains(&format!(
                "floem-winit = {{ path = \"{}/crates/vendor/floem-winit/winit\" }}",
                dir.display()
            )),
            "patch paths must be absolute:\n{patches}"
        );
        assert!(patches.contains("dpi = { path ="), "{patches}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn forwarded_patches_reach_the_generated_manifest() {
        let dir = std::env::temp_dir().join(format!("floom-run-patched-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let crates = fake_crates(&dir);
        let app = dir.join("app.tsx");
        fs::write(&app, "").expect("app");
        let patch = "[patch.\"https://example.invalid/x\"]\nfoo = { path = \"/nowhere\" }\n\n";
        write_project(&dir.join("run"), &app, &crates, "", patch).expect("write");
        let manifest = fs::read_to_string(dir.join("run").join("Cargo.toml")).expect("manifest");
        assert!(
            manifest.contains("[patch.\"https://example.invalid/x\"]"),
            "{manifest}"
        );
        assert!(
            manifest.contains("foo = { path = \"/nowhere\" }"),
            "{manifest}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
