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
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use crate::watch::{Watch, WatchError};

const DEFAULT_FEATURES: &str = "desktop,renderer-gpu";

/// Default poll period for `--watch`: a change is seen within one period.
const DEFAULT_POLL_MS: u64 = 200;
/// Default debounce for `--watch`: an editor's save burst collapses into one
/// rebuild.
const DEFAULT_DEBOUNCE_MS: u64 = 100;

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
    watch: bool,
    poll_ms: u64,
    debounce_ms: u64,
    no_relaunch: bool,
}

/// Entry point for `floom run`.
pub fn run(args: &[String]) -> Result<(), String> {
    let opts = parse(args)?;
    let app = opts
        .app
        .canonicalize()
        .map_err(|e| format!("cannot open {}: {e}", opts.app.display()))?;
    if app.is_dir() {
        return run_rust_dir(&app, &opts);
    }
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

    // The target directory the generated crate builds into: reuse the enclosing
    // workspace's `target/` when there is one, so the first run does not have to
    // rebuild the whole Floom stack from scratch; otherwise the crate's own.
    let target_dir = if let Some(workspace) = workspace_root(app_dir) {
        workspace.join("target")
    } else {
        crate_dir.join("target")
    };

    if opts.watch {
        return watch_run(&opts, &crate_dir, &target_dir, &app);
    }

    if opts.build_only {
        build_app(&crate_dir, &target_dir, opts.release)?;
        return Ok(());
    }

    // Non-watch launch: `cargo run` builds and runs in one shot.
    let mut cmd = Command::new("cargo");
    cmd.arg("run")
        .arg("--manifest-path")
        .arg(crate_dir.join("Cargo.toml"));
    if opts.release {
        cmd.arg("--release");
    }
    cmd.env("CARGO_TARGET_DIR", &target_dir);
    let status = cmd.status().map_err(|e| format!("cannot run cargo: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("`floom run` failed ({status})"))
    }
}

/// Build the generated crate and answer the path of the produced binary.
///
/// Returns `<target_dir>/<debug|release>/app`; the crate's `[[bin]]` is named
/// `app` and the enclosing `CARGO_TARGET_DIR` is the one we passed, so the
/// artifact lands there regardless of which directory `cargo` runs in.
fn binary_path(target_dir: &Path, release: bool) -> PathBuf {
    target_dir
        .join(if release { "release" } else { "debug" })
        .join(if cfg!(windows) { "app.exe" } else { "app" })
}

/// `cargo build` the generated crate, forwarding `--release`. Used by both the
/// `--no-run` path and the watch loop; errors carry the cargo failure.
fn build_app(crate_dir: &Path, target_dir: &Path, release: bool) -> Result<PathBuf, String> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--manifest-path")
        .arg(crate_dir.join("Cargo.toml"));
    if release {
        cmd.arg("--release");
    }
    cmd.env("CARGO_TARGET_DIR", target_dir);
    let status = cmd.status().map_err(|e| format!("cannot run cargo: {e}"))?;
    if status.success() {
        Ok(binary_path(target_dir, release))
    } else {
        Err(format!("`floom run` build failed ({status})"))
    }
}

