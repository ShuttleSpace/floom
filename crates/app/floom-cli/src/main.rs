// SPDX-License-Identifier: MIT
//! `floom` — command line tools for Floom.
//!
//! Stage 1 ships `floom create`, which scaffolds a Flutter-style project: a
//! Rust core under `src/` plus one directory per target platform.

mod native_project;
mod package;
mod pack;
mod run_app;
mod templates;
mod toolchain;
mod watch;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const USAGE: &str = "\
floom — command line tools for Floom

USAGE:
    floom <COMMAND> [OPTIONS]

COMMANDS:
    run <APP.tsx>    Compile a TSX app and launch it (no Rust written by hand)
    create <NAME>    Scaffold a new Floom app
    create --plugin  Scaffold a Floom plugin (Rust half + native halves + .d.ts)
    help             Show this message

    init <NAME>      Scaffold a new Floom app in the current directory
    check <DIR>      Type-check the project in <DIR> (default: .)
    test <DIR>       Run the project's tests in <DIR> (default: .)
    package <DIR>    Build a release binary (add --platform <P> for a platform artifact)
OPTIONS (run):
    --release              Build the app in release mode
    --no-run               Compile the app without launching it
    --features <LIST>      Backend features (default: desktop,renderer-gpu)
    --floom-path <PATH>    Use a local Floom checkout instead of installed crates
    --watch                Rebuild & relaunch on source changes (hot restart)
    --poll-ms <N>          Watch poll period in ms (default: 200)
    --debounce-ms <N>      Settle time before a rebuild fires (default: 100)
    --no-relaunch          Watch + rebuild only, no window (latency harness)

OPTIONS (create):
    --template <KIND>      app (default) or plugin
    --plugin               Shorthand for `--template plugin`
    --style <STYLE>        App flavour: rust (default, a Cargo project) or
                            tsx (one app.tsx; `floom run` builds it)
    --tsx                  Shorthand for `--style tsx`
    --platforms <LIST>     Comma-separated subset of:
                            android,ios,ohos,web,windows,linux,macos,tui
                            (default: all)
    --org <REVERSE_DNS>    Organisation for native projects, e.g. com.example
    --floom-crate <NAME>   Crate to depend on (default: floom)
    --floom-path <PATH>    Depend on a local checkout instead of a version
    --platform-path <PATH> Path to floom-platform, for a plugin's dependency
    --force                Overwrite an existing target directory

OPTIONS (check / test / package):
    --release              Release profile (package only)
    --features <LIST>      Cargo features (repeatable)
    --platform <NAME>      One platform to target (package only)
EXAMPLES:
    floom run app.tsx
    floom run app.tsx --release
    floom create my_app
    floom create my_app --style tsx
    floom create my_app --platforms android,ios
    floom create --plugin --platforms android,ios camera
    floom create --template plugin my_plugin --platforms ohos

    floom init my_app
    floom check
    floom test
    floom package";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let Some(cmd) = args.first() else {
        print!("{USAGE}");
        return Ok(());
    };
    match cmd.as_str() {
        "run" => run_app::run(&args[1..]),
        "create" => create(&args[1..]),
        "init" => init(&args[1..]),
        "check" => check(&args[1..]),
        "test" => test(&args[1..]),
        "package" => package::run(&args[1..]),
        "help" | "-h" | "--help" => {
            print!("{USAGE}");
            Ok(())
        }
        other => Err(format!("unknown command `{other}`\n\n{USAGE}")),
    }
}

/// Validate a package name: must be usable as a Rust crate name and a directory.
fn check_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("project name must not be empty".into());
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(format!(
            "project name `{name}` must start with a letter or underscore"
        ));
    }
    if let Some(bad) = name
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || *c == '_' || *c == '-'))
    {
        return Err(format!(
            "project name `{name}` contains `{bad}`; use letters, digits, `_` or `-`"
        ));
    }
    Ok(())
}

/// What `create` is scaffolding.
///
/// Two templates rather than one, because an app and a plugin have different
/// shapes and different dependents: an app owns a widget tree and depends on the
/// `floom` facade; a plugin owns a capability and depends on `floom-platform`,
/// and must not drag the widget stack in behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Template {
    App,
    Plugin,
}

impl Template {
    fn parse(kind: &str) -> Result<Self, String> {
        match kind {
            "app" => Ok(Template::App),
            "plugin" => Ok(Template::Plugin),
            other => Err(format!("unknown template `{other}`; known: app, plugin")),
        }
    }
}

/// How an app is written: a Rust crate (the default) or a single TSX file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppStyle {
    Rust,
    Tsx,
}

impl AppStyle {
    fn parse(word: &str) -> Result<Self, String> {
        match word {
            "rust" => Ok(AppStyle::Rust),
            "tsx" => Ok(AppStyle::Tsx),
            other => Err(format!("unknown style `{other}`; known: rust, tsx")),
        }
    }
}

#[derive(Debug)]
struct CreateOpts {
    name: String,
    template: Template,
    style: AppStyle,
    platforms: Vec<String>,
    org: String,
    floom_crate: String,
    floom_path: Option<String>,
    platform_path: Option<String>,
    force: bool,
}

