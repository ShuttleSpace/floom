//! The interactive half of the Elements tree: rows that select and collapse, and
//! the menu a right click opens.
//!
//! Everything here runs without a host. A row is a widget, a press is a value,
//! and the state it writes is a shared cell a test can read back — which is
//! exactly the contract that was missing when the tree was plain text and the
//! Style and Layout tabs looked like they did nothing.

use std::sync::{Arc, Mutex};

use floom_core::{ElementId, Widget};
use floom_devtools::panel::DevtoolsState;
use floom_devtools::tree::{
    CHEVRON_WIDTH, ContextMenu, INDENT, LEADING, MENU_ITEM_HEIGHT, MENU_PADDING, MENU_WIDTH,
    RowNode, TreeRow, menu_bounds, row_child,
};
use floom_types::{Event, OverlayKind, Point, PointerButton, PointerEvent, Rect, Size};
use floom_widgets::testkit::{OverlayCall, SceneRecorder, TestEventContext, dispatch};

const ID: ElementId = ElementId::new(1, 1);

fn row_bounds() -> Rect {
    Rect::from_origin_size(Point::new(0.0, 100.0), Size::new(240.0, 20.0))
}

fn shared() -> Arc<Mutex<DevtoolsState>> {
    Arc::new(Mutex::new(DevtoolsState::new()))
}

fn node(key: &str, has_children: bool, expanded: bool) -> RowNode {
    RowNode {
        key: key.to_string(),
        has_children,
        expanded,
        selected: false,
    }
}

fn row(state: &Arc<Mutex<DevtoolsState>>, node: RowNode, depth: usize) -> TreeRow {
    TreeRow::new(state.clone(), node, "host", depth)
}

/// The x of the middle of the chevron column at `depth`.
fn chevron_x(depth: usize) -> f64 {
    LEADING + depth as f64 * INDENT + CHEVRON_WIDTH / 2.0
}

/// The x of a press on the key, clear of the chevron.
fn key_x(depth: usize) -> f64 {
    LEADING + depth as f64 * INDENT + CHEVRON_WIDTH + 30.0
}

fn press(button: PointerButton, at: Point) -> Event {
    let mut event = PointerEvent::primary_down(at);
    event.button = Some(button);
    Event::Pointer(event)
}

fn click(widget: &mut dyn Widget, at: Point) -> bool {
    dispatch(widget, ID, row_bounds(), press(PointerButton::Primary, at))
}

fn right_click(widget: &mut TreeRow, at: Point) -> TestEventContext {
    let mut cx = TestEventContext::pointer(ID, row_bounds(), {
        let mut event = PointerEvent::primary_down(at);
        event.button = Some(PointerButton::Secondary);
        event
    });
    widget.event(&mut cx, ID);
    cx
}

/// The entries a row's menu offers, read off the overlay it opened.
fn menu_labels(widget: &mut TreeRow) -> Vec<&'static str> {
    let mut cx = right_click(widget, Point::new(120.0, 110.0));
    let content = cx.take_content().expect("a right press opens a menu");
    let menu = content
        .as_any()
        .downcast_ref::<ContextMenu>()
        .expect("the overlay content is the menu");
    menu.items().iter().map(|item| item.label).collect()
}

fn read(state: &Arc<Mutex<DevtoolsState>>) -> DevtoolsState {
    state.lock().unwrap().clone()
}

#[test]
fn a_press_on_the_key_selects_the_row() {
    let state = shared();
    let mut widget = row(&state, node("body", false, true), 0);
    assert!(click(&mut widget, Point::new(key_x(0), 110.0)));
    assert_eq!(read(&state).selected(), Some("body"));
    assert!(
        read(&state).collapsed().is_empty(),
        "selecting collapses nothing"
    );
}

#[test]
fn a_press_on_the_chevron_collapses_a_parent_and_keeps_the_selection() {
    let state = shared();
    let mut widget = row(&state, node("body", true, true), 0);
    assert!(click(&mut widget, Point::new(chevron_x(0), 110.0)));
    assert!(read(&state).is_collapsed("body"));
    assert_eq!(
        read(&state).selected(),
        None,
        "the chevron is a disclosure control, not a selection"
    );

    // And again, to open it back up.
    assert!(click(&mut widget, Point::new(chevron_x(0), 110.0)));
    assert!(!read(&state).is_collapsed("body"));
}