/// `floom run --watch`: build once, launch the binary directly (so it is
/// killable), then on every settled file change kill the running copy, rebuild,
/// and relaunch — a hot *restart* (the window closes and reopens; app state is
/// lost). This is honest about its boundary versus stateful in-window hot
/// reload, which `floom-hot-reload`'s `UiPatch` does not yet bridge to a live
/// host (see `docs/developer-experience.md`).
///
/// `--no-relaunch` keeps the watch loop but skips the spawn, so the latency
/// harness measures save→rebuild without needing a display — the only part
/// runnable on a headless machine.
fn watch_run(
    opts: &RunOpts,
    crate_dir: &Path,
    target_dir: &Path,
    app: &Path,
) -> Result<(), String> {
    let bin = build_app(crate_dir, target_dir, opts.release)?;
    println!(
        "floom: watching {} for changes (poll {}ms, debounce {}ms)",
        app.display(),
        opts.poll_ms,
        opts.debounce_ms
    );

    let mut child: Option<Child> = None;
    if !opts.no_relaunch {
        child = Some(spawn_binary(&bin)?);
    }

    let mut watch = Watch::new(
        vec![app.parent().unwrap_or(Path::new(".")).to_path_buf()],
        Duration::from_millis(opts.poll_ms),
        Duration::from_millis(opts.debounce_ms),
    )
    .map_err(|e| match e {
        WatchError::MissingRoot(p) => format!("watch root disappeared: {}", p.display()),
        WatchError::Io(p, e) => format!("{}: {e}", p.display()),
    })?;

    loop {
        let change = match watch.wait_for_change() {
            Ok(c) => c,
            Err(e) => return Err(e.to_string()),
        };
        println!(
            "floom: change detected (+{:?}, {} source files)",
            change.detected_after,
            change.snapshot.len()
        );

        // Tear down the running copy before rebuilding.
        if let Some(mut c) = child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }

        let rebuild_start = Instant::now();
        let bin = build_app(crate_dir, target_dir, opts.release)?;
        let rebuild_ms = rebuild_start.elapsed();

        let relaunch_ms = if opts.no_relaunch {
            Duration::ZERO
        } else {
            let spawn_start = Instant::now();
            child = Some(spawn_binary(&bin)?);
            spawn_start.elapsed()
        };

        let total = change.first_seen.elapsed();
        println!(
            "floom: reloaded — detect +{:?}, debounce {}ms, rebuild {:?}, relaunch {:?} \
             => save→running ~{:?} (detection bounded by poll {}ms)",
            change.detected_after, opts.debounce_ms, rebuild_ms, relaunch_ms, total, opts.poll_ms
        );
    }
}


/// `floom run <dir> --watch`: build a Rust app and hand the reload to it.
///
/// Only the app knows what "a source file changed" means for its own tree, so the
/// CLI's whole job here is `cargo run` plus `--watch` on the far side of `--`. The
/// apps that answer it are the ones built on `run_desktop_software_watched`, which
/// rebuilds in place and keeps widget state; an app that ignores the flag simply
/// runs, which is why this is a flag and not a contract.
fn run_rust_dir(dir: &Path, opts: &RunOpts) -> Result<(), String> {
    let manifest = dir.join("Cargo.toml");
    if !manifest.is_file() {
        return Err(format!(
            "{} has no Cargo.toml; `floom run` takes a `.tsx` file or a Rust app directory",
            dir.display()
        ));
    }
    let argv = rust_run_argv(&manifest, opts.release, opts.watch);
    if opts.watch {
        println!(
            "floom: running {} with --watch; the app owns the rebuild, so changes it \
             watches are reloaded in place rather than by restarting it",
            dir.display()
        );
    }
    let status = Command::new(&argv[0])
        .args(&argv[1..])
        .status()
        .map_err(|e| format!("cannot run {}: {e}", argv[0]))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("the app exited with {status}"))
    }
}

/// The command that starts a Rust app, flag and all. Split out so the flag
/// placement (after `--`, or it lands on cargo) can be tested without cargo.
fn rust_run_argv(manifest: &Path, release: bool, watch: bool) -> Vec<String> {
    let mut argv = vec![
        "cargo".to_string(),
        "run".to_string(),
        "--manifest-path".to_string(),
        manifest.display().to_string(),
    ];
    if release {
        argv.push("--release".to_string());
    }
    if watch {
        argv.push("--".to_string());
        argv.push("--watch".to_string());
    }
    argv
}

/// Spawn the built binary directly (not `cargo run`) so it can be killed and
/// replaced on the next change. It shares this process's foreground process
/// group, so a terminal Ctrl-C also terminates it — no orphaned windows.
fn spawn_binary(bin: &Path) -> Result<Child, String> {
    Command::new(bin)
        .spawn()
        .map_err(|e| format!("cannot launch {}: {e}", bin.display()))
}

