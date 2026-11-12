// SPDX-License-Identifier: MIT
//! The Floom component gallery — an application over the `floom-ui` satellite.
//!
//! Run it with `cargo run -p floom-demo`. `cargo run -p floom-demo -- --headless`
//! mounts every page and prints how many widgets each painted, which is a manual
//! health check rather than something the gate runs.

use std::sync::Arc;

use floom::floom_app::WidgetHost;
use floom::floom_layout::LayoutStyle;

use floom_demo::registry;
use floom_demo::shell::GalleryShell;
use floom_demo::shell::WINDOW;

/// Mounts and pumps every page without a window, printing a painted count for
/// each. A developer runs this to confirm the gallery still renders everything;
/// the gate instead runs `tests/` which assert the same thing as `#[test]`s.
fn run_headless() {
    let mut total = 0;
    for page in registry::pages() {
        let mut host = WidgetHost::new();
        host.set_size(WINDOW);
        host.mount_root(LayoutStyle::default(), page.build_shell());
        let mut painted = 0;
        for _ in 0..3 {
            painted = host.frame().painted;
        }
        assert!(painted > 0, "{} painted no widgets", page.name);
        total += painted;
        println!("{:<28} painted {}", page.name, painted);
    }
    println!("total painted: {total}");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--headless") {
        run_headless();
        return;
    }

    let pages = registry::pages();
    let shell_pages = Arc::new(pages);
    let readout_pages = shell_pages.clone();

    let result = floom::run_desktop_with_devtools(
        "Floom 2.0 — Component gallery",
        WINDOW,
        move |host: &mut WidgetHost| {
            host.mount_root(
                LayoutStyle::default(),
                GalleryShell::showing((*shell_pages).clone(), 0),
            );
        },
        move |_panel| {
            let count = readout_pages.len();
            vec![
                floom::devtools::ReadoutRow::heading("gallery"),
                floom::devtools::ReadoutRow::new("components", count.to_string()),
                floom::devtools::ReadoutRow::new("design systems", "Material, Cupertino, Fluent"),
            ]
        },
    );

    if let Err(error) = result {
        eprintln!("floom-demo failed: {error}");
        std::process::exit(1);
    }
}