#[test]
fn a_press_on_a_parents_key_selects_rather_than_collapsing() {
    // Chrome's rule: the row is the selection, the triangle is the disclosure.
    // Clicking the name of a container to read its styles must not also fold
    // away the children the reader was looking at.
    let state = shared();
    let mut widget = row(&state, node("body", true, true), 0);
    assert!(click(&mut widget, Point::new(key_x(0), 110.0)));
    assert_eq!(read(&state).selected(), Some("body"));
    assert!(!read(&state).is_collapsed("body"));
}

#[test]
fn a_leaf_has_no_chevron_so_that_column_selects() {
    // A leaf draws a blank of the same width so the keys line up; a press there
    // is a press on the row, because there is nothing to disclose.
    let state = shared();
    let mut widget = row(&state, node("heading", false, true), 0);
    assert!(click(&mut widget, Point::new(chevron_x(0), 110.0)));
    assert_eq!(read(&state).selected(), Some("heading"));
    assert!(read(&state).collapsed().is_empty());
}

#[test]
fn the_indent_moves_the_chevron_and_the_key_together() {
    // Depth is the only indent, so a nested row's controls have to move with it
    // or a deep row would be selected by a press aimed at its parent's triangle.
    let state = shared();
    let mut deep = row(&state, node("ok", true, true), 2);
    assert!(click(&mut deep, Point::new(chevron_x(2), 110.0)));
    assert!(read(&state).is_collapsed("ok"));

    // The same x on a top-level row is past its chevron: it selects.
    let state = shared();
    let mut top = row(&state, node("root", true, true), 0);
    assert!(click(&mut top, Point::new(chevron_x(2), 110.0)));
    assert_eq!(read(&state).selected(), Some("root"));
    assert!(!read(&state).is_collapsed("root"));
}

#[test]
fn a_middle_press_is_left_for_somebody_else() {
    let state = shared();
    let mut widget = row(&state, node("body", true, true), 0);
    let middle = dispatch(
        &mut widget,
        ID,
        row_bounds(),
        press(PointerButton::Middle, Point::new(key_x(0), 110.0)),
    );
    assert!(!middle, "a middle press is not a row's business");
    assert_eq!(read(&state).selected(), None);
}

#[test]
fn a_release_is_consumed_so_the_app_never_sees_a_click_it_missed() {
    // The press was taken by the row; the release has to be taken too, or a
    // widget behind the panel would act on a click whose beginning it never saw.
    let state = shared();
    let mut widget = row(&state, node("body", false, true), 0);
    let up = dispatch(
        &mut widget,
        ID,
        row_bounds(),
        Event::Pointer(PointerEvent::primary_up(Point::new(key_x(0), 110.0))),
    );
    assert!(up);
}

#[test]
fn a_row_ignores_the_keyboard() {
    let state = shared();
    let mut widget = row(&state, node("body", false, true), 0);
    let event = Event::Key(floom_types::KeyEvent {
        phase: floom_types::KeyPhase::Down,
        key: floom_types::Key::Char('a'),
        modifiers: floom_types::Modifiers::NONE,
    });
    assert!(!dispatch(&mut widget, ID, row_bounds(), event));
}

#[test]
fn a_row_asks_for_a_repaint_so_the_panel_rebuilds() {
    // A widget cannot mark the host dirty from `event`; the repaint channel is
    // the only wake-up it has. Without this the state changes and nothing draws
    // it, which is indistinguishable from a click that did nothing.
    floom_core::take_repaint_requests();
    let state = shared();
    let mut widget = row(&state, node("body", false, true), 0);
    click(&mut widget, Point::new(key_x(0), 110.0));
    assert_eq!(floom_core::take_repaint_requests(), vec![ID]);
    assert!(!floom_core::has_repaint_requests());
}

#[test]
fn the_selected_row_paints_its_background_before_its_label() {
    // The host paints a flat scene in node order, so the only way a row can put
    // a fill *behind* its own text is to draw both itself, in that order.
    let state = shared();
    let mut selected = node("body", true, true);
    selected.selected = true;
    let widget = row(&state, selected, 0);
    let mut recorder = SceneRecorder::new(row_bounds());
    widget.paint(&mut recorder, ID);
    assert_eq!(
        recorder.scene_node_kinds(),
        vec!["Rect", "Text", "Text", "Text"],
        "fill, chevron, key, kind"
    );
}

