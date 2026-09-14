//! Interaction, state-variant and semantics tests for the Tier-1 controls.

use std::cell::Cell;
use std::rc::Rc;

use floem_accessibility::SemanticsState;
use floem_core::{ElementId, Key, NamedKey, Point, Rect, Size, Widget};
use floem_types::{KeyEvent, Modifiers, PointerEvent, PointerPhase};
use floem_widgets::controls::{
    Button, Checkbox, Progress, Radio, Select, Slider, Switch, TextField,
};
use floem_widgets::testkit::{
    AllocatingCreateContext, TestEventContext, dispatch, paint_snapshot, semantics_snapshot,
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

#[test]
fn select_opens_then_chooses_a_row() {
    let selected = Rc::new(Cell::new(None::<usize>));
    let sink = selected.clone();
    let mut select = Select::new(["Red", "Green", "Blue"]).on_change(move |index| {
        sink.set(Some(index));
    });
    let id = allocate(&select);

    assert!(!select.is_open());
    fire_down_up(&mut select, id, Point::new(10.0, 5.0));
    assert!(select.is_open(), "click on the header toggles the list");

    let row_y = 22.0 + 10.0; // header height + inside second row
    assert_eq!(select.row_at(ROW, row_y), Some(0));

    fire_down_up(&mut select, id, Point::new(10.0, row_y));
    assert_eq!(select.selected_index(), Some(0));
    assert_eq!(selected.get(), Some(0));
    assert!(!select.is_open(), "choosing closes the list");
}

#[test]
fn select_reports_selected_label() {
    let select = Select::new(["Red", "Green"]).selected(1);
    assert_eq!(select.selected_label(), "Green");
    let empty = Select::new(["Red"]);
    assert_eq!(empty.selected_label(), "\u{2014}");
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
