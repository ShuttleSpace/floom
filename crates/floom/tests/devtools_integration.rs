//! The one-line devtools integration, end to end.
//!
//! The promise is one line in and a keystroke away. That promise has two halves
//! that can each fail on their own: `attach` mounting a panel the host can
//! actually see, and the shortcut reaching that panel before the tree does.
//! Both are checkable without a window — the host mounts overlays headlessly,
//! and a key event is a value.
//!
//! Gated on the feature because `floom::devtools` is: without it this file
//! names a module that does not exist, so a plain `cargo test --workspace`
//! fails to compile instead of skipping. The feature is off by default, so
//! `cargo test -p floom --features devtools` is what runs these.

#![cfg(feature = "devtools")]

use floom_app::NodeId;
use floom_app::WidgetHost;
use floom_layout::LayoutStyle;
use floom_types::{Event, Key, KeyEvent, KeyPhase, Modifiers, NamedKey, OverlayHandle, Point, PointerEvent, Size};

/// A host with one widget mounted, the way an app's `build` closure leaves it.
fn host() -> WidgetHost {
    let mut host = WidgetHost::new();
    host.set_size(floom_types::Size::new(800.0, 600.0));
    host.mount_root(LayoutStyle::default(), floom_widgets::Text::new("app"));
    host
}

fn escape() -> KeyEvent {
    KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Named(NamedKey::Escape),
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn one_line_mounts_the_panel_as_an_overlay() {
    let mut host = host();
    assert_eq!(host.overlay_count(), 0);

    // The one line.
    let devtools = floom::devtools::attach(&mut host, |state| {
        vec![floom::devtools::ReadoutRow::new(
            "visible",
            state.visible().to_string(),
        )]
    });

    // The panel is above the app's tree, not part of it.
    assert_eq!(host.overlay_count(), 1);
    // And it starts hidden, so an app that never opens it pays nothing.
    assert!(!devtools.visible());
}

#[test]
fn the_panel_docks_inside_the_window() {
    let mut host = host();
    let _ = floom::devtools::attach(&mut host, |_| Vec::new());
    // One overlay, sized to the window the host was given.
    assert_eq!(host.overlay_count(), 1);
    let bounds = host.overlay_bounds(floom_types::OverlayHandle::new(0));
    assert!(bounds.width() > 0.0 && bounds.width() <= 800.0);
    assert_eq!(bounds.height(), 600.0);
}

#[test]
fn show_false_is_the_default_and_show_true_opens_it() {
    let mut host = host();
    let devtools = floom::devtools::attach(&mut host, |_| Vec::new());
    assert!(!devtools.visible(), "hidden until asked");
    devtools.show();
    assert!(devtools.visible());
    devtools.hide();
    assert!(!devtools.visible());
    devtools.toggle();
    assert!(devtools.visible());
}

#[test]
fn the_shortcut_reaches_the_panel() {
    let mut host = host();
    let mut devtools = floom::devtools::attach(&mut host, |_| Vec::new());
    let f12 = KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Named(NamedKey::Function(12)),
        modifiers: Modifiers::NONE,
    };
    // Consumed, and the panel opened.
    assert!(devtools.handle_key(&f12));
    assert!(devtools.visible());
    // Consumed again, and the panel closed.
    assert!(devtools.handle_key(&f12));
    assert!(!devtools.visible());
}

#[test]
fn an_unrelated_key_falls_through_to_the_app() {
    let mut host = host();
    let mut devtools = floom::devtools::attach(&mut host, |_| Vec::new());
    let tab = KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Named(NamedKey::Tab),
        modifiers: Modifiers::NONE,
    };
    // Not consumed, so a text field keeps typing.
    assert!(!devtools.handle_key(&tab));
    assert!(!devtools.visible());
}

