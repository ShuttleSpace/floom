//! `floom` — command line tools for Floom.
//!
//! Stage 1 ships `floom create`, which scaffolds a Flutter-style project: a
//! Rust core under `src/` plus one directory per target platform.

mod native_project;
mod templates;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
floom — command line tools for Floom

USAGE:
    floom <COMMAND> [OPTIONS]

COMMANDS:
    create <NAME>    Scaffold a new Floom app
    create --plugin  Scaffold a Floom plugin (Rust half + native halves)
    help             Show this message

OPTIONS (create):
    --template <KIND>      app (default) or plugin
    --plugin               Shorthand for `--template plugin`
    --platforms <LIST>     Comma-separated subset of:
                            android,ios,ohos,web,windows,linux,macos,tui
                            (default: all)
    --org <REVERSE_DNS>    Organisation for native projects, e.g. com.example
    --floom-crate <NAME>   Crate to depend on (default: floom)
    --floom-path <PATH>    Depend on a local checkout instead of a version
    --platform-path <PATH> Path to floom-platform, for a plugin's dependency
    --force                Overwrite an existing target directory

EXAMPLES:
    floom create my_app
    floom create my_app --platforms android,ios
    floom create --plugin --platforms android,ios camera
    floom create --template plugin my_plugin --platforms ohos
";

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
        "create" => create(&args[1..]),
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
            other => Err(format!(
                "unknown template `{other}`; known: app, plugin"
            )),
        }
    }

    }

struct CreateOpts {
    name: String,
    template: Template,
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
    let mut platforms: Option<Vec<String>> = None;
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
            "--org" => org = take_value("--org")?,
            "--floom-crate" => floom_crate = take_value("--floom-crate")?,
            "--floom-path" => floom_path = Some(take_value("--floom-path")?),
            "--platform-path" => platform_path = Some(take_value("--platform-path")?),
            "--force" => force = true,
            other if other.starts_with('-') => {
                return Err(format!("unknown option `{other}`\n\n{USAGE}"))
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
    Ok(CreateOpts {
        name,
        template,
        platforms: platforms.unwrap_or_else(|| {
            templates::PLATFORMS.iter().map(|s| s.to_string()).collect()
        }),
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
        Template::App => create_app(&opts, &root)?,
        Template::Plugin => create_plugin(&opts, &root)?,
    }
    Ok(())
}

/// Scaffold an app: a widget tree, plus one directory per platform.
fn create_app(opts: &CreateOpts, root: &Path) -> Result<(), String> {

    // --- Rust core -------------------------------------------------
    write(
        &root.join("Cargo.toml"),
        &templates::cargo_toml(&opts.name, &opts.floom_crate, opts.floom_path.as_deref()),
    )?;
    write(
        &root.join("src/main.rs"),
        &templates::MAIN_RS
            .replace("{{TITLE}}", &opts.name)
            .replace("{{FLOOM}}", &opts.floom_crate),
    )?;
    write(
        &root.join("tests/smoke.rs"),
        "// Integration tests go here. Unit tests belong next to the code in `src/`.\n",
    )?;

    // --- per-platform native projects ------------------------------
    //
    // At the top level, next to `src/`, not under a `native/` directory: this
    // matches a Flutter project, where `android/` sits beside `lib/` and opens in
    // Android Studio without the reader having to know there is a hierarchy.
    for p in &opts.platforms {
        emit_app_platform(opts, root, p)?;
    }

    write(&root.join("README.md"), &templates::root_readme(&opts.name, &opts.platforms))?;

    println!("Created Floom app `{}` in ./{}", opts.name, opts.name);
    println!("  organisation: {}", opts.org);
    println!("  platforms:    {}", opts.platforms.join(", "));
    println!();
    println!("Next:");
    println!("  cd {}", opts.name);
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
    // --- Rust half -----------------------------------------------------
    write(
        &root.join("Cargo.toml"),
        &templates::plugin_cargo_toml(&opts.name, opts.platform_path.as_deref()),
    )?;
    write(
        &root.join("build.rs"),
        &templates::plugin_build_rs(&opts.name),
    )?;
    write(
        &root.join("src/lib.rs"),
        &templates::plugin_lib_rs(&opts.name),
    )?;

    // The C ABI sits at the top level, beside `src/` and every platform
    // directory, because it belongs to the plugin rather than to any one
    // platform: each platform's code implements it, and `bindgen` reads it to
    // generate the Rust side. Written once, outside the platform loop.
    write(&root.join("floom.h"), &templates::plugin_header_rs(&opts.name))?;

    // --- native halves -------------------------------------------------
    //
    // Beside `src/`, not under `native/`: a Flutter plugin puts `android/` at the
    // top level and Android Studio opens it directly. A nested `native/android/`
    // is the same files one level harder to find.
    for p in &opts.platforms {
        emit_plugin_platform(opts, root, p)?;
    }

    // --- example --------------------------------------------------------
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

    println!("Created Floom plugin `{}` in ./{}", opts.name, root.display());
    println!("  crate:        floom-plugin-{}", opts.name);
    println!("  platforms:    {}", opts.platforms.join(", "));
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
            write(&dir.join("settings.gradle.kts"), &native_project::android_app_settings_gradle(&opts.org, &opts.name))?;
            write(&dir.join("build.gradle.kts"), &native_project::android_root_build_gradle())?;
            write(&dir.join("gradle.properties"), &native_project::android_gradle_properties())?;
            write(
                &dir.join("gradle/wrapper/gradle-wrapper.properties"),
                &native_project::android_gradle_wrapper_properties(),
            )?;
            write(&dir.join(".gitignore"), native_project::ANDROID_GITIGNORE)?;
            write(&dir.join("app/proguard-rules.pro"), native_project::ANDROID_PROGUARD_RULES)?;
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
                    native_project::kotlin_package_path(&native_project::android_app_id(&opts.org, &opts.name)),
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
                Some(sdk) => write(&dir.join("local.properties"), &native_project::android_local_properties(&sdk))?,
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
            write(&dir.join("src/AppDelegate.swift"), &templates::ios_app_delegate_swift(&opts.name))?;
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
            write(&dir.join("CMakeLists.txt"), &native_project::desktop_cmake(p, &opts.name, &lib))?;
        }
        "web" => {
            write(
                &dir.join("index.html"),
                &templates::INDEX_HTML.replace("{{PROJECT}}", &opts.name),
            )?;
        }
        "ohos" => {
            write(&dir.join("build-profile.json5"), &templates::ohos_build_profile(&opts.name))?;
            write(
                &dir.join("entry/src/main/module.json5"),
                &templates::ohos_module_json(&opts.name),
            )?;
            write(
                &dir.join("entry/src/main/ets/entryability/EntryAbility.ets"),
                &templates::ohos_entry_ability(&opts.name),
            )?;
            write(&dir.join("oh-package.json5"), &templates::ohos_package_json(&opts.name))?;
        }
        _ => {}
    }

