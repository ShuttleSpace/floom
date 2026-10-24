//! The devtools, in a real window.
//!
//! The whole integration is two lines: one to mount the panel, one per frame to
//! keep it current. Everything else — F12 to open it, Escape to close it,
//! Ctrl+Shift+C to pick an element, the tree, the breadcrumb, the styles, the
//! layout box, the performance timeline, the console — is the panel's own.
//!
//! ```text
//! floom::run_desktop_with_devtools(   // the one line, plus the loop's key hook
//!     "floom — devtools", size, |host| { /* mount your app */ },
//!     |state| { /* the panel's rows */ },
//! );
//! ```
//!
//! What to try:
//!
//! | key | what it does |
//! | --- | --- |
//! | **F12** | show / hide the panel |
//! | **Ctrl+Shift+I** | the same, where F12 belongs to the OS |
//! | **Ctrl+Shift+C** | arm the pick, then click any element to inspect it |
//! | **Escape** | close the panel |
//!
//! Run with `cargo run -p floom-devtools-demo`.

use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

use floom::prelude::*;

/// The window's logical size, mirrored by the host's viewport.
const WINDOW: Size = Size::new(720.0, 560.0);

fn main() {
    // A little state for the demo app to mutate, so the panel has something
    // live to look at: the console and the tree both change as you click.
    let clicks = Arc::new(AtomicI32::new(0));

    let result = floom::run_desktop_with_devtools(
        "floom — devtools",
        WINDOW,
        {
            let clicks = clicks.clone();
            move |host: &mut WidgetHost| {
                let mut column = Stack::new(Axis::Vertical)
                    .gap(12.0)
                    .child(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new().height(28.0),
                        || Box::new(Text::new("devtools demo — press F12")),
                    ))
                    .child(Child::new(
                        ChildKey::index(1),
                        LayoutRequest::new().height(28.0),
                        || Box::new(Text::new("Ctrl+Shift+C picks an element")),
                    ));

                // A row of three buttons: three subtrees the tree panel can
                // show, and three things the pick can land on.
                let mut row = Stack::new(Axis::Horizontal).gap(8.0);
                for (index, label) in ["one", "two", "three"].iter().enumerate() {
                    let counter = clicks.clone();
                    row = row.child(Child::new(
                        ChildKey::index(index),
                        LayoutRequest::new().width(96.0).height(36.0),
                        move || {
                            let counter = counter.clone();
                            Box::new(
                                Button::new(*label).on_click(move || {
                                    counter.fetch_add(1, Ordering::SeqCst);
                                }),
                            )
                        },
                    ));
                }
                let row = std::sync::Arc::new(row);
                let row_slot = row.clone();
                column = column.child(Child::new(
                    ChildKey::index(2),
                    LayoutRequest::new().height(36.0),
                    move || Box::new((*row_slot).clone()),
                ));

                // A counter readout: the same atomic the buttons bump, so the
                // panel's console and this line move together.
                //
                // A signal would be the natural spelling, and it does not work
                // here: a child builder is an `Fn` the host may call from
                // another thread, and the engine's signals are `Rc`-based. The
                // atomic is what crosses that boundary.
                let shown = clicks.clone();
                column = column.child(Child::new(
                    ChildKey::index(3),
                    LayoutRequest::new().height(24.0),
                    move || {
                        let shown = shown.clone();
                        Box::new(Text::new(format!("clicks: {}", shown.load(Ordering::SeqCst))))
                    },
                ));

                host.mount_root(LayoutStyle::default(), column);
            }
        },
        // The panel's readout. This closure is called every frame the panel is
        // visible, and it answers from the app's own state.
        {
            let clicks = clicks.clone();
            move |state| {
                let mut rows = vec![
                    // The demo's own line: what the app knows about itself.
                    floom::devtools::ReadoutRow::heading("demo"),
                    floom::devtools::ReadoutRow::new(
                        "clicks",
                        clicks.load(Ordering::SeqCst).to_string(),
                    ),
                floom::devtools::ReadoutRow::new("tab", format!("{:?}", state.tab())),
            ];
            // The engine's rows — the tree, the breadcrumb, and whichever
            // readout the tab asks for — are composed in by the entry itself,
            // which also refreshes the tree every frame. This closure only
            // answers for the app.
            rows
            }
        },
    );

    if let Err(err) = result {
        eprintln!("window loop failed: {err}");
        std::process::exit(1);
    }
}
