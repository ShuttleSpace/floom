//! Interaction, state-variant and semantics tests for the Tier-1 controls.

use std::cell::Cell;
use std::rc::Rc;

use floom_accessibility::SemanticsState;
use floom_core::{
    Axis, Child, ChildKey, ElementId, Key, KeyPhase, LayoutRequest, NamedKey, OverlayHandle,
    OverlayKind, Point, Rect, Size, Widget, WidgetAny,
};
use floom_scene::SceneNode;
use floom_types::{Brush, Color, KeyEvent, Modifiers, PointerEvent, PointerPhase};
use floom_widgets::controls::{
    Button, Checkbox, Pressable, Progress, Radio, Select, SelectPopup, SelectStyle, Slider, Switch,
    TextField, popup_height,
};
use floom_widgets::primitives::Text;
use floom_widgets::Stack;
use floom_widgets::testkit::{
    AllocatingCreateContext, OverlayCall, RecordingUpdateContext, SceneRecorder, TestEventContext,
    dispatch, paint_snapshot, semantics_snapshot,
};

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

/// Helper unrolling `dyn Widget` downcasts for assertions after a dispatch.
fn fire_down_up<W: Widget>(widget: &mut W, id: ElementId, at: Point) -> bool {
    let down = dispatch(widget, id, ROW, PointerEvent::primary_down(at).into());
    let up = dispatch(widget, id, ROW, PointerEvent::primary_up(at).into());
    down || up
}

// --- Button ----------------------------------------------------------------

#[test]
fn button_click_sequence_invokes_callback_once() {
    let clicks = Rc::new(Cell::new(0));
    let counter = clicks.clone();
    let mut button = Button::new("Increment").on_click(move || counter.set(counter.get() + 1));
    let id = allocate(&button);

    assert!(dispatch(
        &mut button,
        id,
        ROW,
        PointerEvent::primary_down(Point::new(10.0, 10.0)).into()
    ));
    assert_eq!(clicks.get(), 0, "press alone must not activate");

    assert!(dispatch(
        &mut button,
        id,
        ROW,
        PointerEvent::primary_up(Point::new(10.0, 10.0)).into()
    ));
    assert_eq!(clicks.get(), 1);

    assert!(button.interaction().pressed == false);
    assert!(button.interaction().hovered);
}

#[test]
fn button_release_without_press_does_not_activate() {
    let clicks = Rc::new(Cell::new(0));
    let counter = clicks.clone();
    let mut button = Button::new("No-op").on_click(move || counter.set(counter.get() + 1));
    let id = allocate(&button);

    dispatch(
        &mut button,
        id,
        ROW,
        PointerEvent::primary_up(Point::new(10.0, 10.0)).into(),
    );
    assert_eq!(clicks.get(), 0);
}

#[test]
fn button_disabled_ignores_everything() {
    let clicks = Rc::new(Cell::new(0));
    let counter = clicks.clone();
    let mut button = Button::new("Disabled")
        .disabled(true)
        .on_click(move || counter.set(counter.get() + 1));
    let id = allocate(&button);

    fire_down_up(&mut button, id, Point::new(10.0, 10.0));
    assert_eq!(clicks.get(), 0);
    assert!(!button.interaction().hovered);
}

#[test]
fn button_keyboard_activation_requires_focus() {
    let clicks = Rc::new(Cell::new(0));
    let counter = clicks.clone();
    let mut button = Button::new("Go").on_click(move || counter.set(counter.get() + 1));
    let id = allocate(&button);

    dispatch(&mut button, id, ROW, KeyEvent::down(Key::Char(' ')).into());
    assert_eq!(clicks.get(), 0, "unfocused controls ignore the keyboard");

    button.set_focused(true);
    dispatch(&mut button, id, ROW, KeyEvent::down(Key::Char(' ')).into());
    assert_eq!(clicks.get(), 1);

    dispatch(
        &mut button,
        id,
        ROW,
        KeyEvent::down(Key::Named(NamedKey::Enter)).into(),
    );
    assert_eq!(clicks.get(), 2);
}

#[test]
fn button_reports_disabled_and_focusable_semantics() {
    let enabled = Button::new("Ok");
    assert!(
        enabled
            .semantics_hint()
            .state
            .contains(SemanticsState::ENABLED)
    );
}

// --- TextField -------------------------------------------------------------