fn parse_create(args: &[String]) -> Result<CreateOpts, String> {
    let mut name: Option<String> = None;
    let mut template = Template::App;
    let mut style = AppStyle::Rust;
    let mut style_set = false;
    let mut platforms: Option<Vec<String>> = None;
    let mut platforms_set = false;
    let mut org = String::from("com.example");
    let mut floom_crate = String::from("floom");
    let mut floom_path: Option<String> = None;
    let mut platform_path: Option<String> = None;
    let mut force = false;

    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        // Accept both `--flag value` and `--flag=value`.
        let (key, inline) = match a.split_once('=') {
            Some((k, v)) if k.starts_with("--") => (k, Some(v.to_string())),
            _ => (a, None),
        };
        let mut take_value = |what: &str| -> Result<String, String> {
            if let Some(v) = inline.clone() {
                return Ok(v);
            }
            i += 1;
            args.get(i)
                .cloned()
                .ok_or_else(|| format!("`{what}` requires a value"))
        };
        match key {
            "--platforms" => {
                let raw = take_value("--platforms")?;
                let mut list = Vec::new();
                for p in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                    if !templates::PLATFORMS.contains(&p) {
                        return Err(format!(
                            "unknown platform `{p}`; known: {}",
                            templates::PLATFORMS.join(", ")
                        ));
                    }
                    if !list.iter().any(|x| x == p) {
                        list.push(p.to_string());
                    }
                }
                if list.is_empty() {
                    return Err("--platforms selected no platforms".into());
                }
                platforms = Some(list);
                platforms_set = true;
            }
            "--template" | "-t" => {
                let kind = take_value("--template")?;
                template = Template::parse(&kind)?;
            }
            // Shorthand, and checked *after* `--template` so that an explicit
            // `--template` later on the line wins: `floom create -t app --plugin`
            // is a mistake, and the last flag stated is the one honoured, which is
            // what every other CLI does.
            "--plugin" => template = Template::Plugin,
            "--style" => {
                style = AppStyle::parse(&take_value("--style")?)?;
                style_set = true;
            }
            // Same last-one-wins rule as `--template`: an explicit `--style`
            // stated later on the line overrides the shorthand.
            "--tsx" => {
                style = AppStyle::Tsx;
                style_set = true;
            }
            "--org" => org = take_value("--org")?,
            "--floom-crate" => floom_crate = take_value("--floom-crate")?,
            "--floom-path" => floom_path = Some(take_value("--floom-path")?),
            "--platform-path" => platform_path = Some(take_value("--platform-path")?),
            "--force" => force = true,
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`\n\n{USAGE}"));
            }
            other => {
                if name.is_some() {
                    return Err(format!("unexpected argument `{other}`"));
                }
                name = Some(other.to_string());
            }
        }
        i += 1;
    }

    let name = name.ok_or_else(|| format!("`create` requires a project name\n\n{USAGE}"))?;
    check_name(&name)?;
    if template == Template::Plugin && style_set {
        return Err(
            "`--style` applies to apps; a plugin is a Rust crate with an `index.d.ts` facade"
                .into(),
        );
    }
    if style == AppStyle::Tsx && platforms_set {
        return Err(
            "`--platforms` applies to the Rust scaffold; a TSX app runs on `floom run`'s default backend"
                .into(),
        );
    }
    Ok(CreateOpts {
        name,
        template,
        style,
        platforms: platforms
            .unwrap_or_else(|| templates::PLATFORMS.iter().map(|s| s.to_string()).collect()),
        org,
        floom_crate,
        floom_path,
        platform_path,
        force,
    })
}

fn write(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    fs::write(path, contents).map_err(|e| format!("{}: {e}", path.display()))
}

fn create(args: &[String]) -> Result<(), String> {
    let opts = parse_create(args)?;
    let root = PathBuf::from(&opts.name);

    if root.exists() && !opts.force {
        let mut entries = fs::read_dir(&root).map_err(|e| format!("{}: {e}", root.display()))?;
        if entries.next().is_some() {
            return Err(format!(
                "`{}` already exists and is not empty (use --force to overwrite)",
                root.display()
            ));
        }
    }
    fs::create_dir_all(&root).map_err(|e| format!("{}: {e}", root.display()))?;

    match opts.template {
        Template::App => create_app(&opts, &root, &opts.name)?,
        Template::Plugin => create_plugin(&opts, &root)?,
    }
    Ok(())
}

/// Scaffold an app in the current directory, named `name`. Shared by `create`
/// (which points `root` at a fresh subdirectory) and `init` (which points it at
/// `.`); the app's own name is passed separately so it can differ from the path.
fn create_app(opts: &CreateOpts, root: &Path, name: &str) -> Result<(), String> {
    match opts.style {
        AppStyle::Rust => create_rust_app(opts, root, name),
        AppStyle::Tsx => create_tsx_app(opts, root, name),
    }
}

/// Scaffold a Floom app into `.`, reusing the same template `create` uses but
/// without making a subdirectory. Fails if the directory already holds a
/// `Cargo.toml` (unless `--force`), so it never clobbers an existing project.
fn init(args: &[String]) -> Result<(), String> {
    let opts = parse_create(args)?;
    let root = PathBuf::from(".");
    if root.join("Cargo.toml").exists() && !opts.force {
        return Err(
            "current directory already has a Cargo.toml (use --force to overwrite)".to_string(),
        );
    }
    fs::create_dir_all(&root).map_err(|e| format!("{}: {e}", root.display()))?;
    match opts.template {
        Template::App => create_app(&opts, &root, &opts.name)?,
        Template::Plugin => create_plugin(&opts, &root)?,
    }
    Ok(())
}

/// Scaffold a TSX app: one `app.tsx`, the npm metadata an editor needs, and no
/// Rust at all — `floom run` writes the throwaway crate.
fn create_tsx_app(_opts: &CreateOpts, root: &Path, name: &str) -> Result<(), String> {
    write(
        &root.join("app.tsx"),
        &templates::APP_TSX.replace("{{TITLE}}", name),
    )?;
    write(
        &root.join("package.json"),
        &templates::tsx_package_json(name),
    )?;
    write(&root.join("tsconfig.json"), templates::TSX_TSCONFIG)?;
    write(&root.join(".gitignore"), templates::TSX_GITIGNORE)?;
    write(&root.join("README.md"), &templates::tsx_root_readme(name))?;

    println!("Created Floom TSX app `{}` in `{}`", name, root.display());
    println!("  style: tsx (editor types from @types/floom)");
    println!();
    println!("Next:");
    println!("  cd {}", root.display());
    println!("  npm install    # editor completion and `npm run typecheck`");
    println!("  floom run app.tsx");
    Ok(())
}

/// Scaffold a Rust app: a widget tree, plus one directory per platform.
fn create_rust_app(opts: &CreateOpts, root: &Path, name: &str) -> Result<(), String> {
    let mobile = opts
        .platforms
        .iter()
        .any(|p| matches!(p.as_str(), "android" | "ios" | "ohos"));
    write(
        &root.join("Cargo.toml"),
        &templates::cargo_toml(
            &opts.name,
            &opts.floom_crate,
            opts.floom_path.as_deref(),
            mobile,
        ),
    )?;
    write(
        &root.join("src/main.rs"),
        &templates::MAIN_RS
            .replace("{{TITLE}}", name)
            .replace("{{FLOOM}}", &opts.floom_crate),
    )?;
    if mobile {
        write(&root.join("src/lib.rs"), templates::MOBILE_LIB_RS)?;
    }
    write(
        &root.join("tests/smoke.rs"),
        "// Integration tests go here. Unit tests belong next to the code in `src/`.\n",
    )?;

    // At the top level, next to `src/`, not under a `native/` directory: this
    // matches a Flutter project, where `android/` sits beside `lib/` and opens in
    // Android Studio without the reader having to know there is a hierarchy.
    for p in &opts.platforms {
        emit_app_platform(opts, root, p)?;
    }

    write(
        &root.join("README.md"),
        &templates::root_readme(name, &opts.platforms),
    )?;

    println!("Created Floom app `{}` in `{}`", name, root.display());
    println!("  organisation: {}", opts.org);
    println!("  platforms:    {}", opts.platforms.join(", "));
    println!();
    println!("Next:");
    println!("  cd {}", root.display());
    println!("  cargo run");
    Ok(())
}

/// Scaffold a plugin: a Rust capability, plus one native half per platform.
///
/// The layout follows a Flutter plugin package — `android/` and `ios/` beside
/// `src/`, an `example/`, and a README that says which half is which — with one
/// difference that changes the whole build: there is no `pluginClass` to
/// register and no method channel to serialize through. `build.rs` links the
/// native half into the same binary as `src/lib.rs`, so the app calls
/// `extern "C"` directly and the plugin is resolved at link time.
fn create_plugin(opts: &CreateOpts, root: &Path) -> Result<(), String> {
    let android = opts.platforms.iter().any(|p| p == "android");

    write(
        &root.join("Cargo.toml"),
        &templates::plugin_cargo_toml(&opts.name, opts.platform_path.as_deref(), &opts.platforms),
    )?;
    write(
        &root.join("build.rs"),
        &templates::plugin_build_rs(&opts.name),
    )?;
    write(
        &root.join("src/lib.rs"),
        &templates::plugin_lib_rs(&opts.name, android),
    )?;
    if android {
        write(
            &root.join("src/android.rs"),
            &templates::plugin_android_rs(&opts.name),
        )?;
    }

    // The C ABI sits at `native/common/`, beside `src/` and every platform
    // directory, because it belongs to the plugin rather than to any one
    // platform: each platform's code implements it, and `build.rs` reads it to
    // find the sources that implement it. Written once, outside the platform
    // loop.
    //
    // The path is `native/common/floom_<name>.h` and not `floom.h` at the root:
    // `build.rs` watches `native/` as a whole and every platform's source
    // `#include`s this header by that name, so a header that lived somewhere
    // else would be included by a path that does not exist.
    let common_dir = root.join("native").join("common");
    fs::create_dir_all(&common_dir).map_err(|e| format!("{}: {e}", common_dir.display()))?;
    write(
        &common_dir.join(format!("floom_{}.h", opts.name.replace('-', "_"))),
        &templates::plugin_header_rs(&opts.name),
    )?;

    // The framework's provider ABI, for a plugin whose capability is a native
    // view. It sits beside the plugin's own header rather than replacing it:
    // the two are different contracts, and a plugin that contributes a native
    // view implements both.
    write(
        &common_dir.join("floom_native_provider.h"),
        templates::PROVIDER_HEADER,
    )?;

    // The reference desktop native half. It is what makes the generated
    // plugin compile, link and pass its tests on the machine that generated
    // it: the C ABI is proven to work before there is any real capability
    // behind it.
    let desktop_dir = root.join("native").join("desktop");
    fs::create_dir_all(&desktop_dir).map_err(|e| format!("{}: {e}", desktop_dir.display()))?;
    write(
        &desktop_dir.join(format!("floom_{}.c", opts.name.replace('-', "_"))),
        &templates::plugin_native_c(&opts.name),
    )?;
    write(
        &desktop_dir.join("README.md"),
        &templates::plugin_desktop_readme(&opts.name),
    )?;

    // Under `native/`, next to the `common/` directory that holds the C ABI.
    // The desktop half is written above rather than in this loop, because it
    // is *one* directory for all three desktop targets: the C ABI it
    // implements is the same, and a per-OS copy would be three files that
    // differ only in a `#ifdef`.
    for p in &opts.platforms {
        emit_plugin_platform(opts, root, p)?;
    }

    let example = root.join("example");
    write(
        &example.join("Cargo.toml"),
        &templates::plugin_example_cargo_toml(
            &opts.name,
            &opts.floom_crate,
            opts.floom_path.as_deref(),
            opts.platform_path.as_deref(),
        ),
    )?;
    write(
        &example.join("src/main.rs"),
        &templates::plugin_example_main_rs(&opts.name, &opts.floom_crate),
    )?;

    write(
        &root.join("README.md"),
        &templates::plugin_root_readme(&opts.name, &opts.platforms),
    )?;

    // The TypeScript facade sits at the package root, beside the manifest, so
    // an editor resolves `floom-plugin-<name>` from the package it belongs to
    // rather than from a path inside the Rust crate.
    write(&root.join("index.d.ts"), &templates::plugin_dts(&opts.name))?;

    println!(
        "Created Floom plugin `{}` in ./{}",
        opts.name,
        root.display()
    );
    println!("  crate:        floom-plugin-{}", opts.name);
    println!("  platforms:    {}", opts.platforms.join(", "));
    println!("  types:        index.d.ts");
    println!("  organisation: {}", opts.org);
    println!();
    println!("Next:");
    println!("  cd {}/example && cargo run", opts.name);
    println!();
    println!("Wire it into an app:");
    println!(
        "  floom-plugin-{} = {{ path = \"path/to/{}\" }}",
        opts.name, opts.name
    );
    println!("  floom_platform::plugin(\"{}\").enable()?;", opts.name);
    Ok(())
}
/// Run a cargo subcommand inside `dir`, forwarding `--features` when given.
fn cargo_in(dir: &Path, subcmd: &str, features: &[String], release: bool) -> Result<(), String> {
    let mut cmd = Command::new("cargo");
    cmd.arg(subcmd).current_dir(dir);
    if release {
        cmd.arg("--release");
    }
    for f in features {
        cmd.arg("--features").arg(f);
    }
    let status = cmd
        .status()
        .map_err(|e| format!("cannot run `cargo {subcmd}`: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("`cargo {subcmd}` failed ({status})"))
    }
}

/// Shared options for the project-lifecycle commands: a target directory plus
/// the `--release` / `--features` / `--platform` flags `check`, `test`, and
/// `package` all accept.
struct LifecycleOpts {
    dir: PathBuf,
    release: bool,
    features: Vec<String>,
    platform: Option<String>,
}

fn parse_lifecycle(args: &[String]) -> Result<LifecycleOpts, String> {
    let mut dir = PathBuf::from(".");
    let mut release = false;
    let mut features = Vec::new();
    let mut platform = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--release" => release = true,
            "--features" => {
                i += 1;
                let v = args
                    .get(i)
                    .ok_or("`--features` requires a value")?
                    .to_string();
                features.push(v);
            }
            "--platform" => {
                i += 1;
                platform = Some(
                    args.get(i)
                        .ok_or("`--platform` requires a value")?
                        .to_string(),
                );
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unknown option `{flag}` for this command"));
            }
            path => {
                if dir != Path::new(".") {
                    return Err("this command takes one directory argument".into());
                }
                dir = PathBuf::from(path);
            }
        }
        i += 1;
    }
    Ok(LifecycleOpts {
        dir,
        release,
        features,
        platform,
    })
}

