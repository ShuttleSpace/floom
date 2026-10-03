//! `floom` — command line tools for Floom.
//!
//! Stage 1 ships `floom create`, which scaffolds a Flutter-style project: a
//! Rust core under `src/` plus one directory per target platform.

mod templates;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
floom — command line tools for Floom

USAGE:
    floom <COMMAND> [OPTIONS]

COMMANDS:
    create <NAME>    Scaffold a new Floom project
    help             Show this message

OPTIONS (create):
    --platforms <LIST>    Comma-separated subset of:
                           android,ios,ohos,web,windows,linux,macos,tui
                           (default: all)
    --org <REVERSE_DNS>   Organisation for native projects, e.g. com.example
    --floom-crate <NAME>  Crate to depend on (default: floom)
    --floom-path <PATH>   Depend on a local checkout instead of a version
    --force                Overwrite an existing target directory
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

struct CreateOpts {
    name: String,
    platforms: Vec<String>,
    org: String,
    floom_crate: String,
    floom_path: Option<String>,
    force: bool,
}

fn parse_create(args: &[String]) -> Result<CreateOpts, String> {
    let mut name: Option<String> = None;
    let mut platforms: Option<Vec<String>> = None;
    let mut org = String::from("com.example");
    let mut floom_crate = String::from("floom");
    let mut floom_path: Option<String> = None;
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
            "--org" => org = take_value("--org")?,
            "--floom-crate" => floom_crate = take_value("--floom-crate")?,
            "--floom-path" => floom_path = Some(take_value("--floom-path")?),
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
        platforms: platforms.unwrap_or_else(|| {
            templates::PLATFORMS.iter().map(|s| s.to_string()).collect()
        }),
        org,
        floom_crate,
        floom_path,
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

    // --- per-platform directories ----------------------------------
    for p in &opts.platforms {
        let dir = root.join(p);
        fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        if let Some((_, label, opener)) =
            templates::PLATFORM_NOTES.iter().find(|(k, _, _)| k == p)
        {
            write(
                &dir.join("README.md"),
                &templates::platform_readme(label, opener),
            )?;
        }
        if p == "web" {
            write(
                &dir.join("index.html"),
                &templates::INDEX_HTML.replace("{{PROJECT}}", &opts.name),
            )?;
        }
    }

    write(&root.join("README.md"), &templates::root_readme(&opts.name, &opts.platforms))?;

    println!("Created Floom project `{}` in ./{}", opts.name, opts.name);
    println!("  organisation: {}", opts.org);
    println!("  platforms:    {}", opts.platforms.join(", "));
    println!();
    println!("Next:");
    println!("  cd {}", opts.name);
    println!("  cargo run");
    Ok(())
}