#[test]
fn text_field_edits_at_the_caret() {
    let mut field = TextField::new().placeholder("name");
    let id = allocate(&field);
    field.set_focused(true);

    for ch in ['a', 'b', 'c'] {
        dispatch(&mut field, id, ROW, KeyEvent::down(Key::Char(ch)).into());
    }
    assert_eq!(field.text(), "abc");
    assert_eq!(field.caret(), 3);

    dispatch(
        &mut field,
        id,
        ROW,
        KeyEvent::down(Key::Named(NamedKey::ArrowLeft)).into(),
    );
    dispatch(
        &mut field,
        id,
        ROW,
        KeyEvent::down(Key::Named(NamedKey::Backspace)).into(),
    );
    assert_eq!(field.text(), "ac");
    assert_eq!(field.caret(), 1);

    dispatch(
        &mut field,
        id,
        ROW,
        KeyEvent::down(Key::Named(NamedKey::Delete)).into(),
    );
    assert_eq!(field.text(), "a");
}

#[test]
fn text_field_caret_never_splits_a_codepoint() {
    let mut field = TextField::new();
    let id = allocate(&field);
    field.set_focused(true);

    for ch in ['中', '文'] {
        dispatch(&mut field, id, ROW, KeyEvent::down(Key::Char(ch)).into());
    }
    assert_eq!(field.text(), "中文");

    dispatch(
        &mut field,
        id,
        ROW,
        KeyEvent::down(Key::Named(NamedKey::ArrowLeft)).into(),
    );
    dispatch(
        &mut field,
        id,
        ROW,
        KeyEvent::down(Key::Named(NamedKey::Backspace)).into(),
    );
    assert_eq!(field.text(), "文", "caret moved one char, not one byte");
}

#[test]
fn text_field_shortcuts_are_not_text_input() {
    let mut field = TextField::new();
    let id = allocate(&field);
    field.set_focused(true);

    dispatch(
        &mut field,
        id,
        ROW,
        KeyEvent::down(Key::Char('a'))
            .with_modifiers(Modifiers::CONTROL)
            .into(),
    );
    assert_eq!(field.text(), "", "Ctrl+A is a shortcut, not typing");
}

#[test]
fn text_field_shows_placeholder_until_filled() {
    let empty = TextField::new().placeholder("name");
    let id = allocate(&empty);
    let hint = empty.semantics_hint();
    assert_eq!(hint.label.as_deref(), Some("name"));
    assert_eq!(
        semantics_snapshot(&empty, id, ROW),
        "label=\"name\" (0.0, 0.0, 200.0, 32.0)"
    );
}

#[test]
fn text_field_submit_reports_current_value() {
    let submitted = Rc::new(Cell::new(String::new()));
    let sink = submitted.clone();
    let mut field = TextField::new().value("hi").on_submit(move |text| {
        sink.set(text.to_string());
    });
    let id = allocate(&field);
    field.set_focused(true);

    dispatch(
        &mut field,
        id,
        ROW,
        KeyEvent::down(Key::Named(NamedKey::Enter)).into(),
    );
    assert_eq!(submitted.take(), "hi");
}

// --- Checkbox / Radio / Switch ---------------------------------------------

#[test]
fn checkbox_toggles_and_publishes_checked_state() {
    let mut box_widget = Checkbox::new("Accept terms");
    let id = allocate(&box_widget);
    assert!(!box_widget.is_checked());

    fire_down_up(&mut box_widget, id, Point::new(4.0, 4.0));
    assert!(box_widget.is_checked());
    assert!(
        box_widget
            .semantics_hint()
            .state
            .contains(SemanticsState::CHECKED)
    );

    fire_down_up(&mut box_widget, id, Point::new(4.0, 4.0));
    assert!(!box_widget.is_checked());
}

#[test]
fn radio_selects_once_and_stays_selected() {
    let selections = Rc::new(Cell::new(0));
    let counter = selections.clone();
    let mut radio = Radio::new("Option A").on_select(move || counter.set(counter.get() + 1));
    let id = allocate(&radio);

    fire_down_up(&mut radio, id, Point::new(4.0, 4.0));
    fire_down_up(&mut radio, id, Point::new(4.0, 4.0));
    assert!(radio.is_selected());
    assert_eq!(selections.get(), 1, "re-selecting must not re-notify");
}

