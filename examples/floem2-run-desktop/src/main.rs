//! A Floem 2.0 app in a real OS window, through the facade's one-call entry.
//!
//! This is the desktop counterpart of `floem2-hello`: where that example mounts
//! a tree and paints one headless frame, this one hands the same kind of tree
//! to [`floem2::run_desktop`] and lets the backend own the winit event loop.
//! There is no `WinitApp` impl and no `AppRunner` wiring in this file — that is
//! the point of the entry.
//!
//! Run with `cargo run -p floem2-run-desktop`.

use floem2::prelude::*;

fn main() {
    let clicked = std::sync::Arc::new(std::sync::atomic::AtomicI32::new(0));

    let result = floem2::run_desktop("floem2 — run_desktop", Size::new(480.0, 240.0), {
        let clicked = clicked.clone();
        move |host: &mut WidgetHost| {
            let root = Stack::new(Axis::Vertical)
                .gap(16.0)
                .child(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                    Box::new(Text::new("run_desktop: a real winit window"))
                }))
                .child(Child::new(
                    ChildKey::index(1),
                    LayoutRequest::new().width(160.0).height(40.0),
                    // The builder is a replayable `Fn`, so it runs once per
                    // layout and must hand out a fresh clone of the flag rather
                    // than moving it.
                    move || {
                        let flag = clicked.clone();
                        Box::new(Button::new("click me").on_click(move || {
                            flag.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        }))
                    },
                ));
            host.mount_root(LayoutStyle::default(), root);
        }
    });

    if let Err(err) = result {
        eprintln!("desktop loop failed: {err}");
        std::process::exit(1);
    }
    println!(
        "window closed after {} clicks",
        clicked.load(std::sync::atomic::Ordering::SeqCst)
    );
}
