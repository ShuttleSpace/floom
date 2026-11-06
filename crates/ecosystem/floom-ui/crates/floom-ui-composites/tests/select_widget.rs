use std::cell::Cell;
use std::rc::Rc;

use floom_accessibility::SemanticsState;
use floom_core::{
    ElementId, Key, KeyPhase, LayoutRequest, NamedKey, OverlayHandle, OverlayKind, Point, Rect,
    Widget, WidgetAny,
};
use floom_types::{KeyEvent, Modifiers, PointerEvent, PointerPhase};
use floom_ui_composites::{popup_height, Select, SelectPopup, SelectStyle};
use floom_widgets::primitives::Text;
use floom_widgets::testkit::{
    dispatch, paint_snapshot, paint_snapshot_any, AllocatingCreateContext, OverlayCall,
    RecordingUpdateContext, SceneRecorder, TestEventContext,
};
use floom_widgets::Stack;

/// The box the control is laid out in for every test here: wide enough that the
/// widest option never wraps, tall enough that the header and the row height
/// arithmetic below both read as single-line.
const ROW: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 32.0,
};

fn allocate(widget: &dyn Widget) -> ElementId {
    let mut create = AllocatingCreateContext::new();
    widget.create(&mut create)
}

/// Opens `select`'s list the way a host would: dispatch the header click, then
/// report the overlay back through `overlay_opened` (which is how a widget
/// learns the handle it cannot be told about synchronously).
///
/// Returns the content the request built, the rect it was anchored at, and the
/// handle the host issued.
fn open_popup(select: &mut Select, id: ElementId) -> (Box<dyn WidgetAny>, Rect, OverlayHandle) {
    let mut cx =
        TestEventContext::pointer(id, ROW, PointerEvent::primary_up(Point::new(10.0, 5.0)));
    assert!(select.event(&mut cx, id), "the header click opens the list");
    let (handle, kind, bounds) = match cx.last_open() {
        Some(OverlayCall::Open {
            handle,
            kind,
            bounds,
            ..
        }) => (*handle, *kind, *bounds),
        _ => panic!("the select asked for no overlay"),
    };
    assert_eq!(
        kind,
        OverlayKind::Popup,
        "the list is a popup, not a dialog"
    );
    let content = cx.take_content().expect("the request carried content");
    let mut update = RecordingUpdateContext::new();
    select.overlay_opened(&mut update, id, handle);
    assert!(select.is_open(), "the host confirmed the popup is up");
    (content, bounds, handle)
}

/// The style an unstyled `Select` paints with, so the geometry assertions below
/// read the same numbers the widget does instead of a copy of them that would
/// silently agree with a change.
fn default_style() -> SelectStyle {
    SelectStyle::default()
}

/// Centre of row `index` inside `bounds`.
///
/// Rows start *after* the panel's padding — the popup's rect includes it — so
/// the row's centre is the padding plus half a row in.
fn row_center(bounds: Rect, index: usize) -> Point {
    let style = default_style();
    Point::new(
        bounds.x0 + 10.0,
        bounds.y0
            + f64::from(style.panel_padding)
            + (index as f64 + 0.5) * f64::from(style.item_height),
    )
}

fn popup_of(content: &mut Box<dyn WidgetAny>) -> &mut SelectPopup {
    content
        .as_any_mut()
        .downcast_mut::<SelectPopup>()
        .expect("the select's content is a popup list")
}

/// Dispatches a pointer event into the popup and returns what it asked for.
fn click_popup(popup: &mut SelectPopup, bounds: Rect, pointer: PointerEvent) -> TestEventContext {
    let popup_id = ElementId::new(99, 1);
    let mut cx = TestEventContext::pointer(popup_id, bounds, pointer);
    popup.event(&mut cx, popup_id);
    cx
}

/// Dispatches a key into the popup and returns what it asked for.
fn key_popup(popup: &mut SelectPopup, bounds: Rect, key: Key) -> TestEventContext {
    let popup_id = ElementId::new(99, 1);
    let mut cx = TestEventContext::new(popup_id, bounds, KeyEvent::down(key).into());
    popup.event(&mut cx, popup_id);
    cx
}

#[test]
fn select_opens_a_list_as_an_overlay_below_itself() {
    let mut select = Select::new(["Red", "Green", "Blue"]);
    let id = allocate(&select);
    assert!(!select.is_open());

    let (mut content, bounds, handle) = open_popup(&mut select, id);

    // Anchored to the header's bottom edge, as wide as the control, as tall as
    // its rows: the Select guesses the geometry, the host lays the content out
    // against it.
    assert_eq!(bounds.y0, ROW.y1, "the list hangs below the header");
    assert_eq!(bounds.x0, ROW.x0);
    assert_eq!(bounds.width(), ROW.width());
    assert_eq!(bounds.height(), popup_height(&default_style(), 3));
    assert_eq!(select.popup_handle(), Some(handle));
    assert_eq!(
        popup_of(&mut content).options(),
        ["Red", "Green", "Blue"],
        "the list carries the options; the Select no longer paints them itself"
    );

    // The header keeps painting only its own box — the list is a separate node
    // now, which is the whole point of the change. Three ops: the border's
    // rounded rect, the background inset inside it, and the label.
    let painted = paint_snapshot(&select, id, ROW);
    assert_eq!(
        painted.lines().count(),
        3,
        "the closed control paints its border, its background and its label:\n{painted}"
    );
}

