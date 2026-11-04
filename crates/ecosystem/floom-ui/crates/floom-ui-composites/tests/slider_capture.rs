use std::cell::Cell;
use std::rc::Rc;

use floom_testkit::TestApp;
use floom_types::{Point, Size};
use floom_ui_composites::Slider;

#[test]
fn release_outside_does_not_make_hover_change_value() {
    let value = Rc::new(Cell::new(0.0_f64));
    let sink = value.clone();
    let mut app = TestApp::new(
        Size::new(200.0, 32.0),
        Slider::new(0.0, 0.0, 100.0).on_change(move |next| sink.set(next)),
    );
    let slider = app.root();

    // Press three quarters along, drag, then release OUTSIDE the window (y past
    // the slider's 32px height) so the slider never sees an `Up` over itself.
    app.press(Point::new(150.0, 16.0));
    app.move_to(Point::new(150.0, 100.0));
    app.release(Point::new(150.0, 100.0));
    assert_eq!(app.widget::<Slider>(slider).value(), 75.0);

    // Now just hover back over the slider (no button held). With a stuck
    // `pressed` flag this would move the value; it must not.
    app.move_to(Point::new(40.0, 16.0));
    assert_eq!(
        app.widget::<Slider>(slider).value(),
        75.0,
        "hover after an outside release must not move the value"
    );
}
