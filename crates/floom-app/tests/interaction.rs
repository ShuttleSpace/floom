//! End-to-end interaction tests: real platform events in, real control state
//! out.
//!
//! `crates/floom-widgets/tests/controls.rs` drives the widgets directly, so
//! it proves each control's own logic. What it cannot prove is the wiring
//! *around* them: that a pointer button the platform reports reaches the
//! widget under the cursor, that a key reaches whatever holds focus at that
//! moment, and that the resulting state change actually reaches the screen
//! instead of stopping inside the widget.
//!
//! `floom-testkit::TestApp` is that whole chain: `FakePlatform` →
//! `AppRunner` → `WidgetHost` → control, driven with the same `PlatformEvent`s
//! a real windowing backend would deliver. Every interaction asserts on two
//! things: the control's state moved, and the frame that followed had damage to
//! paint.

use std::cell::Cell;
use std::rc::Rc;

use floom_platform::KeyCode;
use floom_testkit::TestApp;
use floom_types::{Point, Size};
use floom_widgets::{Checkbox, Slider, Switch, TextField};

#[test]
fn dragging_a_slider_moves_its_value_and_repaints() {
    let value = Rc::new(Cell::new(0.0_f64));
    let sink = value.clone();
    let mut app = TestApp::new(
        Size::new(200.0, 32.0),
        Slider::new(0.0, 0.0, 100.0).on_change(move |next| sink.set(next)),
    );
    let slider = app.root();
    assert_eq!(app.widget::<Slider>(slider).value(), 0.0);

    // Press three quarters along a 200px track: 75 on a 0..100 range.
    assert!(app.press(Point::new(150.0, 16.0)), "the press repainted");
    assert_eq!(app.widget::<Slider>(slider).value(), 75.0);
    assert_eq!(value.get(), 75.0, "the app's callback saw the drag");

    // Still held: moving keeps following the pointer, no second press needed.
    assert!(app.move_to(Point::new(50.0, 16.0)), "the drag repainted");
    assert_eq!(app.widget::<Slider>(slider).value(), 25.0);
    assert_eq!(value.get(), 25.0);

    assert!(app.release(Point::new(50.0, 16.0)));
    assert!(
        app.is_focused(slider),
        "pressing a slider focuses it, which is what lets the keyboard take over"
    );

    app.settle();
}

#[test]
fn the_keyboard_moves_a_focused_slider() {
    let mut app = TestApp::new(
        Size::new(200.0, 32.0),
        Slider::new(50.0, 0.0, 100.0).step(5.0),
    );
    let slider = app.root();

    // A key with nothing focused is nobody's: it must not move the slider.
    assert!(!app.key(KeyCode::ARROW_RIGHT));
    assert_eq!(app.widget::<Slider>(slider).value(), 50.0);

    // Press near the left end: the pointer itself sets 0 (quantized to the
    // 5.0 step) and hands over focus.
    app.press(Point::new(4.0, 16.0));
    app.release(Point::new(4.0, 16.0));
    assert_eq!(app.widget::<Slider>(slider).value(), 0.0);

    assert!(app.key(KeyCode::ARROW_RIGHT));
    assert_eq!(app.widget::<Slider>(slider).value(), 5.0);

    // A page is ten steps.
    assert!(app.key(KeyCode::PAGE_UP));
    assert_eq!(app.widget::<Slider>(slider).value(), 55.0);

    assert!(app.key(KeyCode::END));
    assert_eq!(app.widget::<Slider>(slider).value(), 100.0);
}

#[test]
fn typing_reaches_the_focused_text_field() {
    let mut app = TestApp::new(Size::new(200.0, 32.0), TextField::new().placeholder("name"));
    let field = app.root();
    assert_eq!(app.widget::<TextField>(field).text(), "");

    // Nothing holds focus yet, so the character has nowhere to go.
    assert!(!app.key(KeyCode::from_char('a')));
    assert_eq!(app.widget::<TextField>(field).text(), "");

    // Clicking focuses it…
    app.press(Point::new(100.0, 16.0));
    app.release(Point::new(100.0, 16.0));
    assert!(app.is_focused(field), "pressing a text field focuses it");

    // …and now committed text lands in the value, caret advancing with it.
    assert!(app.commit("hi"), "typing repainted the field");
    assert_eq!(app.widget::<TextField>(field).text(), "hi");
    assert_eq!(app.widget::<TextField>(field).caret(), 2);

    // Backspace removes the character before the caret.
    assert!(app.key(KeyCode::BACKSPACE));
    assert_eq!(app.widget::<TextField>(field).text(), "h");
    assert_eq!(app.widget::<TextField>(field).caret(), 1);

    // A plain character key arrives as text input too, not as a shortcut.
    assert!(app.key(KeyCode::from_char('i')));
    assert_eq!(app.widget::<TextField>(field).text(), "hi");

    app.settle();
}

#[test]
fn a_click_toggles_a_checkbox() {
    let toggles = Rc::new(Cell::new(0));
    let reported = Rc::new(Cell::new(None::<bool>));
    let count = toggles.clone();
    let state = reported.clone();
    let mut app = TestApp::new(
        Size::new(200.0, 32.0),
        Checkbox::new("I agree").on_toggle(move |on| {
            count.set(count.get() + 1);
            state.set(Some(on));
        }),
    );
    let checkbox = app.root();
    assert!(!app.widget::<Checkbox>(checkbox).is_checked());

    app.press(Point::new(100.0, 16.0));
    assert!(app.release(Point::new(100.0, 16.0)), "the click repainted");
    assert!(
        app.widget::<Checkbox>(checkbox).is_checked(),
        "the click checked it"
    );
    assert_eq!(toggles.get(), 1, "on_toggle fired exactly once");
    assert_eq!(
        reported.get(),
        Some(true),
        "the callback reports the state it moved to"
    );

    // And back again: the callback sees the way back too.
    app.press(Point::new(100.0, 16.0));
    app.release(Point::new(100.0, 16.0));
    assert!(!app.widget::<Checkbox>(checkbox).is_checked());
    assert_eq!(toggles.get(), 2);
    assert_eq!(reported.get(), Some(false));

    app.settle();
}

#[test]
fn a_disabled_control_ignores_the_pointer_end_to_end() {
    let toggles = Rc::new(Cell::new(0));
    let count = toggles.clone();
    let mut app = TestApp::new(
        Size::new(200.0, 32.0),
        Checkbox::new("I agree")
            .disabled(true)
            .on_toggle(move |_| count.set(count.get() + 1)),
    );
    let checkbox = app.root();

    // The whole chain agrees it is disabled: the event is delivered, refused,
    // and nothing changes — so there is nothing to repaint either.
    assert!(!app.press(Point::new(100.0, 16.0)));
    assert!(!app.release(Point::new(100.0, 16.0)));
    assert!(!app.widget::<Checkbox>(checkbox).is_checked());
    assert_eq!(toggles.get(), 0);
}

#[test]
fn a_click_flips_a_switch() {
    let mut app = TestApp::new(Size::new(200.0, 32.0), Switch::new("Wi-Fi"));
    let switch = app.root();
    assert!(!app.widget::<Switch>(switch).is_on());

    // By label rather than by handle: the selector reads the same string the
    // accessibility tree carries, so the test says what a user would.
    app.tap("Wi-Fi");
    assert!(
        app.widget::<Switch>(switch).is_on(),
        "the click flipped it on"
    );

    app.tap("Wi-Fi");
    assert!(!app.widget::<Switch>(switch).is_on());
}