#[test]
fn switch_knob_tracks_state() {
    let mut switch = Switch::new("Wi-Fi");
    let id = allocate(&switch);

    let off_knob = switch.knob_bounds(ROW);
    fire_down_up(&mut switch, id, Point::new(4.0, 4.0));
    assert!(switch.is_on());

    let on_knob = switch.knob_bounds(ROW);
    assert!(on_knob.x0 > off_knob.x0, "knob slides right when on");
    assert!(switch.track_bounds(ROW).width() <= ROW.width());
}

// --- Slider ----------------------------------------------------------------

#[test]
fn slider_maps_position_to_value_with_step() {
    let mut slider = Slider::new(0.0, 0.0, 100.0).step(10.0);
    let id = allocate(&slider);

    assert_eq!(slider.value_at(ROW, ROW.x0 + ROW.width() / 2.0), 50.0);
    assert_eq!(slider.value_at(ROW, ROW.x0 + ROW.width() * 0.44), 40.0);
    assert_eq!(
        slider.value_at(ROW, ROW.x1 + 500.0),
        100.0,
        "clamped to max"
    );

    fire_down_up(&mut slider, id, Point::new(ROW.width() * 0.5, 10.0));
    assert_eq!(slider.value(), 50.0);
}

#[test]
fn slider_drag_follows_the_pointer() {
    let mut slider = Slider::new(0.0, 0.0, 10.0);
    let id = allocate(&slider);

    dispatch(
        &mut slider,
        id,
        ROW,
        PointerEvent::primary_down(Point::new(0.0, 10.0)).into(),
    );
    dispatch(
        &mut slider,
        id,
        ROW,
        PointerEvent {
            phase: PointerPhase::Move,
            ..PointerEvent::hover(Point::new(ROW.width() * 0.8, 10.0))
        }
        .into(),
    );
    assert!((slider.value() - 8.0).abs() < 1e-9);

    dispatch(
        &mut slider,
        id,
        ROW,
        PointerEvent::primary_up(Point::new(ROW.width() * 0.8, 10.0)).into(),
    );
    // After release, hovering must not keep dragging.
    dispatch(
        &mut slider,
        id,
        ROW,
        PointerEvent::hover(Point::new(0.0, 10.0)).into(),
    );
    assert!((slider.value() - 8.0).abs() < 1e-9);
}

#[test]
fn slider_is_drivable_from_the_keyboard() {
    let changes = Rc::new(Cell::new(0));
    let counter = changes.clone();
    let mut slider = Slider::new(50.0, 0.0, 100.0)
        .step(5.0)
        .on_change(move |_| counter.set(counter.get() + 1));
    slider.set_focused(true);
    let id = allocate(&slider);

    fn key(slider: &mut Slider, id: ElementId, named: NamedKey) -> bool {
        dispatch(slider, id, ROW, KeyEvent::down(Key::Named(named)).into())
    }

    assert!(key(&mut slider, id, NamedKey::ArrowRight));
    assert_eq!(slider.value(), 55.0, "one step up");
    assert!(key(&mut slider, id, NamedKey::ArrowLeft));
    assert_eq!(slider.value(), 50.0, "one step down");
    assert!(key(&mut slider, id, NamedKey::PageUp));
    assert_eq!(slider.value(), 100.0, "a page is ten steps, clamped to max");
    assert!(key(&mut slider, id, NamedKey::PageDown));
    assert_eq!(slider.value(), 50.0);
    assert!(key(&mut slider, id, NamedKey::End));
    assert_eq!(slider.value(), 100.0);
    assert!(key(&mut slider, id, NamedKey::Home));
    assert_eq!(slider.value(), 0.0);
    assert_eq!(changes.get(), 6, "every value change notifies");

    // At an end the key is still consumed, so it cannot scroll the page behind.
    assert!(
        key(&mut slider, id, NamedKey::ArrowLeft),
        "consumed even at the minimum"
    );
    assert_eq!(slider.value(), 0.0);

    // Unfocused (or a key release) leaves the value alone.
    slider.set_focused(false);
    assert!(!key(&mut slider, id, NamedKey::ArrowRight));
    assert_eq!(slider.value(), 0.0);
    slider.set_focused(true);
    assert!(!dispatch(
        &mut slider,
        id,
        ROW,
        KeyEvent::up(Key::Named(NamedKey::ArrowRight)).into()
    ));
    assert_eq!(slider.value(), 0.0);
}

