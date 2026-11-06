use std::cell::Cell;
use std::rc::Rc;

use floom_core::{ElementId, Key, NamedKey, Point, Rect, Widget};
use floom_types::{KeyEvent, PointerEvent, PointerPhase};
use floom_ui_composites::Slider;
use floom_widgets::testkit::{dispatch, AllocatingCreateContext};

/// The box the control is laid out in for every test here: wide enough for the
/// track to be dragged across, tall enough for the thumb to sit on it.
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

/// --- Slider ----------------------------------------------------------------

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
