//! Reconciliation across the composite components.
//!
//! These run the real [`WidgetHost`] with the real `floem-widgets` composites and
//! assert what the reconciler mounts/unmounts. Because removed subtrees are only
//! flagged `dead` (their `NodeId` slot is kept for stability), we count *live*
//! widgets via [`FrameStats::painted`], which the host increments once per
//! non-dead node.

use std::sync::atomic::Ordering;

use floem_app::{NodeId, WidgetHost};
use floem_core::{Axis, Child, ChildKey, LayoutRequest, Point, Size};
use floem_layout::LayoutStyle;
use floem_types::PointerEvent;
use floem_widgets::{
    BottomSheet, Button, Conditional, Dialog, Grid, List, NavigationBar, NavigationDrawer, Popup,
    Stack, Tabs, Text,
};

/// Stable FNV-1a key for the string items used by the list/grid tests.
fn key_of(s: &String) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for byte in s.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn host_viewport() -> WidgetHost {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(240.0, 320.0));
    host
}

fn click(host: &mut WidgetHost, node: NodeId) -> bool {
    let bounds = host.bounds(node);
    let centre = Point::new(
        bounds.x0 + bounds.width() / 2.0,
        bounds.y0 + bounds.height() / 2.0,
    );
    let down = host.dispatch(PointerEvent::primary_down(centre).into());
    let up = host.dispatch(PointerEvent::primary_up(centre).into());
    down && up
}

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

#[test]
fn stack_mounts_every_child_once() {
    let mut host = host_viewport();
    let mut stack = Stack::new(Axis::Vertical).gap(4.0);
    for i in 0..3 {
        stack = stack.push(move || Box::new(Text::new(format!("row {i}"))));
    }
    host.mount_root(LayoutStyle::default(), stack);
    let stats = host.frame();
    assert_eq!(stats.painted, 4, "1 stack + 3 texts");
}

#[test]
fn list_mounts_unmounts_on_diff_keeping_state_slots() {
    let mut host = host_viewport();
    let list = List::<String>::new(
        Axis::Vertical,
        vec!["a".into(), "b".into(), "c".into()],
        |s: &String| {
            let mut h = 0u64;
            for b in s.bytes() {
                h = h.wrapping_mul(31).wrapping_add(b as u64);
            }
            h
        },
        |s: String| Box::new(Text::new(s)),
    );
    let list_id = host.mount_root(LayoutStyle::default(), list);
    let stats = host.frame();
    assert_eq!(stats.painted, 4, "1 list + 3 rows");

    // Insert one row in the middle: only the new row should mount.
    host.widget_mut::<List<String>>(list_id)
        .expect("list node")
        .set_items(vec!["a".into(), "b".into(), "x".into(), "c".into()]);
    let stats = host.frame();
    assert_eq!(stats.painted, 5, "1 list + 4 rows");

    // Remove two rows: the surviving slots stay mounted, the rest unmount.
    host.widget_mut::<List<String>>(list_id)
        .expect("list node")
        .set_items(vec!["a".into(), "c".into()]);
    let stats = host.frame();
    assert_eq!(stats.painted, 3, "1 list + 2 rows");
}

#[test]
fn conditional_toggles_its_subtree() {
    let mut host = host_viewport();
    let cond =
        Conditional::new(false).show(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
            Box::new(Text::new("hi"))
        }));
    let id = host.mount_root(LayoutStyle::default(), cond);
    let stats = host.frame();
    assert_eq!(stats.painted, 1, "hidden conditional is alone");

    host.widget_mut::<Conditional>(id)
        .expect("cond node")
        .set_when(true);
    let stats = host.frame();
    assert_eq!(stats.painted, 2, "conditional + its child mounted");

    host.widget_mut::<Conditional>(id)
        .expect("cond node")
        .set_when(false);
    let stats = host.frame();
    assert_eq!(stats.painted, 1, "subtree unmounted again");
}

#[test]
fn dialog_mounts_a_full_overlay_subtree_when_shown() {
    let mut host = host_viewport();
    let dialog = Dialog::new().title("Hello").body(Child::new(
        ChildKey::index(0),
        LayoutRequest::new(),
        || Box::new(Text::new("body")),
    ));
    let id = host.mount_root(LayoutStyle::default(), dialog);
    let stats = host.frame();
    assert_eq!(stats.painted, 1, "hidden dialog is alone");

    host.widget_mut::<Dialog>(id)
        .expect("dialog node")
        .set_present(true);
    let stats = host.frame();
    // dialog + overlay + card stack + title text + body text
    assert_eq!(stats.painted, 5, "dialog surface fully mounted");

    host.widget_mut::<Dialog>(id)
        .expect("dialog node")
        .set_present(false);
    let stats = host.frame();
    assert_eq!(stats.painted, 1, "dialog surface fully unmounted");
}

#[test]
fn tabs_swap_the_active_panel_on_click() {
    let mut host = host_viewport();
    let tabs = Tabs::new()
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
        );
    let id = host.mount_root(LayoutStyle::default(), tabs);
    let stats = host.frame();
    // tabs + bar(stack) + 2 buttons + active panel
    assert_eq!(stats.painted, 5);

    let selection = host.widget::<Tabs>(id).expect("tabs node").selection();
    assert_eq!(selection.load(Ordering::SeqCst), 0, "starts on tab zero");

    let labels_before = semantics_labels(&host);
    assert!(
        labels_before.iter().any(|l| l == "panel one"),
        "first panel is visible"
    );
    assert!(
        !labels_before.iter().any(|l| l == "panel two"),
        "second panel is not mounted yet"
    );

    // Click the inactive tab to switch.
    let other = find_button(&host, "Two");
    assert!(click(&mut host, other), "tab button must be hittable");
    host.frame();

    assert_eq!(
        selection.load(Ordering::SeqCst),
        1,
        "clicking the tab bar flips the selection"
    );

    let labels_after = semantics_labels(&host);
    assert!(
        labels_after.iter().any(|l| l == "panel two"),
        "active panel swapped in"
    );
    assert!(
        !labels_after.iter().any(|l| l == "panel one"),
        "old panel swapped out"
    );
    // Live widget count is unchanged by a swap (bar + 2 buttons + panel).
    assert_eq!(host.frame().painted, 5);
}

