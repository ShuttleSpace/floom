//! The Floem 2.0 composite-components demo.
//!
//! `floem-app` is a *headless* shell, so this example mounts a tree of the new
//! Tier-2 composites, pumps frames and prints what the reconciler did. It
//! exercises every component from Phase 14.3:
//!
//! ```text
//! Stack → List / Conditional / Overlay+Dialog / Tabs
//! ```
//!
//! The interesting part is the *reconciliation*: mutating a composite via
//! `host.widget_mut(..)` and pumping a frame mounts/unmounts exactly the
//! subtree that changed, reusing every surviving widget by its [`ChildKey`].
//!
//! Run with `cargo run -p floem2-counter`.

use std::any::type_name;

use floem_app::{NodeId, WidgetHost};
use floem_core::{Axis, Child, ChildKey, LayoutRequest, Point, Size, WidgetAny};
use floem_layout::LayoutStyle;
use floem_types::PointerEvent;
use floem_widgets::{Button, Conditional, Dialog, List, Stack, Tabs, Text};

/// Stable items backing the demo [`List`].
fn item_key(s: &String) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for byte in s.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Builds the tab strip used by the demo.
fn make_tabs() -> Tabs {
    Tabs::new()
        .tab(
            "One",
            Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                Box::new(Text::new("panel one"))
            }),
        )
        .tab(
            "Two",
            Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                Box::new(Text::new("panel two"))
            }),
        )
}

fn main() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(320.0, 420.0));

    let initial_items = vec![
        "Alpha".to_string(),
        "Bravo".to_string(),
        "Charlie".to_string(),
    ];

    // Root is a single vertical `Stack`; every other component is a child of it,
    // described by builders so the host owns the live widgets.
    let root = Stack::new(Axis::Vertical).gap(8.0);
    let root = root.child(Child::new(
        ChildKey::index(0),
        LayoutRequest::new(),
        || Box::new(Text::new("Count: 0")),
    ));
    let root = root.child(Child::new(ChildKey::index(1), LayoutRequest::new(), {
        let items = initial_items.clone();
        move || {
            Box::new(List::<String>::new(
                Axis::Vertical,
                items.clone(),
                item_key,
                |s: String| Box::new(Text::new(s)),
            ))
        }
    }));
    let root = root.child(Child::new(
        ChildKey::index(2),
        LayoutRequest::new(),
        || {
            Box::new(
                Conditional::new(false).show(Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new(),
                    || Box::new(Text::new("Note: the counter is positive")),
                )),
            )
        },
    ));
    let root = root.child(Child::new(ChildKey::index(3), LayoutRequest::new(), {
        let body = Child::new(ChildKey::index(0), LayoutRequest::new(), || {
            Box::new(Text::new("Dialog body — covers the screen until dismissed."))
        });
        move || Box::new(Dialog::new().title("Hello").body(body.clone()))
    }));
    let root = root.child(Child::new(
        ChildKey::index(4),
        LayoutRequest::new(),
        || Box::new(make_tabs()),
    ));

    let _root = host.mount_root(LayoutStyle::default(), root);

    // Pump one frame so the reconciler mounts the whole subtree, then resolve the
    // composite nodes we will mutate directly.
    let mounted = host.frame();
    let list_id = find_node::<List<String>>(&host);
    let cond_id = find_node::<Conditional>(&host);
    let dialog_id = find_node::<Dialog>(&host);
    let tabs_id = find_node::<Tabs>(&host);

    println!("viewport      : {}x{}", host.bounds(host.root().unwrap()).width() as u32, host.bounds(host.root().unwrap()).height() as u32);
    println!("initial frame : {} live widgets\n", mounted.painted);

    // 1) Increment the counter label — a leaf repaint, no reconciliation.
    let label = find_text_prefix(&host, "Count:");
    host.widget_mut::<Text>(label)
        .expect("counter label")
        .set_content("Count: 1".to_string());
    host.frame();
    println!("[counter] set to 1 — repaint only ({})", host.frame().painted);

    // 2) Append a list row — the reconciler mounts exactly one new row and keeps
    //    the three existing rows alive (their `ChildKey` is unchanged).
    let mut items = initial_items;
    items.push("Delta".to_string());
    host.widget_mut::<List<String>>(list_id)
        .expect("list")
        .set_items(items);
    host.frame();
    println!(
        "[list] appended 'Delta' — live widgets now {}",
        host.frame().painted
    );

    // 3) Reveal the conditional note — mounts a single subtree.
    host.widget_mut::<Conditional>(cond_id)
        .expect("conditional")
        .set_when(true);
    host.frame();
    println!(
        "[conditional] shown — live widgets now {}",
        host.frame().painted
    );

    // 4) Switch tabs by clicking the "Two" tab button — the panel subtree swaps
    //    while the bar and its buttons are rebuilt with the new highlight. (Done
    //    before opening the dialog so the overlay does not intercept the click.)
    let tab = find_button(&host, "Two");
    let bounds = host.bounds(tab);
    let centre = Point::new(
        bounds.x0 + bounds.width() / 2.0,
        bounds.y0 + bounds.height() / 2.0,
    );
    host.dispatch(PointerEvent::primary_down(centre).into());
    host.dispatch(PointerEvent::primary_up(centre).into());
    host.frame();
    let selection = host
        .widget::<Tabs>(tabs_id)
        .expect("tabs")
        .selection()
        .load(std::sync::atomic::Ordering::SeqCst);
    println!("[tabs] clicked 'Two' — selection is now {selection}");

    // 5) Open the dialog — mounts overlay + card + title + body (4 new widgets).
    host.widget_mut::<Dialog>(dialog_id)
        .expect("dialog")
        .set_present(true);
    host.frame();
    println!(
        "[dialog] opened — live widgets now {}",
        host.frame().painted
    );

    // 6) Dismiss the dialog — unmounts the 4 dialog widgets again.
    host.widget_mut::<Dialog>(dialog_id)
        .expect("dialog")
        .set_present(false);
    host.frame();
    println!(
        "[dialog] dismissed — live widgets back to {}",
        host.frame().painted
    );

    // The semantics tree reflects the surviving structure.
    let tree = host.semantics();
    println!("\nsemantics tree : {} published nodes", tree.node_count());
}

/// Finds the mounted node implementing `T` (used for composites we built inline).
fn find_node<T: WidgetAny + 'static>(host: &WidgetHost) -> NodeId {
    for index in 0..host.node_count() {
        let node = host.node_at(index).unwrap();
        if host.widget::<T>(node).is_some() {
            return node;
        }
    }
    panic!("no mounted node of type {}", type_name::<T>());
}

/// Finds a `Text` whose content starts with `prefix`.
fn find_text_prefix(host: &WidgetHost, prefix: &str) -> NodeId {
    for index in 0..host.node_count() {
        let node = host.node_at(index).unwrap();
        if let Some(text) = host.widget::<Text>(node) {
            if text.content().starts_with(prefix) {
                return node;
            }
        }
    }
    panic!("no text starting with {prefix:?}");
}

/// Finds a `Button` by its label.
fn find_button(host: &WidgetHost, label: &str) -> NodeId {
    for index in 0..host.node_count() {
        let node = host.node_at(index).unwrap();
        if let Some(button) = host.widget::<Button>(node) {
            if button.label() == label {
                return node;
            }
        }
    }
    panic!("no button labelled {label:?}");
}