/// `floom check <DIR>` — type-check the project without producing a binary.
fn check(args: &[String]) -> Result<(), String> {
    let opts = parse_lifecycle(args)?;
    cargo_in(&opts.dir, "check", &opts.features, false)
}

/// `floom test <DIR>` — run the project's test suite.
fn test(args: &[String]) -> Result<(), String> {
    let opts = parse_lifecycle(args)?;
    cargo_in(&opts.dir, "test", &opts.features, false)
}

/// Writes the native host project for one platform of an **app**.
///
/// Each platform gets a directory the platform's own toolchain recognises, so
/// "open it" works: `android/` is a Gradle project Android Studio opens,
/// `windows/` is a CMake project Visual Studio opens with the CMake
/// integration. A platform with nothing to open — `tui` is a terminal, and `web`
/// is served statically — gets what it actually needs and a note saying what the
/// note is.
fn emit_app_platform(opts: &CreateOpts, root: &Path, platform: &str) -> Result<(), String> {
    let dir = root.join(platform);
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let lib = native_project::lib_name(&opts.name);

    let (_, label, opener) = templates::PLATFORM_NOTES
        .iter()
        .find(|(k, _, _)| *k == platform)
        .copied()
        .unwrap_or((platform, platform, "an editor"));

    match platform {
        "android" => {
            write(
                &dir.join("settings.gradle.kts"),
                &native_project::android_app_settings_gradle(&opts.org, &opts.name),
            )?;
            write(
                &dir.join("build.gradle.kts"),
                &native_project::android_root_build_gradle(),
            )?;
            write(
                &dir.join("gradle.properties"),
                &native_project::android_gradle_properties(),
            )?;
            write(
                &dir.join("gradle/wrapper/gradle-wrapper.properties"),
                &native_project::android_gradle_wrapper_properties(),
            )?;
            write(&dir.join(".gitignore"), native_project::ANDROID_GITIGNORE)?;
            write(
                &dir.join("app/proguard-rules.pro"),
                native_project::ANDROID_PROGUARD_RULES,
            )?;
            write(
                &dir.join("app/build.gradle.kts"),
                &native_project::android_app_build_gradle(&opts.org, &opts.name, &lib),
            )?;
            write(
                &dir.join("app/src/main/AndroidManifest.xml"),
                &native_project::android_app_manifest(&opts.org, &opts.name),
            )?;
            write(
                // The file name is the class name. Kotlin and Java both require a
                // public class to live in a file named after it, and this is not
                // checked until the Kotlin plugin compiles — so a mismatch is a
                // build error far from the file that caused it.
                &dir.join(format!(
                    "app/src/main/kotlin/{}/{}Activity.kt",
                    native_project::kotlin_package_path(&native_project::android_app_id(
                        &opts.org, &opts.name
                    )),
                    native_project::kotlin_class_name(&opts.name),
                )),
                &native_project::android_main_activity(&opts.org, &opts.name, &lib),
            )?;
            write(
                &dir.join("app/src/main/res/values/strings.xml"),
                &native_project::android_strings_xml(&opts.name),
            )?;
            write(
                &dir.join("app/src/main/res/values/styles.xml"),
                &native_project::android_styles_xml(&opts.name),
            )?;
            write(
                &dir.join("app/src/main/res/values/colors.xml"),
                &native_project::android_colors_xml(),
            )?;
            write(
                &dir.join("app/src/main/res/mipmap-anydpi-v26/ic_launcher.xml"),
                &native_project::android_launcher_icon_xml(),
            )?;
            write(
                &dir.join("app/src/main/res/drawable/ic_launcher_foreground.xml"),
                native_project::ANDROID_LAUNCHER_FOREGROUND_XML,
            )?;
            // `local.properties` is machine-specific and gitignored, so it is
            // written only when an SDK was actually found — and its absence is
            // explained by the README rather than left to be discovered.
            match native_project::detect_android_sdk() {
                Some(sdk) => write(
                    &dir.join("local.properties"),
                    &native_project::android_local_properties(&sdk),
                )?,
                None => write(
                    &dir.join("local.properties.example"),
                    "# Copy to local.properties and point sdk.dir at your SDK.\n\
                     # Android Studio writes this file itself on first open.\n",
                )?,
            }
        }
        // macOS is a desktop target: an AppKit host built by CMake, which an IDE
        // opens and `cmake` builds. Not the same as `ios/`, which needs a bundle
        // id and a provisioning profile and therefore an Xcode project.
        "macos" => {
            write(&dir.join("include/floom.h"), native_project::DESKTOP_HOST_H)?;
            write(&dir.join("src/main.m"), &templates::ios_main_m(&opts.name))?;
            write(
                &dir.join("CMakeLists.txt"),
                &native_project::desktop_cmake("macos", &opts.name, &lib),
            )?;
        }
        "ios" => {
            // An iOS app is a bundle, not a binary: it needs an `Info.plist`, a
            // bundle id and a build configuration that can be signed. That is an
            // Xcode project, and it is emitted as one — see `ios/README.md` for
            // what to open and `build` for the phase that generates the pbxproj.
            write(
                &dir.join("Info.plist"),
                &templates::ios_info_plist(&opts.org, &opts.name),
            )?;
            write(&dir.join("include/floom.h"), native_project::DESKTOP_HOST_H)?;
            write(
                &dir.join("src/AppDelegate.swift"),
                &templates::ios_app_delegate_swift(&opts.name),
            )?;
            write(
                &dir.join("src/ViewController.swift"),
                &templates::ios_view_controller_swift(&opts.name),
            )?;
        }
        p @ ("windows" | "linux") => {
            write(&dir.join("include/floom.h"), native_project::DESKTOP_HOST_H)?;
            write(
                &dir.join("src/main.c"),
                &templates::desktop_main_c(p, &opts.name),
            )?;
            write(
                &dir.join("CMakeLists.txt"),
                &native_project::desktop_cmake(p, &opts.name, &lib),
            )?;
        }
        "web" => {
            write(
                &dir.join("index.html"),
                &templates::INDEX_HTML.replace("{{PROJECT}}", &opts.name),
            )?;
        }
        "ohos" => {
            write(
                &dir.join("build-profile.json5"),
                &templates::ohos_build_profile(&opts.name),
            )?;
            write(
                &dir.join("entry/src/main/module.json5"),
                &templates::ohos_module_json(&opts.name),
            )?;
            write(
                &dir.join("entry/src/main/ets/entryability/EntryAbility.ets"),
                &templates::ohos_entry_ability(&opts.name),
            )?;
            write(
                &dir.join("oh-package.json5"),
                &templates::ohos_package_json(&opts.name),
            )?;
        }
        _ => {}
    }

    // Every platform gets a README, but after its project files rather than
    // instead of them: it now explains how to build a thing that exists.
    write(
        &dir.join("README.md"),
        &templates::platform_readme(label, opener),
    )
}