#[test]
fn the_shortcut_makes_the_host_mount_the_panels_rows() {
    let mut host = host();
    let mut devtools = floom::devtools::attach(&mut host, |_| {
        vec![
            floom::devtools::ReadoutRow::heading("Elements"),
            floom::devtools::ReadoutRow::new("row", "one"),
        ]
    });
    let hidden = host.frame().layout_nodes;

    // The shortcut, through the same door the event loop uses.
    let f12 = KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Named(NamedKey::Function(12)),
        modifiers: Modifiers::NONE,
    };
    assert!(devtools.handle_key(&f12));
    host.frame();

    let shown = host.frame().layout_nodes;
    assert!(
        shown > hidden,
        "the host mounts the rows the panel now declares: {hidden} -> {shown}"
    );
}

#[test]
fn a_shown_panel_actually_mounts_its_rows() {
    let mut host = host();
    let devtools = floom::devtools::attach(&mut host, |state| {
        vec![floom::devtools::ReadoutRow::new(
            "tab",
            format!("{:?}", state.tab()),
        )]
    });
    // `show()` is the programmatic door — a menu item, a `?debug=1` flag — and
    // it has to reach the host the same way the shortcut does: through the
    // panel's own `update`, which is the only place that knows its children
    // changed. Asking the widget for `children()` directly would prove the
    // widget can build rows and say nothing about whether the host ever asks.
    let hidden = host.frame().layout_nodes;
    devtools.show();
    let shown = host.frame().layout_nodes;
    assert!(
        shown > hidden,
        "the host mounts the rows the panel now declares: {hidden} -> {shown}"
    );

    // And closing it unmounts them again, so a panel that is down costs what it
    // cost before it was ever opened.
    devtools.hide();
    assert_eq!(host.frame().layout_nodes, hidden);
}

// --- The pick: Ctrl+Shift+C, hover, click, select. -------------------------

/// The picker is a second overlay above the panel, and it is mounted the moment
/// the picker is attached — not when it is armed.
///
/// That is deliberate: the box has to be able to appear on the first hover
/// without the host mounting an overlay mid-gesture, and a passthrough overlay
/// costs nothing while inert (it draws no rect, takes no clicks, takes no
/// focus).
#[test]
fn the_picker_mounts_its_highlight_above_the_panel() {
    let mut host = host();
    let devtools = floom::devtools::attach(&mut host, |_| Vec::new());
    let panel_overlay = floom_types::OverlayHandle::new(0);
    assert_eq!(host.overlay_count(), 1, "just the panel so far");

    let picker = floom::devtools::attach_picker(&mut host, devtools.state());
    assert_eq!(host.overlay_count(), 2, "the panel and the box");
    assert_ne!(
        picker.overlay(),
        panel_overlay,
        "the box is its own overlay, not the panel's"
    );
    // Above the panel: the box is drawn after it, so a highlight over the
    // devtools itself is still visible.
    let box_bounds = host.overlay_bounds(picker.overlay());
    assert_eq!(box_bounds.width(), 800.0, "the box covers the window");
    assert_eq!(box_bounds.height(), 600.0);
}

