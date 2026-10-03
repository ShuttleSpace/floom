//! Headless runner: mount every demo into a [`WidgetHost`], pump one frame,
//! and assert it painted something.
//!
//! This is what makes the gallery a *golden* example rather than a screenshot:
//! `cargo run -p floom2-demo` compiles nothing extra and needs no display, so
//! `scripts/run-golden-examples.sh` can run it on every change. A demo whose
//! builder stops producing a paintable tree fails here instead of silently
//! rendering an empty window.

use floom_app::WidgetHost;
use floom_layout::LayoutStyle;
use floom_types::Size;

use crate::demos;

/// The viewport the headless run lays out against. Big enough that a demo's
/// intrinsic size does not collapse to zero.
const VIEWPORT: Size = Size::new(420.0, 760.0);

pub fn run() {
    let registry = demos::demos();
    println!("floom2-demo — {} demos, headless", registry.len());

    let mut empty: Vec<&str> = Vec::new();
    for demo in registry {
        let mut host = WidgetHost::new();
        host.set_size(VIEWPORT);
        host.mount_root(LayoutStyle::default(), demos::demo_root(demo));
        let stats = host.frame();
        println!(
            "  {:<16} {:<11} painted={}",
            demo.id,
            demo.section.title(),
            stats.painted
        );
        if stats.painted == 0 {
            empty.push(demo.id);
        }
    }

    assert!(
        empty.is_empty(),
        "these demos mounted no paintable widgets: {empty:?}"
    );
    println!("all {} demos mounted and painted", registry.len());
}
