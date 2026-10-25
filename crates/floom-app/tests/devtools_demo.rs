//! The devtools demo's wiring, headless.
//!
//! The demo itself needs a GPU window, which CI does not have. What *is*
//! checkable without one is the whole chain the demo relies on: mount an app,
//! attach the panel, refresh the tree, read the composed rows, and pick an
//! element by clicking where it is. If this passes, the demo's two lines are
//! doing what they claim.

use std::sync::{Arc, Mutex};

use floom_app::{HostedNode, WidgetHost};
use floom_core::{Axis, Child, ChildKey, LayoutRequest, Widget};
use floom_hot_reload::describe;
use floom_layout::LayoutStyle;
use floom_types::{Point, Size};
use floom_widgets::{Button, Stack, Text};

/// The demo's app: a heading, a row of three buttons, and a counter line.
fn demo_app(host: &mut WidgetHost) {
    let mut column = Stack::new(Axis::Vertical)
        .gap(12.0)
        .child(Child::new(ChildKey::index(0), LayoutRequest::new().height(28.0), || {
            Box::new(Text::new("devtools demo — press F12"))
        }));

    let mut row = Stack::new(Axis::Horizontal).gap(8.0);
    for (index, label) in ["one", "two", "three"].iter().enumerate() {
        row = row.child(Child::new(
            ChildKey::index(index),
            LayoutRequest::new().width(96.0).height(36.0),
            move || Box::new(Button::new(*label)),
        ));
    }
    let row = Arc::new(row);
    let row_slot = row.clone();
    column = column.child(Child::new(
        ChildKey::index(1),
        LayoutRequest::new().height(36.0),
        move || Box::new((*row_slot).clone()),
    ));

    host.set_size(Size::new(720.0, 560.0));
    host.mount_root(LayoutStyle::default(), column);
    host.frame();
}

#[test]
fn the_demos_app_mounts_and_is_describable() {
    let mut host = WidgetHost::new();
    demo_app(&mut host);

    let nodes: Vec<HostedNode> = host.tree_nodes();
    assert!(!nodes.is_empty(), "the app mounted");
    // The heading, the row and the three buttons: the tree panel's content.
    assert!(
        nodes.len() >= 5,
        "a heading, a row, three buttons: {:?}",
        nodes.len()
    );

    // And describing them answers the questions the panel asks.
    let live = describe(&nodes);
    let inspector = floom_devtools::Inspector::new(&live);
    let rows = floom_devtools::panel::tree_rows(&inspector, None);
    assert!(!rows.is_empty(), "the tree panel has rows");
}

#[test]
fn the_panel_composes_the_demos_rows_over_the_engines() {
    let mut host = WidgetHost::new();
    demo_app(&mut host);

    // The demo's own rows, then the engine's — the shape
    // `run_desktop_with_devtools` builds.
    let cell = Arc::new(Mutex::new(host.tree_nodes()));
    let readout_cell = cell.clone();
    let rows = (|state: &floom_devtools::panel::DevtoolsState| {
        let mut rows = vec![
            floom_devtools::panel::ReadoutRow::heading("demo"),
            floom_devtools::panel::ReadoutRow::new("clicks", "0"),
            floom_devtools::panel::ReadoutRow::new("tab", format!("{:?}", state.tab())),
        ];
        let nodes = readout_cell.lock().map(|n| n.clone()).unwrap_or_default();
        let live = describe(&nodes);
        let inspector = floom_devtools::Inspector::new(&live);
        rows.extend(floom_devtools::panel::elements_readout(&inspector, state));
        rows
    })(&floom_devtools::panel::DevtoolsState::new());

    // The demo's heading is first, then its rows, then the engine's tree.
    assert_eq!(rows[0].label, "demo");
    assert!(rows[0].heading);
    assert_eq!(rows[1].label, "clicks");
    assert!(
        rows.iter().any(|r| r.label == "Elements"),
        "the engine's tree follows the app's rows: {:?}",
        rows.iter().map(|r| r.label.clone()).collect::<Vec<_>>()
    );
}

#[test]
fn refreshing_the_cell_keeps_the_panel_current() {
    let mut host = WidgetHost::new();
    demo_app(&mut host);

    let cell = Arc::new(Mutex::new(host.tree_nodes()));
    let before = cell.lock().unwrap().len();

    // The app changes: a second row of buttons mounts.
    let mut extra = Stack::new(Axis::Horizontal).gap(8.0);
    for (index, label) in ["four", "five"].iter().enumerate() {
        extra = extra.child(Child::new(
            ChildKey::index(index),
            LayoutRequest::new().width(96.0).height(36.0),
            move || Box::new(Button::new(*label)),
        ));
    }
    let root = host.root().expect("the demo mounted a root");
    host.mount(root, LayoutStyle::default(), extra);
    host.frame();

    // Refresh, and the panel sees the new tree rather than the startup one.
    if let Ok(mut nodes) = cell.lock() {
        *nodes = host.tree_nodes();
    }
    let after = cell.lock().unwrap().len();
    assert!(
        after > before,
        "the refreshed cell has the new nodes: {before} -> {after}"
    );
}

#[test]
fn clicking_a_button_picks_it_without_pressing_it() {
    let mut host = WidgetHost::new();
    demo_app(&mut host);

    // Where the first button is: the row starts at the top of the column, after
    // the heading's 28px and the 12px gap.
    let pick = Arc::new(Mutex::new(floom_devtools::pick::PickState::new()));
    pick.lock().unwrap().arm();

    // The host's own hit test is what a click uses, so a pick lands on the same
    // node a press would.
    let at = Point::new(48.0, 40.0);
    // The same key the facade's pointer hook builds: `WidgetHost::node_key`,
    // not a private `node-{raw}` spelling, so the pick names the node the panel
    // can look back up with `node_by_key`.
    let key_of = |p: Point| host.node_at_point(p).map(|n| WidgetHost::node_key(host.node_id(n)));
    assert!(key_of(at).is_some(), "the point is over the button row: {at:?}");

    // And the picker consumes the event, so the button never sees it.
    let consumed = floom_devtools::pick::pick_pointer(&pick, &press(at), key_of);
    assert!(consumed, "a picking click is the picker's, not the button's");
    assert!(pick.lock().unwrap().picked().is_some(), "and it selected");
}

fn press(at: Point) -> floom_types::PointerEvent {
    floom_types::PointerEvent {
        phase: floom_types::PointerPhase::Down,
        kind: floom_types::PointerKind::Mouse,
        button: Some(floom_types::PointerButton::Primary),
        position: at,
        modifiers: floom_types::Modifiers::NONE,
        click_count: 1,
    }
}
