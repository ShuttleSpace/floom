//! `Slider` driven through real platform events.
//!
//! These moved with the widget from `floom-app/tests/interaction.rs`, for the
//! reason `select_interaction.rs` moved: a test of a widget's behaviour belongs
//! with the widget, and this file is what has to be deleted with it.
//!
//! What they pin is the wiring *around* the widget's own logic -- a press the
//! platform reports reaching the control under the cursor, a key reaching
//! whatever holds focus at that moment -- which is the chain `TestApp` gives
//! and the direct dispatch in `slider.rs` cannot.

use std::cell::Cell;
use std::rc::Rc;

use floom_platform::KeyCode;
use floom_testkit::TestApp;
use floom_types::{Point, Size};
use floom_ui_composites::Slider;

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

    // Letting go changes the pixels even though the value holds still: the
    // pressed fill differs from the resting one, so the release has damage.
    assert!(app.release(Point::new(50.0, 16.0)), "letting go repaints");
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