/// The box follows the pointer while picking, and settles on the pick after.
///
/// This is the whole interaction, through the facade's own handle rather than
/// the raw `pick_pointer` function: arm from a key event, move, click, and read
/// back both the selection the panel will show and the rect the box draws.
#[test]
fn picking_an_element_retargets_the_box_and_selects_it_in_the_panel() {
    let mut host = WidgetHost::new();
    host.set_size(floom_types::Size::new(400.0, 300.0));
    host.mount_root(LayoutStyle::default(), floom_widgets::Text::new("app"));
    host.frame();

    let devtools = floom::devtools::attach(&mut host, |_| Vec::new());
    devtools.show();
    let picker = floom::devtools::attach_picker(&mut host, devtools.state());
    host.frame();

    // Ctrl+Shift+C arms it, and the key is consumed — so a text field that had
    // focus does not also receive the chord.
    let arm = KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Char('c'),
        modifiers: Modifiers::CONTROL.union(Modifiers::SHIFT),
    };
    assert!(picker.handle_key(&arm));
    assert!(picker.armed());

    // A move over the app's root is consumed and retargets the box there.
    let root = host.root().expect("mounted");
    let root_bounds = host.bounds(root);
    let over_root = floom_types::Point::new(root_bounds.x0 + 1.0, root_bounds.y0 + 1.0);
    let moved = floom_types::PointerEvent::hover(over_root);
    let consumed = picker.pointer(&moved, |point| {
        host.element_at(point).map(floom_app::WidgetHost::node_key)
    });
    assert!(consumed, "a picking move is the picker's, not the tree's");
    let target = picker.highlight_target().expect("something is hovered");
    assert_eq!(
        Some(target.clone()),
        host.element_at(over_root).map(floom_app::WidgetHost::node_key),
        "the box is around what the pointer is over"
    );
    assert!(picker.highlight_is_live(), "it is following the hover");

    // And the host can resolve that key back to a box for the highlight.
    picker.sync_highlight(&mut host);
    let box_node = host.overlay_node_id(picker.overlay()).expect("the box is up");
    let drawn = host
        .widget_mut::<floom::devtools::HighlightBox>(box_node)
        .and_then(|box_widget| box_widget.rect());
    assert_eq!(drawn, Some(root_bounds), "the box is around the root");

    // A click commits: consumed, selected in the panel, and no longer picking.
    let press = floom_types::PointerEvent {
        phase: floom_types::PointerPhase::Down,
        kind: floom_types::PointerKind::Mouse,
        button: Some(floom_types::PointerButton::Primary),
        position: over_root,
        modifiers: Modifiers::NONE,
        click_count: 1,
    };
    assert!(picker.pointer(&press, |point| {
        host.element_at(point).map(floom_app::WidgetHost::node_key)
    }));
    assert!(!picker.armed(), "a click stops the pick");
    assert!(picker.commit_selection(), "the pick reached the panel");
    assert_eq!(
        devtools.state().lock().unwrap().selected(),
        Some(target.as_str()),
        "the readout is about the element that was clicked"
    );
    // The box stays where the pick landed, no longer live.
    assert!(!picker.highlight_is_live());
    assert_eq!(picker.highlight_target(), Some(target));
}

/// A key that is not the pick chord is left to the app, and the panel's own
/// chord does not arm the picker.
///
/// Two shortcuts, two jobs: pressing `Ctrl+Shift+I` opens the panel and must not
/// also turn the pointer into a picker.
#[test]
fn the_panel_shortcut_does_not_arm_the_picker() {
    let mut host = host();
    let devtools = floom::devtools::attach(&mut host, |_| Vec::new());
    let picker = floom::devtools::attach_picker(&mut host, devtools.state());

    let ctrl_shift_i = KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Char('i'),
        modifiers: Modifiers::CONTROL.union(Modifiers::SHIFT),
    };
    assert!(!picker.handle_key(&ctrl_shift_i), "not the picker's key");
    assert!(!picker.armed());
}

/// The picker consumes only the three phases it acts on, and only while armed.
///
/// A picker that ate every pointer event would make the app unusable the moment
/// it was attached, and one that ate a release after committing would swallow
/// the `Up` of a click the app never saw the `Down` for.
#[test]
fn a_disarmed_picker_gives_the_pointer_back() {
    let mut host = host();
    let devtools = floom::devtools::attach(&mut host, |_| Vec::new());
    let picker = floom::devtools::attach_picker(&mut host, devtools.state());

    let at = floom_types::Point::new(10.0, 10.0);
    let map = |point: floom_types::Point| {
        host.element_at(point).map(floom_app::WidgetHost::node_key)
    };
    assert!(
        !picker.pointer(&floom_types::PointerEvent::hover(at), map),
        "idle: the app keeps its moves"
    );
    let press = floom_types::PointerEvent {
        phase: floom_types::PointerPhase::Down,
        kind: floom_types::PointerKind::Mouse,
        button: Some(floom_types::PointerButton::Primary),
        position: at,
        modifiers: Modifiers::NONE,
        click_count: 1,
    };
    assert!(
        !picker.pointer(&press, map),
        "idle: a click presses the app"
    );
    assert!(picker.picked().is_none());
}