/// Writes the native half of one platform of a **plugin**.
///
/// The difference from an app's platform is that a plugin is a *library* on every
/// platform: a Gradle library, a podspec, a CMake static library. It never owns
/// an activity, a bundle id or an `Info.plist`, because the host owns those, and
/// emitting them would make two plugins fight over the host's identity.
///
/// # Why `native/<platform>/` and not `<platform>/`
///
/// The layout mirrors the plugin's own README and the path `build.rs` looks
/// in: the native half for a platform lives under `native/`, next to the
/// `common/` directory that holds the C ABI. A plugin that put its platform
/// directories at the root would have a `build.rs` that compiles
/// `native/desktop/` and a `src/lib.rs` that a reader cannot connect to the
/// Kotlin file three directories away.
fn emit_plugin_platform(opts: &CreateOpts, root: &Path, platform: &str) -> Result<(), String> {
    // Android's half is a Gradle *library* beside `src/`, the Flutter plugin
    // shape: `android/build.gradle.kts`, `android/src/main/java/…`, and the
    // manifest that declares the plugin's permissions. The Rust edge is
    // `src/android.rs`; there is no `native/` directory for it.
    let dir = if platform == "android" {
        root.join("android")
    } else {
        root.join("native").join(platform)
    };
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let lib = native_project::lib_name(&opts.name);

    // A platform with a native half of its own gets a README that explains
    // that half; the generic one is written first and this one second, so the
    // specific text is the one that survives.
    let mut specific_readme: Option<String> = None;

    match platform {
        "android" => {
            write(
                &dir.join("settings.gradle.kts"),
                &native_project::android_plugin_settings_gradle(&opts.name),
            )?;
            // A plugin project is a single Gradle module: `build.gradle.kts` is
            // both the root and the library, so it is written once, and it
            // carries the `repositories` block itself.
            write(
                &dir.join("build.gradle.kts"),
                &native_project::android_plugin_build_gradle(&opts.name, &lib),
            )?;
            write(
                &dir.join("gradle.properties"),
                &native_project::android_gradle_properties(),
            )?;
            write(
                &dir.join("gradle/wrapper/gradle-wrapper.properties"),
                &native_project::android_gradle_wrapper_properties(),
            )?;
            write(&dir.join(".gitignore"), native_project::ANDROID_GITIGNORE)?;
            write(
                &dir.join("src/main/AndroidManifest.xml"),
                &native_project::android_plugin_manifest(&opts.name),
            )?;
            let class = native_project::android_java_class(&opts.name);
            let package_dir = format!("dev/floom/plugin/{}", opts.name.replace('-', "_"));
            write(
                &dir.join(format!(
                    "src/main/java/{package_dir}/{}Plugin.java",
                    class.strip_suffix("Plugin").unwrap_or(&class)
                )),
                &native_project::android_plugin_java(&opts.name),
            )?;
            specific_readme = Some(templates::plugin_platform_readme(
                "Android",
                "Android Studio",
            ));
        }
        "ios" => {
            write(
                &dir.join(format!("floom-plugin-{}.podspec", opts.name)),
                &native_project::ios_podspec(&opts.name),
            )?;
            write(
                // File name matches the class name, as in the Android half.
                &dir.join(format!(
                    "Classes/{}Plugin.swift",
                    native_project::kotlin_class_name(&opts.name)
                )),
                &native_project::ios_plugin_swift(&opts.name),
            )?;
            write(
                &dir.join(format!("include/{}-Bridging-Header.h", opts.name)),
                &native_project::ios_bridging_header(&opts.name),
            )?;
        }
        // The desktop half is C, and `build.rs` compiles it — so there is no
        // CMake project for it. Two compilers over the same source is how a
        // plugin ends up with duplicate symbols at link time, and the CMake
        // would have had to produce an *executable* to match the app template,
        // which is the wrong shape for a library.
        "windows" | "linux" | "macos" => {}
        "web" => {
            // The web's native half is JavaScript, imported by the wasm
            // module rather than linked into it.
            let js_dir = dir.join("js");
            fs::create_dir_all(&js_dir).map_err(|e| format!("{}: {e}", js_dir.display()))?;
            write(
                &js_dir.join(format!("floom_{}.js", opts.name.replace('-', "_"))),
                &templates::plugin_web_js(&opts.name),
            )?;
            specific_readme = Some(templates::plugin_web_readme(&opts.name));
        }
        "ohos" => {
            // OpenHarmony's native half is ArkTS, reached through a C shim
            // that implements the same C ABI as every other platform.
            let ets_dir = dir.join("ets");
            fs::create_dir_all(&ets_dir).map_err(|e| format!("{}: {e}", ets_dir.display()))?;
            write(
                &ets_dir.join(format!(
                    "{}Plugin.ets",
                    native_project::kotlin_class_name(&opts.name)
                )),
                &templates::plugin_ohos_ets(&opts.name),
            )?;
            write(
                &dir.join(format!("floom_{}_ohos.c", opts.name.replace('-', "_"))),
                &templates::plugin_ohos_c_shim(&opts.name),
            )?;
            // The ArkTS half is an ArkTS package, so the host's HAP can depend
            // on it the way it depends on any other.
            write(
                &dir.join("oh-package.json5"),
                &templates::ohos_package_json(&opts.name),
            )?;
            specific_readme = Some(templates::plugin_ohos_readme(&opts.name));
        }
        "tui" => {
            // A terminal has no OS capability surface, so there is no native
            // half to write. The README says so, which is more useful than a
            // file that would have to explain itself.
            specific_readme = Some(templates::plugin_tui_readme(&opts.name));
        }
        _ => {}
    }

    let (_, label, opener) = templates::PLATFORM_NOTES
        .iter()
        .find(|(k, _, _)| *k == platform)
        .copied()
        .unwrap_or((platform, platform, "an editor"));
    write(
        &dir.join("README.md"),
        &templates::plugin_platform_readme(label, opener),
    )?;
    if let Some(readme) = specific_readme {
        write(&dir.join("README.md"), &readme)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The repository root, found from this crate's manifest directory.
    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .expect("crates/app/floom-cli sits three levels below the root")
            .to_path_buf()
    }

    /// Scaffold a plugin into a scratch directory and return it.
    fn scaffold(name: &str, platforms: &[&str]) -> PathBuf {
        let root = repo_root();
        let dir = root.join("tests").join(format!("floom-cli-test-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch directory");

        let opts = CreateOpts {
            name: name.to_string(),
            template: Template::Plugin,
            style: AppStyle::Rust,
            platforms: platforms.iter().map(|s| s.to_string()).collect(),
            org: "com.example".to_string(),
            floom_crate: "floom".to_string(),
            floom_path: Some(root.join("crates").join("floom").display().to_string()),
            platform_path: Some(
                root.join("crates")
                    .join("floom-platform")
                    .display()
                    .to_string(),
            ),
            force: true,
        };
        create_plugin(&opts, &dir).expect("scaffold the plugin");
        dir
    }

    fn read(dir: &Path, rel: &str) -> String {
        fs::read_to_string(dir.join(rel))
            .unwrap_or_else(|e| panic!("{}: {e}", dir.join(rel).display()))
    }

    /// The four entry points the C ABI is made of.
    const ABI_SYMBOLS: &[&str] = &["version", "enable", "disable", "capability"];

    /// Whether a Kotlin source *declares* an `external fun`.
    ///
    /// Only declarations count, not the prose: the generated Kotlin explains
    /// at length why an `external fun` would be the wrong contract, and a
    /// check that cannot tell an explanation from a declaration would have to
    /// be deleted rather than fixed.
    fn declares_external_fun(source: &str) -> bool {
        source.lines().any(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("//")
                && !trimmed.starts_with('*')
                && trimmed.contains("external fun")
        })
    }

    #[test]
    fn the_c_abi_header_is_the_one_the_native_half_includes() {
        // The header is the contract, and every native half includes it by the
        // name `build.rs` puts on the include path. A mismatch here is a
        // compile error in the generated project, reported from the C
        // compiler and naming a header nobody can find.
        let dir = scaffold("abiheader", &["macos"]);
        let source = read(&dir, "native/desktop/floom_abiheader.c");

        assert!(
            source.contains("#include \"floom_abiheader.h\""),
            "the desktop half must include the header by the name it is written under"
        );
    }

    #[test]
    fn the_header_and_every_native_half_agree_on_the_symbol_names() {
        // This is the check that catches the failure mode the whole design
        // exists to prevent: two halves that each name the same capability
        // differently. The header declares the symbols; the C half, the Android
        // shim and the Swift half define them. If any pair disagrees, the
        // plugin links against symbols that do not exist — and the failure
        // appears as an undefined symbol at link time, on a machine that is
        // not the one that wrote the code.
        let dir = scaffold("symbols", &["ios"]);
        let header = read(&dir, "native/common/floom_symbols.h");
        let desktop = read(&dir, "native/desktop/floom_symbols.c");
        let swift = read(&dir, "native/ios/Classes/SymbolsPlugin.swift");

        for sym in ABI_SYMBOLS {
            let symbol = format!("floom_symbols_{sym}");
            assert!(header.contains(&symbol), "the header must declare {symbol}");
            assert!(
                desktop.contains(&symbol),
                "the desktop half must define {symbol}"
            );
            assert!(
                swift.contains(&symbol),
                "the Swift half must export {symbol}"
            );
        }

        // The ABI version is a function as well as a macro, because the Rust
        // half has to be able to ask a plugin it did not compile what it was
        // built against.
        assert!(header.contains("floom_symbols_abi_version"));
        assert!(desktop.contains("floom_symbols_abi_version"));
        assert!(swift.contains("floom_symbols_abi_version"));
    }

    #[test]
    fn the_rust_half_calls_the_abi_behind_the_flag_build_rs_emits() {
        // The extern block is behind `floom_plugin_native`, and `build.rs` is
        // what emits it. If either side stops agreeing, the plugin either fails
        // to link or reaches for symbols that are not there.
        let dir = scaffold("cfg", &["macos"]);
        let build_rs = read(&dir, "build.rs");
        let lib_rs = read(&dir, "src/lib.rs");

        assert!(
            build_rs.contains("cargo:rustc-cfg=floom_plugin_native"),
            "build.rs must emit the flag the Rust half gates on"
        );
        assert!(
            build_rs.contains("cargo:rustc-check-cfg=cfg(floom_plugin_native)"),
            "build.rs must declare the flag, or every build warns about an unknown cfg"
        );
        assert!(
            lib_rs.contains("#[cfg(floom_plugin_native)]"),
            "the extern block must be behind the flag"
        );
        assert!(
            !lib_rs.contains("cc::Build::new()"),
            "the Rust half must not compile anything; build.rs does that"
        );
        assert!(
            build_rs.contains("cc::Build::new()"),
            "build.rs is what compiles the native half"
        );
    }

    #[test]
    fn the_android_half_is_java_and_the_rust_edge_calls_it_over_jni() {
        // The capability is Java, compiled into the host's dex, and the Rust
        // half is a JNI call: JNI *is* the FFI here, and a C shim would be the
        // same JNI code moved into a third language with no isolation gained.
        let dir = scaffold("kotlin", &["android"]);
        let java = read(
            &dir,
            "android/src/main/java/dev/floom/plugin/kotlin/KotlinPlugin.java",
        );
        let android_rs = read(&dir, "src/android.rs");
        let lib_rs = read(&dir, "src/lib.rs");
        let cargo_toml = read(&dir, "Cargo.toml");
        let build_rs = read(&dir, "build.rs");
        let gradle = read(&dir, "android/build.gradle.kts");

        assert!(
            !declares_external_fun(&java),
            "the Java half must not declare an `external fun`: that is the \
             opposite contract, and the JNI call is what reaches it"
        );
        assert!(
            java.contains("public static boolean enable(Context context)"),
            "the Java half has to be reachable as a static method"
        );
        assert!(java.contains("public static void disable()"));

        assert!(
            android_rs.contains("bridge::app_class"),
            "the Rust edge must resolve the class through the app's class loader"
        );
        assert!(
            android_rs.contains("dev/floom/plugin/kotlin/KotlinPlugin"),
            "the class name the Rust side looks up"
        );
        assert!(
            android_rs.contains("jni::jni_str!(\"enable\")"),
            "the edge must call the Java half's enable"
        );

        assert!(
            lib_rs.contains("#[cfg(target_os = \"android\")]"),
            "the Rust half has to be split from the C half on android"
        );
        assert!(lib_rs.contains("android::enable"));
        assert!(lib_rs.contains("android::disable"));
        assert!(lib_rs.contains("return Some(CAPABILITY_ID)"));

        assert!(
            cargo_toml.contains("floom-platform-mobile"),
            "the android edge needs the JNI bridge from the platform layer"
        );
        assert!(
            cargo_toml.contains("jni = \"0.22\""),
            "the edge drives the JVM through the jni crate"
        );

        assert!(
            build_rs.contains("if target_os == \"android\""),
            "build.rs must let the Java half through without looking for C sources"
        );
        assert!(gradle.contains("com.android.library"));
        assert!(
            !gradle.contains("org.jetbrains.kotlin.android"),
            "the java half means no Kotlin plugin: d8 consumes javac output directly"
        );
        assert!(!dir.join("native/android").exists());
    }

    #[test]
    fn the_swift_half_exports_the_c_abi() {
        // `@objc static func` is an Objective-C method, not a C symbol: the
        // Rust half's `extern "C"` declaration would not resolve against it.
        let dir = scaffold("swift", &["ios"]);
        let swift = read(&dir, "native/ios/Classes/SwiftPlugin.swift");

        assert!(
            swift.contains("@_cdecl(\"floom_swift_enable\")"),
            "the Swift half must export the C ABI with `@_cdecl`"
        );
    }

    #[test]
    fn the_web_half_is_a_javascript_module_the_wasm_imports() {
        // The web is the one platform where the native half cannot be
        // linked: there is no native library to link, so the wasm module
        // imports JavaScript. The import path and the file on disk have to
        // agree, and the names have to be the ABI's names.
        let dir = scaffold("web", &["web"]);
        let js = read(&dir, "native/web/js/floom_web.js");
        let lib_rs = read(&dir, "src/lib.rs");

        for sym in ABI_SYMBOLS {
            let symbol = format!("floom_web_{sym}");
            assert!(
                js.contains(&format!("export function {symbol}(")),
                "the JavaScript half must export {symbol}"
            );
        }
        assert!(
            js.contains("export function floom_web_abi_version("),
            "the ABI version is a function too, not only a macro"
        );

        // The import path in the Rust half, resolved relative to the crate
        // root, has to be the file that was written.
        assert!(
            lib_rs.contains("/native/web/js/floom_web.js"),
            "the wasm import must name the module that was written:\n{lib_rs}"
        );
        assert!(
            dir.join("native/web/js/floom_web.js").is_file(),
            "the module the import names must exist"
        );
    }

    #[test]
    fn the_ohos_half_is_arkts_behind_a_c_shim() {
        // OpenHarmony's OS services are reachable only from ArkTS, so the
        // capability is written in ArkTS. The C ABI is implemented by a shim
        // next to it, which reaches ArkTS through the one helper Rust
        // exports.
        let dir = scaffold("ohos", &["ohos"]);
        let ets = read(&dir, "native/ohos/ets/OhosPlugin.ets");
        let shim = read(&dir, "native/ohos/floom_ohos_ohos.c");
        let lib_rs = read(&dir, "src/lib.rs");

        for sym in ABI_SYMBOLS {
            let symbol = format!("floom_ohos_{sym}");
            assert!(shim.contains(&symbol), "the C shim must implement {symbol}");
        }
        assert!(
            shim.contains("floom_ohos_arkts_call"),
            "the shim reaches ArkTS through the Rust-exported helper"
        );
        assert!(
            lib_rs.contains("floom_ohos_arkts_call"),
            "Rust must export the helper the shim calls:\n{lib_rs}"
        );
        assert!(
            ets.contains("export class OhosPlugin"),
            "the capability is written in ArkTS"
        );
    }

    #[test]
    fn the_terminal_half_says_there_is_none() {
        // A terminal has no OS capability surface. The half says so, which is
        // more useful than a file that would have to explain itself.
        let dir = scaffold("tui", &["tui"]);
        let readme = read(&dir, "native/tui/README.md");

        assert!(
            readme.contains("no native half"),
            "the terminal half must say what the platform does not have:\n{readme}"
        );
        assert!(
            !dir.join("native/tui").join("floom_tui.c").exists(),
            "there is no native half to write for a terminal"
        );
    }

    #[test]
    fn the_provider_header_the_plugin_gets_is_the_crates_own() {
        // The CLI embeds the framework's provider ABI rather than depending on
        // the crate that owns it, because the CLI is dependency-free on
        // purpose. An embedded copy can drift, so it is checked against the
        // original: a plugin that implements a stale header would fail to
        // link, on a machine that is not the one that generated it.
        let real = include_str!("../../../native/floom-native-ffi/include/floom_native_provider.h");
        assert_eq!(
            templates::PROVIDER_HEADER,
            real,
            "the CLI's copy of the provider header has drifted from the crate's"
        );

        let dir = scaffold("providerheader", &["macos"]);
        let written = read(&dir, "native/common/floom_native_provider.h");
        assert_eq!(
            written, real,
            "the header the plugin gets is the crate's, byte for byte"
        );
    }

    #[test]
    fn the_generated_plugin_is_its_own_workspace() {
        // A plugin is developed on its own. Without an empty `[workspace]` in
        // its manifest, generating it inside a checkout makes cargo walk up,
        // find the enclosing workspace and refuse to build it.
        let dir = scaffold("workspace", &["macos"]);
        let manifest = read(&dir, "Cargo.toml");

        assert!(
            manifest.contains("[workspace]"),
            "the plugin manifest must declare an empty [workspace]"
        );
        assert!(
            manifest.contains("cc = \"1\""),
            "build.rs compiles the native half, so `cc` is a build dependency"
        );
    }

    #[test]
    fn the_generated_plugin_carries_a_typescript_facade() {
        // The Rust half is the implementation, but a TSX or web host reads the
        // package through its declarations. A scaffold without them is a
        // package an editor cannot complete.
        let dir = scaffold("dts", &["macos"]);
        let dts = read(&dir, "index.d.ts");

        assert!(
            dts.contains("declare module \"floom-plugin-dts\""),
            "the facade must declare the package it belongs to:\n{dts}"
        );
        for name in ["version", "enable", "disable", "capability", "register"] {
            assert!(
                dts.contains(&format!("export function {name}(")),
                "missing `{name}` in the facade:\n{dts}"
            );
        }
    }

    #[test]
    fn the_generated_facade_mirrors_the_generated_rust_api() {
        // The facade claims to mirror the crate's public Rust API name for
        // name. A scaffold that breaks the claim hands every new plugin a
        // facade that is already wrong on the day it is written: the API a
        // TypeScript author sees is not the API the crate has.
        //
        // The names are taken from the scaffold's own Rust half rather than
        // written out here, so the check follows the scaffold when it grows.
        let dir = scaffold("mirror", &["macos"]);
        let rust = read(&dir, "src/lib.rs");
        let dts = read(&dir, "index.d.ts");

        let mut declared = 0usize;
        for line in rust.lines() {
            // Only the crate root is the API. A `pub` item inside a module --
            // the scaffold's private `ffi` block, its `extern` declarations --
            // is not something a caller can reach, so it is not part of what
            // the facade has to name.
            if !line.starts_with("pub ") {
                continue;
            }
            let name = if let Some(rest) = line.strip_prefix("pub fn ") {
                rest.split(['(', ' ']).next().unwrap_or_default()
            } else if let Some(rest) = line.strip_prefix("pub const ") {
                rest.split([':', ' ']).next().unwrap_or_default()
            } else {
                continue;
            };
            if name.is_empty() {
                continue;
            }
            declared += 1;
            assert!(
                dts.contains(&format!("export function {name}("))
                    || dts.contains(&format!("export const {name}:")),
                "the scaffold's facade does not name the crate's `{name}`:\n{dts}"
            );
        }
        assert!(
            declared >= 5,
            "the scaffold's Rust half should publish a real API, found {declared}"
        );
    }

    #[test]
    fn the_native_half_lives_where_build_rs_looks_for_it() {
        // `build.rs` reads `native/<platform>/` for the platforms whose half is
        // C, Swift or ArkTS; Android is the exception, written into `build.rs`
        // and `src/lib.rs` itself: its half is Java, so the Rust edge owns
        // `src/android.rs` and the capability owns `android/`.
        let dir = scaffold("layout", &["macos"]);

        for rel in [
            "native/common/floom_layout.h",
            "native/desktop/floom_layout.c",
        ] {
            assert!(dir.join(rel).is_file(), "{rel} must exist under native/");
        }
        assert!(
            !dir.join("android").exists(),
            "the native half must not live beside src/ for a non-android target"
        );
        assert!(
            !dir.join("ios").exists(),
            "the native half must not live beside src/ for a non-ios target"
        );

        let android_dir = scaffold("nativedroid", &["android"]);
        assert!(
            android_dir
                .join("android/src/main/java/dev/floom/plugin/nativedroid/NativedroidPlugin.java")
                .is_file(),
            "the java half lives in the android library module"
        );
        assert!(
            android_dir.join("src/android.rs").is_file(),
            "the rust edge lives beside src/"
        );
        assert!(
            !android_dir.join("native/android").exists(),
            "there is no native/ directory for the java half"
        );
    }

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    /// Scaffold an app of the given style into a scratch directory.
    fn scaffold_app(name: &str, style: AppStyle) -> PathBuf {
        let dir = repo_root()
            .join("tests")
            .join(format!("floom-cli-test-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch directory");
        let opts = CreateOpts {
            name: name.to_string(),
            template: Template::App,
            style,
            platforms: vec!["macos".to_string()],
            org: "com.example".to_string(),
            floom_crate: "floom".to_string(),
            floom_path: None,
            platform_path: None,
            force: true,
        };
        create_app(&opts, &dir, &opts.name).expect("scaffold the app");
        dir
    }

    #[test]
    fn style_defaults_to_rust_and_accepts_the_two_spellings() {
        let default = parse_create(&args(&["demo"])).expect("default style");
        assert_eq!(default.style, AppStyle::Rust);

        let explicit = parse_create(&args(&["demo", "--style", "tsx"])).expect("style tsx");
        assert_eq!(explicit.style, AppStyle::Tsx);

        let shorthand = parse_create(&args(&["demo", "--tsx"])).expect("--tsx");
        assert_eq!(shorthand.style, AppStyle::Tsx);
    }

    #[test]
    fn unknown_style_is_rejected() {
        let error = parse_create(&args(&["demo", "--style", "swift"])).expect_err("unknown style");
        assert!(error.contains("unknown style `swift`"), "{error}");
        assert!(error.contains("rust, tsx"), "{error}");
    }

    #[test]
    fn style_is_rejected_for_plugins() {
        let error = parse_create(&args(&["--plugin", "--style", "tsx", "cam"]))
            .expect_err("style on a plugin");
        assert!(error.contains("applies to apps"), "{error}");
    }

    #[test]
    fn platforms_are_rejected_for_tsx_apps() {
        let error = parse_create(&args(&["--tsx", "--platforms", "android", "app"]))
            .expect_err("platforms on a tsx app");
        assert!(error.contains("applies to the Rust scaffold"), "{error}");
    }

    #[test]
    fn rust_style_writes_no_typescript_files() {
        let dir = scaffold_app("rustapp", AppStyle::Rust);
        assert!(dir.join("Cargo.toml").is_file());
        assert!(dir.join("src/main.rs").is_file());
        for absent in ["app.tsx", "package.json", "tsconfig.json"] {
            assert!(
                !dir.join(absent).exists(),
                "a Rust-style app must not carry {absent}"
            );
        }
        let readme = read(&dir, "README.md");
        assert!(!readme.contains("@types/floom"), "{readme}");
    }

    #[test]
    fn mobile_scaffold_emits_a_cdylib_lib_target() {
        let mobile = templates::cargo_toml("my_app", "floom", None, true);
        assert!(mobile.contains("[lib]"), "{mobile}");
        assert!(mobile.contains("crate-type = [\"cdylib\"]"), "{mobile}");
        assert!(mobile.contains("name = \"my_app\""), "{mobile}");
        assert!(mobile.contains("path = \"src/lib.rs\""), "{mobile}");

        let desktop = templates::cargo_toml("my_app", "floom", None, false);
        assert!(!desktop.contains("[lib]"), "{desktop}");
        assert!(!desktop.contains("cdylib"), "{desktop}");
    }

    #[test]
    fn mobile_scaffold_writes_the_cdylib_stub_and_desktop_does_not() {
        let mobile = repo_root()
            .join("tests")
            .join("floom-cli-test-mobilelib");
        let _ = fs::remove_dir_all(&mobile);
        fs::create_dir_all(&mobile).unwrap();
        let opts = CreateOpts {
            name: "mob".to_string(),
            template: Template::App,
            style: AppStyle::Rust,
            platforms: vec!["android".to_string()],
            org: "com.example".to_string(),
            floom_crate: "floom".to_string(),
            floom_path: None,
            platform_path: None,
            force: true,
        };
        create_rust_app(&opts, &mobile, "mob").unwrap();
        assert!(mobile.join("src/lib.rs").is_file());
        assert!(
            read(&mobile, "Cargo.toml").contains("crate-type = [\"cdylib\"]")
        );

        let desktop = scaffold_app("deskapp", AppStyle::Rust);
        assert!(!desktop.join("src/lib.rs").exists());
    }

    #[test]
    fn tsx_style_writes_only_the_typescript_app() {
        let dir = scaffold_app("tsxapp", AppStyle::Tsx);
        for present in [
            "app.tsx",
            "package.json",
            "tsconfig.json",
            ".gitignore",
            "README.md",
        ] {
            assert!(dir.join(present).is_file(), "{present} must be scaffolded");
        }
        for absent in ["Cargo.toml", "src", "tests", "macos"] {
            assert!(
                !dir.join(absent).exists(),
                "a TSX app must not carry {absent}"
            );
        }
    }

    #[test]
    fn tsx_style_pins_the_editor_declarations() {
        let dir = scaffold_app("tsxpins", AppStyle::Tsx);
        let package = read(&dir, "package.json");
        assert!(
            package.contains(&format!(
                "\"@types/floom\": \"^{}\"",
                env!("CARGO_PKG_VERSION")
            )),
            "the declarations must track the toolchain version:\n{package}"
        );
        assert!(package.contains("\"typescript\""), "{package}");

        let tsconfig = read(&dir, "tsconfig.json");
        assert!(tsconfig.contains(r#""lib": ["ES2020"]"#), "{tsconfig}");

        let gitignore = read(&dir, ".gitignore");
        assert!(gitignore.contains(".floom/"), "{gitignore}");
        assert!(gitignore.contains("node_modules/"), "{gitignore}");

        let app = read(&dir, "app.tsx");
        assert!(app.contains("mount(App)"), "{app}");
        assert!(
            !app.contains("{{TITLE}}"),
            "the title placeholder must be filled"
        );
    }

    #[test]
    fn lifecycle_options_parse_dir_and_flags() {
        let opts = parse_lifecycle(&["../some/app".into(), "--release".into()]).unwrap();
        assert_eq!(opts.dir, Path::new("../some/app"));
        assert!(opts.release);

        let opts = parse_lifecycle(&[
            "--features".into(),
            "desktop,renderer-gpu".into(),
            "--platform".into(),
            "ios".into(),
        ])
        .unwrap();
        assert_eq!(opts.dir, Path::new("."));
        assert_eq!(opts.features, vec!["desktop,renderer-gpu".to_string()]);
        assert_eq!(opts.platform.as_deref(), Some("ios"));

        // Two positional dirs, or an unknown flag, are rejected.
        assert!(parse_lifecycle(&["a".into(), "b".into()]).is_err());
        assert!(parse_lifecycle(&["--bogus".into()]).is_err());
    }
}
