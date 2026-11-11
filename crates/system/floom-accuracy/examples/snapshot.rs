// SPDX-License-Identifier: MIT
//! `floom snapshot` — render a gallery screen to PNG + element-tree JSON.
//!
//! ```text
//! just snapshot settings              # -> snapshots/settings.{png,json}
//! just snapshot --list                # print the screen names
//! just snapshot --all ./out           # render all nine screens
//! cargo run -p floom-accuracy --example snapshot -- feed /tmp/floom-snap
//! ```
//!
//! Everything is headless: no window, no GPU, no display. The JSON bundles the
//! same `UiTree` / `SemanticsEntryDto` / `SceneLines` / `ScreenDto` /
//! `EvaluationReport` types the agent and evaluate loop already publish, so a
//! CI job or an agent can parse it with the vocabulary it links.

use std::path::PathBuf;
use std::process::ExitCode;

use floom_accuracy::headless::{capture, stem, write_capture};
use floom_accuracy::{Screen, ScreenTokens};

const USAGE: &str = "\
floom snapshot — headless PNG + element-tree dump

USAGE:
    snapshot <SCREEN> [OUT_DIR]   Render one screen (OUT_DIR defaults to ./snapshots)
    snapshot --all [OUT_DIR]      Render all nine screens
    snapshot --list               List screen names
    snapshot --help               Show this message

SCREEN is a case-insensitive Screen name (see --list). Output is
<OUT_DIR>/<screen>.png and <OUT_DIR>/<screen>.json.
";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(first) = args.first() else {
        print!("{USAGE}");
        return Ok(());
    };

    if first == "--help" || first == "-h" {
        print!("{USAGE}");
        return Ok(());
    }
    if first == "--list" {
        for screen in Screen::ALL {
            println!("{}", stem(screen));
        }
        return Ok(());
    }

    let all = first == "--all";
    let (screens, out_dir): (Vec<Screen>, PathBuf) = if all {
        (
            Screen::ALL.to_vec(),
            PathBuf::from(args.get(1).map(String::as_str).unwrap_or("snapshots")),
        )
    } else {
        let screen = parse_screen(first)?;
        let out = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("snapshots"));
        if args.len() > 2 {
            return Err(format!("unexpected argument `{}`\n\n{USAGE}", args[2]));
        }
        (vec![screen], out)
    };

    let tokens = ScreenTokens::reference();
    for screen in screens {
        let shot = capture(screen, &tokens)?;
        let (png, json) = write_capture(&out_dir, &shot)?;
        println!(
            "{}: {}x{}  {} tree nodes -> {} , {}",
            shot.bundle.screen,
            shot.bundle.width,
            shot.bundle.height,
            shot.bundle.tree.nodes.len(),
            png.display(),
            json.display(),
        );
    }
    Ok(())
}

fn parse_screen(name: &str) -> Result<Screen, String> {
    let needle = name.to_lowercase();
    Screen::ALL
        .iter()
        .copied()
        .find(|s| stem(*s) == needle)
        .ok_or_else(|| {
            let known: Vec<String> = Screen::ALL.iter().map(|s| stem(*s)).collect();
            format!("unknown screen `{name}`; known: {}", known.join(", "))
        })
}