#[test]
fn an_unselected_row_paints_no_background() {
    let state = shared();
    // A leaf with no kind to print: the chevron column and the key, and nothing
    // behind either.
    let widget = TreeRow::new(state.clone(), node("body", false, true), "", 0);
    let mut recorder = SceneRecorder::new(row_bounds());
    widget.paint(&mut recorder, ID);
    assert_eq!(
        recorder.scene_node_kinds(),
        vec!["Text", "Text"],
        "chevron and key, nothing behind them"
    );
}

#[test]
fn a_right_press_opens_a_non_modal_popup_at_the_pointer() {
    let state = shared();
    let mut widget = row(&state, node("body", true, true), 0);
    let at = Point::new(120.0, 110.0);
    let cx = right_click(&mut widget, at);

    let opened = cx.overlay_calls();
    assert_eq!(opened.len(), 1);
    match &opened[0] {
        OverlayCall::Open {
            kind,
            modal,
            bounds,
            ..
        } => {
            assert_eq!(*kind, OverlayKind::Popup);
            assert!(
                !modal,
                "a modal menu would swallow the press that dismisses it"
            );
            assert_eq!(
                *bounds,
                menu_bounds(at, Rect::ZERO, 2),
                "a parent with nothing selected offers collapse and select"
            );
        }
        OverlayCall::Close(_) => unreachable!("the only call is an open"),
    }
}

#[test]
fn the_menu_offers_only_the_actions_it_can_perform() {
    // A leaf has nothing to disclose, so no expand entry; with nothing selected
    // there is nothing to clear. A menu of dead entries is worse than a short one.
    let state = shared();
    assert_eq!(
        menu_labels(&mut row(&state, node("heading", false, true), 0)),
        vec!["Select element"]
    );

    let state = shared();
    assert_eq!(
        menu_labels(&mut row(&state, node("body", true, true), 0)),
        vec!["Collapse", "Select element"]
    );

    // Collapsed, the same node offers to open.
    let state = shared();
    state.lock().unwrap().toggle_collapsed("body");
    assert_eq!(
        menu_labels(&mut row(&state, node("body", true, false), 0)),
        vec!["Expand", "Select element"]
    );

    // And a selection gives the menu something to clear.
    state.lock().unwrap().select("heading");
    assert_eq!(
        menu_labels(&mut row(&state, node("body", true, false), 0)),
        vec!["Expand", "Select element", "Clear selection"]
    );
}

// ---------------------------------------------------------------------------
// The menu itself
// ---------------------------------------------------------------------------

fn menu(state: &Arc<Mutex<DevtoolsState>>, items: Vec<&'static str>) -> ContextMenu {
    let items = items
        .into_iter()
        .map(|label| floom_devtools::tree::MenuItem {
            label,
            action: floom_devtools::tree::MenuAction::ClearSelection,
        })
        .collect();
    ContextMenu::new(state.clone(), items, floom_types::OverlayHandle::new(1))
}

fn menu_bounds_for(count: usize) -> Rect {
    Rect::from_origin_size(
        Point::new(100.0, 100.0),
        Size::new(
            MENU_WIDTH,
            MENU_PADDING * 2.0 + MENU_ITEM_HEIGHT * count as f64,
        ),
    )
}

fn press_in_menu(at: Point) -> Event {
    Event::Pointer(PointerEvent::primary_down(at))
}

#[test]
fn the_menu_sits_where_it_was_put_and_picks_by_row() {
    let state = shared();
    let widget = menu(&state, vec!["one", "two", "three"]);
    let bounds = menu_bounds_for(3);
    let mut recorder = SceneRecorder::new(bounds);
    widget.paint(&mut recorder, ID);
    assert_eq!(
        recorder.scene_node_kinds(),
        vec!["Rect", "Text", "Text", "Text"],
        "the panel, then one run per entry"
    );
}