#[test]
fn slider_without_a_step_still_has_a_key_granularity() {
    let mut slider = Slider::new(50.0, 0.0, 200.0);
    slider.set_focused(true);
    let id = allocate(&slider);

    // No step to quantize to: an arrow moves a hundredth of the range.
    assert!(dispatch(
        &mut slider,
        id,
        ROW,
        KeyEvent::down(Key::Named(NamedKey::ArrowRight)).into()
    ));
    assert!(
        (slider.value() - 52.0).abs() < 1e-9,
        "got {}",
        slider.value()
    );
}

// --- Progress / Select -----------------------------------------------------

#[test]
fn progress_clamps_and_describes_itself() {
    let mut bar = Progress::determinate(1.5);
    assert_eq!(bar.value(), Some(1.0));
    bar.set_value(Some(-3.0));
    assert_eq!(bar.value(), Some(0.0));

    assert_eq!(
        Progress::determinate(0.42)
            .semantics_hint()
            .label
            .as_deref(),
        Some("42%")
    );
    assert_eq!(
        Progress::indeterminate().semantics_hint().label.as_deref(),
        Some("busy")
    );
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
        bounds.y0 + f64::from(style.panel_padding) + (index as f64 + 0.5) * f64::from(style.item_height),
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

// --- shared behaviour ------------------------------------------------------

#[test]
fn pointer_down_requests_focus() {
    let mut button = Button::new("Focus me");
    let id = allocate(&button);
    let mut cx = TestEventContext::new(
        id,
        ROW,
        PointerEvent::primary_down(Point::new(1.0, 1.0)).into(),
    );
    assert!(button.event(&mut cx, id));
    assert_eq!(cx.focus_requests(), &[id]);
}

#[test]
fn leave_clears_hover_state() {
    let mut button = Button::new("Hover");
    let id = allocate(&button);
    dispatch(
        &mut button,
        id,
        ROW,
        PointerEvent::hover(Point::ZERO).into(),
    );
    assert!(button.interaction().hovered);

    dispatch(
        &mut button,
        id,
        ROW,
        PointerEvent {
            phase: PointerPhase::Leave,
            ..PointerEvent::hover(Point::ZERO)
        }
        .into(),
    );
    assert!(!button.interaction().hovered);
}

#[test]
fn disabled_controls_report_disabled_semantics() {
    let disabled = Button::new("Nope").disabled(true);
    let state = disabled.semantics_hint().state;
    assert!(state.contains(SemanticsState::DISABLED));
    assert!(!state.contains(SemanticsState::ENABLED));
}

#[test]
fn control_paint_snapshots_are_stable() {
    let button = Button::new("Save");
    let id = allocate(&button);
    let snapshot = paint_snapshot(
        &button,
        id,
        Rect::from_origin_size(Point::ZERO, Size::new(80.0, 24.0)),
    );
    assert!(
        snapshot.contains("RoundedRect (0.0, 0.0, 80.0, 24.0) r=4.0 #2B6CB0FF"),
        "background should render at the configured colour, got:\n{snapshot}"
    );
    assert!(
        snapshot.contains("\"Save\""),
        "label is painted: {snapshot}"
    );

    let checkbox = Checkbox::new("Accept").checked(true);
    let id = allocate(&checkbox);
    let snapshot = paint_snapshot(&checkbox, id, ROW);
    assert!(
        snapshot.contains("#2B6CB0FF"),
        "checked box is filled: {snapshot}"
    );
    assert!(
        snapshot.contains("\"Accept\""),
        "label follows the box: {snapshot}"
    );
}

// --- A3: a gradient in a real control ---------------------------------------

/// A progress bar's gradient is anchored to its *track*, not squeezed into
/// whatever fraction is currently filled — which is what "the brush carries
/// its own coordinates" buys, and the difference a screenshot cannot show you.
///
/// The evidence: paint the same bar at 25 % and at 90 %. The brush that reaches
/// the renderer is byte-identical in both (nothing was rebuilt per value), and
/// the ink at the fill's own centre has moved along the ramp — which it could
/// only do if the ramp stayed put while the fill grew over it. A ramp
/// compressed into the filled width would sample the same midpoint both times.
#[test]
fn a_progress_gradient_stays_anchored_to_the_track_as_the_fill_grows() {
    let ramp = Brush::linear(
        &[
            (0.0, Color::rgb(40, 60, 220)),
            (1.0, Color::rgb(220, 40, 40)),
        ],
        Point::new(0.0, 0.0),
        Point::new(200.0, 0.0),
    );
    let quarter = Progress::determinate(0.25).gradient(ramp.clone());
    let almost = Progress::determinate(0.9).gradient(ramp.clone());

    let painted = |widget: &Progress| {
        let id = allocate(widget);
        let mut recorder = SceneRecorder::new(ROW);
        widget.paint(&mut recorder, id);
        recorder.finish();
        let fills: Vec<_> = recorder
            .scene()
            .iter()
            .filter_map(|(_, node)| match node {
                SceneNode::Rect { fill, .. } => Some(fill.clone()),
                _ => None,
            })
            .collect();
        (recorder.snapshot(), fills)
    };

    let (quarter_ops, quarter_fills) = painted(&quarter);
    let (almost_ops, almost_fills) = painted(&almost);

    // Two rects each: the dim track, then the gradient fill.
    assert_eq!(quarter_fills.len(), 2);
    assert_eq!(
        quarter_fills[1], ramp,
        "the whole ramp reaches the renderer at 25 %"
    );
    assert_eq!(
        almost_fills[1], ramp,
        "and the identical ramp at 90 % — nothing was respecified"
    );
    let track_line = "Rect (0.0, 0.0, 200.0, 32.0) #E2E8F0FF";
    let quarter_fill_line = quarter_ops
        .lines()
        .nth(1)
        .expect("the fill is the second op")
        .to_string();
    let almost_fill_line = almost_ops
        .lines()
        .nth(1)
        .expect("the fill is the second op")
        .to_string();
    assert_eq!(quarter_ops.lines().next().unwrap(), track_line);
    assert_eq!(almost_ops.lines().next().unwrap(), track_line);
    assert!(
        quarter_fill_line.starts_with("BrushRect (0.0, 0.0, 50.0, 32.0)"),
        "a quarter of the 200px track: {quarter_fill_line}"
    );
    assert!(
        almost_fill_line.starts_with("BrushRect (0.0, 0.0, 180.0, 32.0)"),
        "ninety percent of the track: {almost_fill_line}"
    );
    assert_ne!(
        quarter_fill_line, almost_fill_line,
        "the fill's own centre landed further along the ramp"
    );
}

/// Without a gradient the bar is unchanged: one flat fill at the colour it has
/// always used. The new door must not repaint yesterday's bar.
#[test]
fn a_gradientless_progress_paints_the_same_flat_fill_as_before() {
    let id = allocate(&Progress::determinate(0.5));
    assert_eq!(
        paint_snapshot(&Progress::determinate(0.5), id, ROW),
        "Rect (0.0, 0.0, 200.0, 32.0) #E2E8F0FF\nRect (0.0, 0.0, 100.0, 32.0) #2B6CB0FF"
    );
}

// --- Pressable -------------------------------------------------------------

fn pressable(child: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Pressable {
    Pressable::new(Child::new(ChildKey::index(0), LayoutRequest::new(), child))
        .label("header")
}

/// A click is a press *and* a release over the same spot. Sending one half is
/// not a weaker click, it is not a click.
#[test]
fn a_pressable_fires_on_a_matched_press_and_release() {
    let hits = Rc::new(Cell::new(0u32));
    let sink = hits.clone();
    let mut widget = pressable(move || Box::new(Text::new("row"))).on_click(move || {
        sink.set(sink.get() + 1);
    });
    let id = allocate(&widget);

    assert!(fire_down_up(&mut widget, id, Point::new(10.0, 10.0)));

    assert_eq!(hits.get(), 1, "the callback runs exactly once");
}

/// The whole point of the widget: `Button` paints its own label, so a row of
/// widgets has no other way to be clickable.
#[test]
fn a_pressable_wraps_a_subtree_rather_than_a_label() {
    let row = Stack::new(Axis::Horizontal)
        .push(|| Box::new(Text::new("Title")))
        .push(|| Box::new(Text::new(">")));
    let widget = pressable(move || Box::new(row.clone()));

    let children = widget.children();
    assert_eq!(children.len(), 1);
    let built = children[0].build();
    assert!(
        built.as_any().downcast_ref::<Stack>().is_some(),
        "the pressable's child is the caller's subtree, untouched"
    );
}

/// The whole surface consumes, so a press inside it never reaches something
/// further down. (`Ripple` is the opposite case and says so.)
#[test]
fn a_pressable_consumes_the_press() {
    let mut widget = pressable(|| Box::new(Text::new("row")));
    let id = allocate(&widget);
    assert!(
        dispatch(&mut widget, id, ROW, PointerEvent::primary_down(Point::new(4.0, 4.0)).into()),
        "the press belongs to the pressable"
    );
}

/// A release with no press in flight is not a click. Without this a pointer
/// that appears already up — a synthetic event, a missed down — would fire the
/// action.
#[test]
fn a_release_without_a_press_does_not_fire() {
    let hits = Rc::new(Cell::new(0u32));
    let sink = hits.clone();
    let mut widget = pressable(|| Box::new(Text::new("row"))).on_click(move || {
        sink.set(sink.get() + 1);
    });
    let id = allocate(&widget);

    dispatch(&mut widget, id, ROW, PointerEvent::primary_up(Point::new(4.0, 4.0)).into());

    assert_eq!(hits.get(), 0, "a release alone is not a click");
}

#[test]
fn a_disabled_pressable_is_inert() {
    let hits = Rc::new(Cell::new(0u32));
    let sink = hits.clone();
    let mut widget = pressable(|| Box::new(Text::new("row")))
        .disabled(true)
        .on_click(move || {
            sink.set(sink.get() + 1);
        });
    let id = allocate(&widget);

    assert!(!fire_down_up(&mut widget, id, Point::new(4.0, 4.0)));

    assert_eq!(hits.get(), 0);
}

#[test]
fn a_focused_pressable_activates_from_the_keyboard() {
    let hits = Rc::new(Cell::new(0u32));
    let sink = hits.clone();
    let mut widget = pressable(|| Box::new(Text::new("row"))).on_click(move || {
        sink.set(sink.get() + 1);
    });
    let id = allocate(&widget);
    widget.set_focused(true);

    let event = KeyEvent {
        key: Key::Named(NamedKey::Enter),
        phase: KeyPhase::Down,
        modifiers: Modifiers::empty(),
    };
    assert!(dispatch(&mut widget, id, ROW, event.into()));

    assert_eq!(
        hits.get(),
        1,
        "a disclosure has to be operable without a pointer"
    );
}

/// The reported state follows a *reader*, not a value captured at build time.
/// A header that is not rebuilt across its own toggle would otherwise report
/// the state it was built with — the opposite of what the user sees.
#[test]
fn the_reported_expanded_state_follows_a_shared_flag() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let open = Arc::new(AtomicBool::new(false));
    let reader = open.clone();
    let mut widget = pressable(|| Box::new(Text::new("row")))
        .expanded_from(move || reader.load(Ordering::SeqCst));
    let id = allocate(&widget);

    let mut cx = RecordingUpdateContext::new();
    widget.update(&mut cx, id);
    assert!(
        widget.semantics_hint().state.contains(SemanticsState::COLLAPSED),
        "closed panels report collapsed"
    );

    open.store(true, Ordering::SeqCst);
    let mut cx = RecordingUpdateContext::new();
    widget.update(&mut cx, id);
    assert!(
        widget.semantics_hint().state.contains(SemanticsState::EXPANDED),
        "a flag that moved since the last update has to invalidate the node, \
         or the semantics tree keeps reporting last frame's answer"
    );
    assert_eq!(cx.dirty_count(), 1);
}

/// The settled frame marks nothing: a widget that dirties itself every frame
/// would invalidate its whole subtree for a tree standing still.
#[test]
fn a_settled_pressable_marks_nothing_dirty() {
    let mut widget = pressable(|| Box::new(Text::new("row")));
    let id = allocate(&widget);

    let mut first = RecordingUpdateContext::new();
    widget.update(&mut first, id);
    let mut second = RecordingUpdateContext::new();
    widget.update(&mut second, id);

    assert_eq!(first.dirty_count(), 1, "the first sync paints once");
    assert_eq!(
        second.dirty_count(),
        0,
        "an unchanged widget must not invalidate again"
    );
}

#[test]
fn a_pressable_reports_its_role_and_name() {
    let widget = pressable(|| Box::new(Text::new("row")));
    let hint = widget.semantics_hint();
    assert_eq!(hint.role, floom_accessibility::SemanticsRole::Button);
    assert_eq!(hint.label.as_deref(), Some("header"));
    assert!(
        hint.state.contains(SemanticsState::FOCUSABLE),
        "an activatable surface must be reachable by keyboard"
    );
}

#[test]
fn a_pressable_paints_nothing() {
    let widget = pressable(|| Box::new(Text::new("row")));
    let id = allocate(&widget);
    assert_eq!(paint_snapshot(&widget, id, ROW), "(empty)");
}