// --- The panel as chrome: what it takes, and what it gives back ------------

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use floom_core::{Axis, LayoutRequest};
use floom_widgets::Stack;

/// A host whose whole window is one button, so a press anywhere in it is
/// observable. The window-sized box is what makes "the panel ate the column"
/// distinguishable from "the press missed the app".
fn host_with_app(clicks: Arc<AtomicUsize>) -> WidgetHost {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(800.0, 600.0));
    let mut stack = Stack::new(Axis::Vertical);
    // The child builder is `Send + Sync` because the host may build children
    // off the UI thread, so the counter is an `Arc<AtomicUsize>` rather than the
    // `Rc<Cell>` a single-threaded test would reach for.
    //
    // The slot is what makes the button the whole window: `fill` grows it down
    // the stack's main axis, `width_pct(100)` across it. A slot with default
    // layout would shrink-wrap the label at the top-left, and a press in the
    // panel's column would then miss the app for a reason that has nothing to do
    // with the panel.
    stack = stack.push_layout(
        LayoutRequest::new().width_pct(100.0).fill(),
        move || {
        let clicks = clicks.clone();
        Box::new(
            floom_widgets::Button::new("app").on_click(move || {
                clicks.fetch_add(1, Ordering::Relaxed);
            }),
        )
    },
    );
    host.mount_root(LayoutStyle::default(), stack);
    host.frame();
    host
}

fn count(clicks: &Arc<AtomicUsize>) -> usize {
    clicks.load(Ordering::Relaxed)
}

fn click(host: &mut WidgetHost, at: Point) -> bool {
    let down = host.dispatch(Event::Pointer(PointerEvent::primary_down(at)));
    let up = host.dispatch(Event::Pointer(PointerEvent::primary_up(at)));
    down || up
}

fn centre_of(host: &WidgetHost, node: NodeId) -> Point {
    let bounds = host.bounds(node);
    Point::new(
        bounds.x0 + bounds.width() / 2.0,
        bounds.y0 + bounds.height() / 2.0,
    )
}

/// Every `Button` in the overlay's subtree, in tree order.
fn overlay_buttons(host: &WidgetHost, id: OverlayHandle) -> Vec<(NodeId, String)> {
    let Some(root) = host.overlay_node_id(id) else {
        return Vec::new();
    };
    let mut queue = std::collections::VecDeque::from([root]);
    let mut found = Vec::new();
    while let Some(node) = queue.pop_front() {
        if let Some(button) = host.widget::<floom_widgets::Button>(node) {
            found.push((node, button.label().to_string()));
        }
        for index in 0..host.child_count(node) {
            if let Some(child) = host.child_at(node, index) {
                queue.push_back(child);
            }
        }
    }
    found
}

#[test]
fn a_hidden_panel_does_not_eat_the_column_it_docks_in() {
    let clicks = Arc::new(AtomicUsize::new(0));
    let mut host = host_with_app(clicks.clone());
    let mut devtools = floom::devtools::attach(&mut host, |_| Vec::new());
    devtools.sync(&mut host);
    host.frame();

    // Inside the drawer's bounds, which span the trailing edge whether the panel
    // paints anything there or not.
    let bounds = host.overlay_bounds(devtools.overlay());
    let at = Point::new(bounds.x0 + bounds.width() / 2.0, 300.0);
    assert!(click(&mut host, at));
    assert_eq!(
        count(&clicks),
        1,
        "a drawer that is not showing must not claim its column"
    );
}