#[test]
fn a_press_on_an_entry_applies_it_and_closes_the_menu() {
    let state = shared();
    state.lock().unwrap().select("body");
    let mut widget = menu(&state, vec!["one", "two", "three"]);
    let bounds = menu_bounds_for(3);

    // Second entry: one padding, then one full row down.
    let at = Point::new(
        bounds.x0 + 10.0,
        bounds.y0 + MENU_PADDING + MENU_ITEM_HEIGHT * 1.5,
    );
    let mut cx = TestEventContext::new(ID, bounds, press_in_menu(at));
    assert!(widget.event(&mut cx, ID));

    assert_eq!(
        cx.closed_handles(),
        vec![floom_types::OverlayHandle::new(1)],
        "the menu closes itself in the same dispatch"
    );
    assert_eq!(
        read(&state).selected(),
        None,
        "the entry's action ran: this menu's entries all clear the selection"
    );
}

#[test]
fn a_press_in_the_padding_closes_without_applying_anything() {
    let state = shared();
    state.lock().unwrap().select("body");
    let mut widget = menu(&state, vec!["one"]);
    let bounds = menu_bounds_for(1);
    // Above the first entry, inside the box.
    let at = Point::new(bounds.x0 + 10.0, bounds.y0 + MENU_PADDING / 2.0);
    let mut cx = TestEventContext::new(ID, bounds, press_in_menu(at));
    assert!(widget.event(&mut cx, ID));

    assert_eq!(
        read(&state).selected(),
        Some("body"),
        "a press on the menu's own padding is not a choice"
    );
    assert_eq!(cx.closed_handles().len(), 1, "but it does close the menu");
}

// ---------------------------------------------------------------------------
// Where the menu opens
// ---------------------------------------------------------------------------

#[test]
fn a_menu_opens_at_the_pointer_when_there_is_room() {
    let viewport = Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(800.0, 600.0));
    let at = Point::new(100.0, 100.0);
    let bounds = menu_bounds(at, viewport, 3);
    assert_eq!(bounds.x0, 100.0);
    assert_eq!(bounds.y0, 100.0);
    assert_eq!(bounds.width(), MENU_WIDTH);
    assert_eq!(bounds.height(), MENU_PADDING * 2.0 + MENU_ITEM_HEIGHT * 3.0);
}

#[test]
fn a_menu_near_the_right_edge_flips_to_the_left() {
    let viewport = Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(800.0, 600.0));
    let at = Point::new(780.0, 100.0);
    let bounds = menu_bounds(at, viewport, 1);
    assert!(
        bounds.x1 <= 780.0,
        "the menu must not run off the window: {bounds:?}"
    );
    assert_eq!(bounds.x1, 780.0, "and it flips to end at the pointer");
}

#[test]
fn a_menu_near_the_bottom_edge_flips_up() {
    let viewport = Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(800.0, 600.0));
    let at = Point::new(100.0, 590.0);
    let bounds = menu_bounds(at, viewport, 2);
    assert!(bounds.y1 <= 590.0, "{bounds:?}");
    assert_eq!(bounds.y1, 590.0);
}

#[test]
fn a_menu_that_cannot_fit_anywhere_still_stays_inside_the_window() {
    // A window shorter than the menu: flipping up and clamping both overshoot, so
    // the clamp to the viewport's origin is what keeps the menu on screen.
    let viewport = Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(800.0, 40.0));
    let bounds = menu_bounds(Point::new(100.0, 30.0), viewport, 3);
    assert_eq!(bounds.y0, 0.0, "clamped to the top of the window");
}

#[test]
fn a_menu_that_does_not_know_the_viewport_opens_at_the_pointer() {
    // `Rect::ZERO` is "the host cannot say where its content is visible". Guessing
    // an edge would be worse than letting it overhang.
    let bounds = menu_bounds(Point::new(700.0, 500.0), Rect::ZERO, 3);
    assert_eq!(bounds.x0, 700.0);
    assert_eq!(bounds.y0, 500.0);
}

// ---------------------------------------------------------------------------
// The slot a row is mounted through
// ---------------------------------------------------------------------------

#[test]
fn a_row_slot_builds_a_row() {
    let state = shared();
    let child = row_child(7, state.clone(), node("body", true, true), "host".into(), 1);
    assert_eq!(child.key, floom_core::ChildKey::keyed(7));
    let widget = (child.build)();
    assert!(
        widget.as_any().downcast_ref::<TreeRow>().is_some(),
        "the slot builds the row widget"
    );
}