#[test]
fn choosing_a_row_moves_the_selection() {
    let selected = Rc::new(Cell::new(None::<usize>));
    let sink = selected.clone();
    let mut select = Select::new(["Red", "Green", "Blue"]).on_change(move |index| {
        sink.set(Some(index));
    });
    let id = allocate(&select);

    let (mut content, bounds, handle) = open_popup(&mut select, id);
    let popup = popup_of(&mut content);
    assert_eq!(popup.chosen(), None);

    // Click the third row.
    let cx = click_popup(
        popup,
        bounds,
        PointerEvent::primary_up(row_center(bounds, 2)),
    );
    assert_eq!(
        cx.closed_handles(),
        vec![handle],
        "picking a row takes the popup down"
    );
    assert_eq!(popup.chosen(), Some(2));

    // The host closes the overlay and hands the content back to the opener.
    let mut update = RecordingUpdateContext::new();
    select.overlay_closed(&mut update, id, handle, content);

    assert!(!select.is_open());
    assert_eq!(select.popup_handle(), None);
    assert_eq!(select.selected_index(), Some(2));
    assert_eq!(select.selected_label(), "Blue");
    assert_eq!(
        selected.get(),
        Some(2),
        "on_change fired once, with the row"
    );
    assert_eq!(update.dirty_count(), 1, "the header repaints");
}

#[test]
fn dismissing_the_list_leaves_the_selection_alone() {
    let selected = Rc::new(Cell::new(None::<usize>));
    let sink = selected.clone();
    let mut select = Select::new(["Red", "Green"]).on_change(move |index| {
        sink.set(Some(index));
    });
    let id = allocate(&select);

    let (mut content, bounds, handle) = open_popup(&mut select, id);
    let popup = popup_of(&mut content);
    let cx = key_popup(popup, bounds, Key::Named(NamedKey::Escape));

    assert_eq!(popup.chosen(), None, "escape chooses nothing");
    assert_eq!(cx.closed_handles(), vec![handle]);

    let mut update = RecordingUpdateContext::new();
    select.overlay_closed(&mut update, id, handle, content);

    assert!(!select.is_open(), "the list is gone");
    assert_eq!(select.selected_index(), None, "and nothing was chosen");
    assert_eq!(selected.get(), None, "on_change never fired");
}

#[test]
fn popup_walks_its_rows_with_the_keyboard() {
    let mut select = Select::new(["Red", "Green", "Blue"]).selected(0);
    let id = allocate(&select);
    let (mut content, bounds, _handle) = open_popup(&mut select, id);
    let popup = popup_of(&mut content);

    // The highlight starts on the current selection, so the keyboard carries
    // on from where the control left off.
    assert_eq!(popup.highlight(), 0);

    key_popup(popup, bounds, Key::Named(NamedKey::ArrowDown));
    assert_eq!(popup.highlight(), 1);
    key_popup(popup, bounds, Key::Named(NamedKey::ArrowDown));
    assert_eq!(popup.highlight(), 2);
    key_popup(popup, bounds, Key::Named(NamedKey::ArrowDown));
    assert_eq!(popup.highlight(), 2, "the last row clamps");

    key_popup(popup, bounds, Key::Named(NamedKey::ArrowUp));
    assert_eq!(popup.highlight(), 1);
    key_popup(popup, bounds, Key::Named(NamedKey::Home));
    assert_eq!(popup.highlight(), 0);
    key_popup(popup, bounds, Key::Named(NamedKey::End));
    assert_eq!(popup.highlight(), 2);

    // Enter commits the highlighted row.
    let cx = key_popup(popup, bounds, Key::Named(NamedKey::Enter));
    assert_eq!(popup.chosen(), Some(2));
    assert_eq!(cx.closed_handles().len(), 1, "committing closes the list");
}

#[test]
fn hovering_the_list_moves_the_highlight() {
    let mut select = Select::new(["Red", "Green", "Blue"]);
    let id = allocate(&select);
    let (mut content, bounds, _handle) = open_popup(&mut select, id);
    let popup = popup_of(&mut content);

    click_popup(popup, bounds, PointerEvent::hover(row_center(bounds, 1)));
    assert_eq!(popup.highlight(), 1, "the highlight follows the pointer");
    assert_eq!(popup.chosen(), None, "hovering alone chooses nothing");
}