fn parse(args: &[String]) -> Result<RunOpts, String> {
    let mut app = None;
    let mut release = false;
    let mut build_only = false;
    let mut features = DEFAULT_FEATURES.to_string();
    let mut floom_path = None;
    let mut watch = false;
    let mut poll_ms = DEFAULT_POLL_MS;
    let mut debounce_ms = DEFAULT_DEBOUNCE_MS;
    let mut no_relaunch = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--release" => release = true,
            "--no-run" | "--build-only" => build_only = true,
            "--watch" => watch = true,
            // A measure-only latency harness still needs the watch loop, so this
            // implies `--watch` even if it was not spelled out.
            "--no-relaunch" => {
                no_relaunch = true;
                watch = true;
            }
            "--poll-ms" => {
                i += 1;
                poll_ms = args
                    .get(i)
                    .ok_or("`--poll-ms` requires a value")?
                    .parse::<u64>()
                    .map_err(|_| "`--poll-ms` must be a positive integer".to_string())?;
            }
            "--debounce-ms" => {
                i += 1;
                debounce_ms = args
                    .get(i)
                    .ok_or("`--debounce-ms` requires a value")?
                    .parse::<u64>()
                    .map_err(|_| "`--debounce-ms` must be a positive integer".to_string())?;
            }
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
        watch,
        poll_ms,
        debounce_ms,
        no_relaunch,
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
            // `shared::arg` gives forward slashes; `{:?}` on a `PathBuf` would
            // hand TOML a raw `C:\...`, where `\a` is an invalid escape.
            let absolute = crate::pack::shared::arg(root.join(path));
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
    let dep = |name: &str| {
        format!(
            "{name} = {{ path = {:?} }}\n",
            crate::pack::shared::arg(path_of(name))
        )
    };
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
        app = crate::pack::shared::arg(app),
        floom_dep = crate::pack::shared::arg(path_of("floom")),
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
        assert!(!opts.watch, "watch is opt-in");
        assert_eq!(opts.poll_ms, DEFAULT_POLL_MS);
        assert_eq!(opts.debounce_ms, DEFAULT_DEBOUNCE_MS);
    }

    #[test]
    fn run_reads_the_watch_flags() {
        let opts = parse(&args(&[
            "app.tsx",
            "--watch",
            "--poll-ms",
            "50",
            "--debounce-ms",
            "25",
        ]))
        .expect("parse");
        assert!(opts.watch);
        assert_eq!(opts.poll_ms, 50);
        assert_eq!(opts.debounce_ms, 25);
        assert!(!opts.no_relaunch);
    }

    #[test]
    fn no_relaunch_implies_watch() {
        // A measure-only harness still needs the watch loop; `--no-relaunch`
        // must not silently fall back to a one-shot `cargo run`.
        let opts = parse(&args(&["app.tsx", "--no-relaunch"])).expect("parse");
        assert!(opts.watch);
        assert!(opts.no_relaunch);
    }

    #[test]
    fn bad_watch_durations_are_rejected() {
        assert!(parse(&args(&["app.tsx", "--poll-ms", "abc"])).is_err());
        assert!(parse(&args(&["app.tsx", "--debounce-ms", "-1"])).is_err());
        assert!(parse(&args(&["app.tsx", "--poll-ms"])).is_err());
    }

    #[test]
    fn the_binary_path_matches_the_profile_and_platform() {
        let target = Path::new("/tmp/floom/target");
        let debug = binary_path(target, false);
        let release = binary_path(target, true);
        assert!(debug.ends_with("debug/app") || debug.ends_with("debug/app.exe"));
        assert!(release.ends_with("release/app") || release.ends_with("release/app.exe"));
        assert_ne!(debug, release);
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
                crate::pack::shared::arg(&expected["floom"])
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
                "floem-winit = {{ path = {:?} }}",
                crate::pack::shared::arg(dir.join("crates/vendor/floem-winit/winit"))
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

#[cfg(test)]
mod run_rust_tests {
    use super::*;

    #[test]
    fn the_watch_flag_goes_after_the_separator() {
        let argv = rust_run_argv(Path::new("/tmp/app/Cargo.toml"), false, true);
        assert_eq!(argv[0], "cargo");
        assert_eq!(argv[1], "run");
        let separator = argv.iter().position(|a| a == "--").expect("a `--`");
        assert_eq!(&argv[separator + 1..], &["--watch".to_string()]);
    }

    #[test]
    fn without_watch_the_app_is_started_plainly() {
        let argv = rust_run_argv(Path::new("/tmp/app/Cargo.toml"), true, false);
        assert!(!argv.iter().any(|a| a == "--watch"));
        assert!(argv.contains(&"--release".to_string()));
    }
}
