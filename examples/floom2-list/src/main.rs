//! Golden example: keyed list (Phase 30.1).
//!
//! A list is not "a stack of rows" — it is a *reconciler*. Rows are identified
//! by a stable key, so replacing the backing collection rebuilds only what
//! actually changed. This example measures that: appending one row to a large
//! list must not rebuild the other rows.
//!
//! Run with `cargo run -p floom2-list`.

use floom2::prelude::*;

/// Stable identity for a row: the row's own id, not its position.
fn key_of(item: &Row) -> u64 {
    item.id
}

#[derive(Clone)]
struct Row {
    id: u64,
    label: String,
}

fn rows(count: u64) -> Vec<Row> {
    (0..count)
        .map(|id| Row {
            id,
            label: format!("row {id}"),
        })
        .collect()
}

fn main() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(240.0, 400.0));

    let items = rows(1_000);
    let list = List::new(
        Axis::Vertical,
        items,
        key_of,
        |row: Row| Box::new(Text::new(row.label)),
    )
    .gap(2.0);

    // The list is the root itself: a keyed list is a widget, mounted directly.
    host.mount_root(LayoutStyle::default(), list);

    let mounted = host.frame();
    println!("mounted : {} live widgets for 1000 rows", mounted.painted);

    // Appending one row: the reconciler mounts exactly that row.
    let mut grown = rows(1_001);
    host.widget_mut::<List<Row>>(find_list(&host))
        .expect("list is mounted")
        .set_items(grown.clone());
    let after_append = host.frame();
    println!(
        "append  : {} widgets (+{}), rows now 1001",
        after_append.painted,
        after_append.painted.saturating_sub(mounted.painted)
    );
    assert!(
        after_append.painted > mounted.painted,
        "appending a row must materialize one more widget"
    );
    assert!(
        after_append.painted - mounted.painted <= 4,
        "appending must mount one row, not rebuild the list: grew by {}",
        after_append.painted - mounted.painted
    );

    // Removing a row from the middle: the survivors keep their keys, so their
    // widgets are reused rather than rebuilt.
    let removed = grown.remove(500);
    host.widget_mut::<List<Row>>(find_list(&host))
        .expect("list is mounted")
        .set_items(grown.clone());
    let after_remove = host.frame();
    println!(
        "remove  : {} widgets ({}) after dropping row {}",
        after_remove.painted,
        if after_remove.painted < after_append.painted {
            "released"
        } else {
            "unchanged"
        },
        removed.id
    );
    assert!(
        after_remove.painted < after_append.painted,
        "dropping a row releases exactly that row's widgets"
    );

    // Reordering keeps every row alive: the same keys, a different order.
    grown.reverse();
    host.widget_mut::<List<Row>>(find_list(&host))
        .expect("list is mounted")
        .set_items(grown);
    let after_reorder = host.frame();
    println!("reorder : {} widgets (same set, new order)", after_reorder.painted);
    assert_eq!(
        after_reorder.painted, after_remove.painted,
        "reordering must reuse every existing row widget"
    );

    println!("\nlist golden example: OK");
}

/// Locates the mounted `List<Row>` through the host's public surface.
fn find_list(host: &WidgetHost) -> NodeId {
    for index in 0..host.node_count() {
        let node = host.node_at(index).unwrap();
        if host.widget::<List<Row>>(node).is_some() {
            return node;
        }
    }
    panic!("no mounted List<Row>");
}