#[test]
fn grid_mounts_rows_and_reconciles_on_diff() {
    let mut host = host_viewport();
    let grid = Grid::<String>::new(
        2,
        vec!["a".into(), "b".into(), "c".into()],
        key_of,
        |s: String| Box::new(Text::new(s)),
    );
    let id = host.mount_root(LayoutStyle::default(), grid);
    // grid + row(a,b) + row(c) + 3 cells
    assert_eq!(host.frame().painted, 6, "two rows over three cells");

    // Adding one cell grows only the affected row: 2 rows, 4 cells.
    host.widget_mut::<Grid<String>>(id)
        .expect("grid node")
        .set_items(vec!["a".into(), "b".into(), "c".into(), "d".into()]);
    assert_eq!(host.frame().painted, 7, "second row gained a cell");

    // Shrinking to a single row unmounts the other row's whole subtree.
    host.widget_mut::<Grid<String>>(id)
        .expect("grid node")
        .set_items(vec!["a".into(), "b".into()]);
    assert_eq!(host.frame().painted, 4, "one row + two cells + grid");
}

#[test]
fn popup_mounts_and_dismisses_its_layer() {
    let mut host = host_viewport();
    let popup = Popup::new().label("menu").content(Child::new(
        ChildKey::index(0),
        LayoutRequest::new(),
        || Box::new(Text::new("item")),
    ));
    let id = host.mount_root(LayoutStyle::default(), popup);
    assert_eq!(host.frame().painted, 1, "hidden popup is alone");

    host.widget_mut::<Popup>(id)
        .expect("popup node")
        .set_present(true);
    // popup + overlay layer + content
    assert_eq!(host.frame().painted, 3, "popup layer mounted");

    host.widget_mut::<Popup>(id).expect("popup node").dismiss();
    assert_eq!(host.frame().painted, 1, "popup layer unmounted");
}

#[test]
fn bottom_sheet_mounts_a_bottom_layer_when_shown() {
    let mut host = host_viewport();
    let sheet = BottomSheet::new()
        .title("Share")
        .height(160.0)
        .body(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
            Box::new(Text::new("body"))
        }));
    let id = host.mount_root(LayoutStyle::default(), sheet);
    assert_eq!(host.frame().painted, 1, "hidden sheet is alone");

    host.widget_mut::<BottomSheet>(id)
        .expect("sheet node")
        .set_present(true);
    // sheet + overlay + bottom column + card + title + body
    assert_eq!(host.frame().painted, 6, "sheet layer fully mounted");

    host.widget_mut::<BottomSheet>(id)
        .expect("sheet node")
        .set_present(false);
    assert_eq!(host.frame().painted, 1, "sheet layer fully unmounted");
}

#[test]
fn navigation_bar_swaps_the_highlight_on_click() {
    let mut host = host_viewport();
    let bar = NavigationBar::new()
        .destination("Home")
        .destination("Search");
    let id = host.mount_root(LayoutStyle::default(), bar);
    assert_eq!(host.frame().painted, 3, "bar + two destinations");

    let selection = host
        .widget::<NavigationBar>(id)
        .expect("bar node")
        .selection();
    assert_eq!(selection.load(Ordering::SeqCst), 0);

    let labels = semantics_labels(&host);
    assert!(labels.iter().any(|l| l == "● Home"), "first is highlighted");

    let search = find_button(&host, "Search");
    assert!(click(&mut host, search), "destination must be hittable");
    host.frame();
    assert_eq!(
        selection.load(Ordering::SeqCst),
        1,
        "clicking a destination flips selection"
    );
    assert!(
        semantics_labels(&host).iter().any(|l| l == "● Search"),
        "highlight moved to the new destination"
    );
    assert_eq!(host.frame().painted, 3, "still bar + two destinations");
}

#[test]
fn navigation_drawer_mounts_and_dismisses() {
    let mut host = host_viewport();
    let drawer = NavigationDrawer::new()
        .header("Menu")
        .destination("Home")
        .destination("Settings");
    let id = host.mount_root(LayoutStyle::default(), drawer);
    assert_eq!(host.frame().painted, 1, "hidden drawer is alone");

    host.widget_mut::<NavigationDrawer>(id)
        .expect("drawer node")
        .set_present(true);
    // drawer + overlay + column + panel + header + two destinations
    assert_eq!(host.frame().painted, 7, "drawer panel fully mounted");

    host.widget_mut::<NavigationDrawer>(id)
        .expect("drawer node")
        .set_present(false);
    assert_eq!(host.frame().painted, 1, "drawer panel fully unmounted");
}

/// Walks the published semantics tree collecting every non-empty label.
fn semantics_labels(host: &WidgetHost) -> Vec<String> {
    let tree = host.semantics();
    let mut out = Vec::new();
    if let Some(root) = tree.root() {
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            if let Some(node) = tree.get(id) {
                if !node.label.is_empty() {
                    out.push(node.label.clone());
                }
                for child in &node.children {
                    stack.push(*child);
                }
            }
        }
    }
    out
}
