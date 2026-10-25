//! The devtools, in a real window.
//!
//! The whole integration is one call: `run_desktop_with_devtools` mounts the
//! panel, keeps its tree current every frame, and routes the shortcut and the
//! pick before the app's own tree sees them. What the panel shows is split in
//! two, and the split is worth knowing:
//!
//! - **The engine's half** — the element tree, the breadcrumb to the selected
//!   node, and that node's style or layout — is composed in by the entry itself,
//!   from the live tree. The `Elements` / `Style` / `Layout` tabs switch between
//!   those, and the tree is re-read every frame rather than at startup.
//! - **The app's half** — the rows this demo's `readout` closure returns — is
//!   whatever the app knows about itself. This one shows a click counter.
//!
//! Performance and the console are not wired here: `performance_rows` and
//! `console_rows` build those rows, but they need a `PerformanceSnapshot` and a
//! list of caught `Report`s, which belong to the app (or to a profiler the app
//! opted into) rather than to the panel. An app that has them appends the rows
//! from its own closure; this one has neither, so it shows neither rather than
//! a timeline of nothing.
//!
//! ```text
//! floom::run_desktop_with_devtools(
//!     "floom — devtools", size, |host| { /* mount your app */ },
//!     |state| { /* the app's own rows */ },
//! );
//! ```
//!
//! What to try:
//!
//! | input | what it does |
//! | --- | --- |
//! | **F12** | show / hide the panel |
//! | **Cmd+Shift+I** (macOS) / **Ctrl+Shift+I** | the same, where F12 belongs to the OS |
//! | **Ctrl+Shift+C** | arm the pick, then click any element to inspect it |
//! | **Escape** | close the panel — and only while it is open, so Escape stays the app's |
//! | **click a tab** | switch between the tree, the selected element's style, and its layout |
//! | **click a row's name** | select that element; Style and Layout then describe it |
//! | **click a row's triangle** | fold or unfold its children |
//! | **right-click a row** | the row's menu: expand/collapse, select, clear selection |
//!
//! The two clicks are deliberately different. The name selects — that is what a
//! reader wants when opening Style — and the triangle discloses, so inspecting a
//! container does not fold away the children being inspected. A leaf has no
//! triangle, so a press in that column selects it like any other.
//!
//! The panel is chrome rather than a dialog: while it is hidden its column
//! belongs to the app, clicking a tab does not take the keyboard, and a press
//! outside the panel does not close it.
//!
//! ## Why the window is white
//!
//! The panel paints itself on a dark card, and the app under it paints its own
//! background. A demo that inherited a dark window would have made every
//! low-contrast bug in the app look like a devtools bug; white makes the
//! app's ink and the panel's chrome separable at a glance.
//!
//! Run with `cargo run -p floom-devtools-demo`.

use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

use floom::prelude::*;
// `Surface` is a decorated box that holds children; it is not in the prelude
// (that tier is the beginner widget set), so the demo names it explicitly.
use floom::floom_types::Insets;
use floom::floom_widgets::Surface;

/// The window's logical size, mirrored by the host's viewport.
const WINDOW: Size = Size::new(720.0, 560.0);

fn main() {
    // A little state for the demo app to mutate, so the panel has something
    // live to look at: the tree changes as you click, because the counter line
    // is rebuilt from it.
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
                            Box::new(Button::new(*label).on_click(move || {
                                counter.fetch_add(1, Ordering::SeqCst);
                            }))
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
                // panel's tree and this line move together.
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
                        Box::new(Text::new(format!(
                            "clicks: {}",
                            shown.load(Ordering::SeqCst)
                        )))
                    },
                ));

                // The root is a white surface with padding, so the demo's own
                // ink sits on white and the panel's dark card is visibly a
                // separate thing. `Stack` is `Clone` but a child builder is an
                // `Fn`, so the column travels in an `Arc` the way the button row
                // above does.
                let column = std::sync::Arc::new(column);
                let column_slot = column.clone();
                host.mount_root(
                    LayoutStyle::default(),
                    Surface::new()
                        .background(Color::WHITE)
                        .padding(Insets::uniform(24.0))
                        .child(Child::new(
                            ChildKey::index(0),
                            LayoutRequest::new().width_pct(100.0).fill(),
                            move || Box::new((*column_slot).clone()),
                        )),
                );
            }
        },
        // The panel's readout. This closure is called every frame the panel is
        // visible, and it answers from the app's own state.
        {
            let clicks = clicks.clone();
            move |state| {
                // The demo's own rows: what the app knows about itself. The
                // engine's rows — the tree, the breadcrumb, and whichever
                // readout the tab asks for — are composed in by the entry
                // itself, which also refreshes the tree every frame. This
                // closure only answers for the app.
                vec![
                    floom::devtools::ReadoutRow::heading("demo"),
                    floom::devtools::ReadoutRow::new(
                        "clicks",
                        clicks.load(Ordering::SeqCst).to_string(),
                    ),
                    floom::devtools::ReadoutRow::new("tab", format!("{:?}", state.tab())),
                ]
            }
        },
    );

    if let Err(err) = result {
        eprintln!("window loop failed: {err}");
        std::process::exit(1);
    }
}