    // Every platform gets a README, but after its project files rather than
    // instead of them: it now explains how to build a thing that exists.
    write(&dir.join("README.md"), &templates::platform_readme(label, opener))
}

/// Writes the native half of one platform of a **plugin**.
///
/// The difference from an app's platform is that a plugin is a *library* on every
/// platform: a Gradle library, a podspec, a CMake static library. It never owns
/// an activity, a bundle id or an `Info.plist`, because the host owns those, and
/// emitting them would make two plugins fight over the host's identity.
fn emit_plugin_platform(opts: &CreateOpts, root: &Path, platform: &str) -> Result<(), String> {
    let dir = root.join(platform);
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let lib = native_project::lib_name(&opts.name);

    match platform {
        "android" => {
            write(&dir.join("settings.gradle.kts"), &native_project::android_plugin_settings_gradle(&opts.name))?;
            // A plugin project is a single Gradle module: `build.gradle.kts` is
            // both the root and the library, so it is written once, and it
            // carries the `repositories` block itself.
            //
            // Writing a root file and then overwriting it with the library file is
            // the shape of bug that costs an afternoon: the overwrite drops
            // `allprojects { repositories { ... } }` and the build fails at
            // `compileDebugKotlin` with "Cannot resolve external dependency
            // org.jetbrains.kotlin:kotlin-stdlib because no repositories are
            // defined" — a message about Kotlin, in a file that never mentioned it.
            write(
                &dir.join("build.gradle.kts"),
                &native_project::android_plugin_build_gradle(&opts.name, &lib),
            )?;
            write(&dir.join("gradle.properties"), &native_project::android_gradle_properties())?;
            write(
                &dir.join("gradle/wrapper/gradle-wrapper.properties"),
                &native_project::android_gradle_wrapper_properties(),
            )?;
            write(&dir.join(".gitignore"), native_project::ANDROID_GITIGNORE)?;
            write(&dir.join("src/main/AndroidManifest.xml"), &native_project::android_plugin_manifest(&opts.name))?;
            write(
                &dir.join(format!(
                    "src/main/kotlin/dev/floom/plugin/{}Plugin.kt",
                    native_project::kotlin_class_name(&opts.name)
                )),
                &native_project::android_plugin_kotlin(&opts.name),
            )?;
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
        "windows" | "linux" => {
            write(&dir.join("CMakeLists.txt"), &native_project::desktop_cmake(platform, &opts.name, &lib))?;
        }
        "ohos" => {
            write(&dir.join("oh-package.json5"), &templates::ohos_package_json(&opts.name))?;
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
    )
}