#[test]
fn a_shown_panel_takes_its_column_back() {
    let clicks = Arc::new(AtomicUsize::new(0));
    let mut host = host_with_app(clicks.clone());
    let mut devtools = floom::devtools::attach(&mut host, |_| Vec::new());
    devtools.show();
    devtools.sync(&mut host);
    host.frame();

    let bounds = host.overlay_bounds(devtools.overlay());
    let at = Point::new(bounds.x0 + bounds.width() / 2.0, 300.0);
    click(&mut host, at);
    assert_eq!(count(&clicks), 0, "the panel owns its column while it is up");
}

#[test]
fn a_press_outside_the_visible_panel_does_not_unmount_it() {
    let clicks = Arc::new(AtomicUsize::new(0));
    let mut host = host_with_app(clicks.clone());
    let mut devtools = floom::devtools::attach(&mut host, |_| Vec::new());
    devtools.show();
    devtools.sync(&mut host);
    host.frame();

    assert!(click(&mut host, Point::new(100.0, 300.0)));
    assert_eq!(count(&clicks), 1, "the press reached the app");
    assert!(devtools.visible(), "and did not close the panel");
    assert_eq!(host.overlay_count(), 1, "the panel is still mounted");
}

#[test]
fn escape_closes_the_panel_wherever_the_keyboard_is() {
    let clicks = Arc::new(AtomicUsize::new(0));
    let mut host = host_with_app(clicks.clone());
    let mut devtools = floom::devtools::attach(&mut host, |_| Vec::new());
    // While the panel is down, Escape belongs to the app: swallowing it would
    // break every dialog and menu the app owns.
    assert!(!devtools.handle_key(&escape()), "hidden: not ours to take");

    devtools.show();
    devtools.sync(&mut host);
    host.frame();
    // The keyboard is still the app's — `attach` handed it back — and the panel
    // closes anyway. That is the whole reason Escape lives here rather than in
    // the panel's `event`: a key routed to the app never reaches the panel.
    assert!(devtools.handle_key(&escape()), "shown: the panel closes");
    assert!(!devtools.visible());

    devtools.sync(&mut host);
    host.frame();
    let bounds = host.overlay_bounds(devtools.overlay());
    click(&mut host, Point::new(bounds.x0 + 10.0, 300.0));
    assert_eq!(
        count(&clicks),
        1,
        "and the column it was docking in goes back to the app"
    );
}

#[test]
fn opening_the_panel_leaves_the_keyboard_with_the_app() {
    let mut host = host_with_app(Arc::new(AtomicUsize::new(0)));
    host.focus_next();
    let before = host.focused();
    assert!(before.is_some(), "the app button is a focus stop");

    let _devtools = floom::devtools::attach(&mut host, |_| Vec::new());
    assert_eq!(
        host.focused(),
        before,
        "the panel is chrome: attaching it must not take the keyboard"
    );
}

#[test]
fn the_tab_bar_switches_the_readout_in_a_live_host() {
    let mut host = host_with_app(Arc::new(AtomicUsize::new(0)));
    let mut devtools =
        floom::devtools::attach_with_tabs(&mut host, |_| Vec::new(), floom::devtools::DevtoolsTab::ELEMENTS);
    devtools.show();
    devtools.sync(&mut host);
    host.frame();

    let buttons = overlay_buttons(&host, devtools.overlay());
    let labels: Vec<&str> = buttons.iter().map(|(_, label)| label.as_str()).collect();
    assert_eq!(labels, vec!["Elements", "Style", "Layout"]);

    let style = buttons
        .iter()
        .find(|(_, label)| label == "Style")
        .map(|(node, _)| *node)
        .expect("the bar offers Style");
    let at = centre_of(&host, style);
    click(&mut host, at);
    assert_eq!(devtools.tab(), floom::devtools::DevtoolsTab::Style);

    // The rows the next frame mounts are the ones for the tab that is now up:
    // the panel noticed the change in its `update`, which is the only place it
    // can — the click never passed through the panel widget.
    host.frame();
    let buttons = overlay_buttons(&host, devtools.overlay());
    assert_eq!(buttons.len(), 3, "the bar survived the rebuild");
}
