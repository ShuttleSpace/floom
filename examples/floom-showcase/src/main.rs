//! The Floom 2.0 showcase: one application over the whole stack.
//!
//! Run with `cargo run -p floom-showcase`.
//!
//! - `/components` mounts the engine's Tier-0 widgets directly.
//! - `/signals` drives state through `floom-signal`'s `SyncBaseSignal`.
//! - `/kits` mounts the ported `floom-ui` material components.
//! - `/editor` mounts `floom-editor`'s `EditorWidget`.
//! - `/devtools` explains the panel that F12 opens (the entry is
//!   `run_desktop_with_devtools`, so it is wired for the whole app).
//!
//! Navigation goes through `floom-navigation`'s `Router`. That crate is
//! widget-free, so the shell owns the router and re-declares its subtree when
//! `Router::current` changes.

pub mod editor;
pub mod pages;
pub mod routes;
pub mod shell;
pub mod state;
#[cfg(test)]
pub mod tests;
pub mod widgets;

use std::sync::Arc;

use floom::floom_app::WidgetHost;
use floom::floom_layout::LayoutStyle;
use floom::floom_types::Size;

use crate::routes::{AppRoute, NavIntent};
use crate::shell::ShowcaseShell;
use crate::state::AppState;
use floom_signal::SignalGet;

const WINDOW: Size = Size::new(1180.0, 820.0);

/// Mounts and pumps every route without a window, for the golden-example gate.
fn run_headless() {
    let mut total = 0;
    for route in AppRoute::ALL {
        let state = Arc::new(AppState::new());
        let mut host = WidgetHost::new();
        host.set_size(WINDOW);
        host.mount_root(LayoutStyle::default(), ShowcaseShell::new_at(state, route));
        let mut painted = 0;
        for _ in 0..3 {
            painted = host.frame().painted;
        }
        assert!(painted > 0, "{} painted no widgets", route.path());
        total += painted;
        println!("{:<12} painted {}", route.path(), painted);
    }
    // The router is the one piece with no unit test in an engine crate: it is
    // the shell's own wiring, so exercise a push and a pop through a real host.
    let state = Arc::new(AppState::new());
    let mut host = WidgetHost::new();
    host.set_size(WINDOW);
    let root = host.mount_root(LayoutStyle::default(), ShowcaseShell::new(state));
    host.frame();
    for route in [
        AppRoute::Components,
        AppRoute::Kits,
        AppRoute::Editor,
        AppRoute::Devtools,
    ] {
        if let Some(shell) = host.widget_mut::<ShowcaseShell>(root) {
            *shell.pending.lock().unwrap() = Some(NavIntent::Go(route));
        }
        host.frame();
        let rendered = host
            .widget::<ShowcaseShell>(root)
            .map(|shell| shell.rendered)
            .expect("the shell is the root");
        assert_eq!(rendered, route, "router did not reach {}", route.path());
    }
    if let Some(shell) = host.widget_mut::<ShowcaseShell>(root) {
        *shell.pending.lock().unwrap() = Some(NavIntent::Back);
    }
    host.frame();
    let rendered = host
        .widget::<ShowcaseShell>(root)
        .map(|shell| shell.rendered)
        .expect("the shell is the root");
    assert_eq!(rendered, AppRoute::Editor, "Back did not pop to /editor");

    println!(
        "showcase: {} routes mounted, {} paints total; router push/back ok",
        AppRoute::ALL.len(),
        total
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--headless") {
        run_headless();
        return;
    }

    let state = Arc::new(AppState::new());
    let shell_state = state.clone();
    let readout_state = state.clone();

    let result = floom::run_desktop_with_devtools(
        "Floom 2.0 — Showcase",
        WINDOW,
        move |host: &mut WidgetHost| {
            host.mount_root(
                LayoutStyle::default(),
                ShowcaseShell::new(shell_state.clone()),
            );
        },
        move |_panel| {
            let route = AppRoute::ALL
                .into_iter()
                .find(|r| r.id() as u8 == readout_state.route.get())
                .map(|r| r.path())
                .unwrap_or("/");
            vec![
                floom::devtools::ReadoutRow::heading("showcase"),
                floom::devtools::ReadoutRow::new("route", route),
                floom::devtools::ReadoutRow::new(
                    "engine clicks",
                    readout_state.clicks.get().to_string(),
                ),
                floom::devtools::ReadoutRow::new(
                    "signal value",
                    readout_state.signal_count.get().to_string(),
                ),
                floom::devtools::ReadoutRow::new(
                    "kit clicks",
                    readout_state.kit_clicks.get().to_string(),
                ),
            ]
        },
    );

    if let Err(err) = result {
        eprintln!("window loop failed: {err}");
        std::process::exit(1);
    }
}