#[test]
fn select_reports_selected_label() {
    let select = Select::new(["Red", "Green"]).selected(1);
    assert_eq!(select.selected_label(), "Green");
    let empty = Select::new(["Red"]);
    assert_eq!(empty.selected_label(), "\u{2014}");
}

#[test]
fn select_is_drivable_from_the_keyboard() {
    let selected = Rc::new(Cell::new(None::<usize>));
    let sink = selected.clone();
    let mut select = Select::new(["Red", "Green", "Blue"]).on_change(move |index| {
        sink.set(Some(index));
    });
    let id = allocate(&select);
    select.set_focused(true);

    fn key(select: &mut Select, id: ElementId, named: NamedKey) -> bool {
        dispatch(select, id, ROW, KeyEvent::down(Key::Named(named)).into())
    }

    // Enter opens the list by asking the host for an overlay; the control only
    // counts as open once the host says it took the request.
    assert!(!select.is_open());
    let mut cx = TestEventContext::new(id, ROW, KeyEvent::down(Key::Named(NamedKey::Enter)).into());
    assert!(select.event(&mut cx, id));
    let handle = cx
        .last_open()
        .map(OverlayCall::handle)
        .expect("enter asks the host for a list");
    let mut update = RecordingUpdateContext::new();
    select.overlay_opened(&mut update, id, handle);
    assert!(select.is_open(), "the list is up once the host confirms it");

    // While the list is up the keyboard belongs to the popup, so walking the
    // options here must not move the selection behind its back.
    assert!(!key(&mut select, id, NamedKey::ArrowDown));
    assert_eq!(select.selected_index(), None);
    assert_eq!(
        selected.get(),
        None,
        "the popup owns navigation while it is up"
    );

    // Escape (or a second Enter) takes it back down without choosing.
    let mut cx =
        TestEventContext::new(id, ROW, KeyEvent::down(Key::Named(NamedKey::Escape)).into());
    assert!(select.event(&mut cx, id));
    assert_eq!(
        cx.closed_handles(),
        vec![handle],
        "escape dismisses the list"
    );
    assert!(!select.is_open());

    // Closed, the arrow keys move the selection directly.
    assert!(key(&mut select, id, NamedKey::ArrowDown));
    assert_eq!(select.selected_index(), Some(0));
    assert_eq!(selected.get(), Some(0));
    assert!(key(&mut select, id, NamedKey::ArrowDown));
    assert_eq!(select.selected_index(), Some(1));
    assert!(key(&mut select, id, NamedKey::ArrowUp));
    assert_eq!(select.selected_index(), Some(0));
    assert!(key(&mut select, id, NamedKey::ArrowUp));
    assert_eq!(select.selected_index(), Some(0), "the first row clamps");
    assert!(key(&mut select, id, NamedKey::End));
    assert_eq!(select.selected_index(), Some(2));
    assert!(key(&mut select, id, NamedKey::Home));
    assert_eq!(select.selected_index(), Some(0));

    // Escape with nothing open is left to the window, and a key release, an
    // unfocused control and a key we do not handle all pass through.
    assert!(!key(&mut select, id, NamedKey::Escape));
    assert!(!dispatch(
        &mut select,
        id,
        ROW,
        KeyEvent::up(Key::Named(NamedKey::ArrowDown)).into()
    ));
    assert!(!key(&mut select, id, NamedKey::Tab));
    select.set_focused(false);
    assert!(!key(&mut select, id, NamedKey::ArrowDown));
    assert_eq!(select.selected_index(), Some(0));
}

#[test]
fn select_without_options_never_opens_a_list() {
    let mut select = Select::new(Vec::<String>::new());
    let id = allocate(&select);
    let mut cx = TestEventContext::pointer(id, ROW, PointerEvent::primary_up(Point::new(5.0, 5.0)));
    assert!(!select.event(&mut cx, id), "an empty list is not handled");
    assert!(cx.overlay_calls().is_empty(), "and nothing is requested");
    assert!(!select.is_open());
}

#[test]
fn select_repicking_the_open_row_dismisses_it() {
    let mut select = Select::new(["Red", "Green"]).selected(0);
    let id = allocate(&select);

    let (mut content, bounds, handle) = open_popup(&mut select, id);
    assert!(select.is_open());

    // Picking the row that is already chosen reports no new value, but the
    // popup still has to go away — and closing is a repaint of its own, or the
    // list would stay on screen behind a closed header.
    let popup = popup_of(&mut content);
    let cx = click_popup(
        popup,
        bounds,
        PointerEvent::primary_up(row_center(bounds, 0)),
    );
    assert_eq!(popup.chosen(), Some(0));
    assert_eq!(cx.closed_handles(), vec![handle]);

    let mut update = RecordingUpdateContext::new();
    select.overlay_closed(&mut update, id, handle, content);

    assert_eq!(select.selected_index(), Some(0));
    assert!(
        !select.is_open(),
        "re-picking the current option still closes the list"
    );
    assert_eq!(
        update.dirty_count(),
        1,
        "closing alone has to invalidate the widget"
    );
}
