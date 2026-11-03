
use std::cell::Cell;
use std::rc::Rc;

use floom_testkit::TestApp;
use floom_types::{Point, Size};
use floom_ui_composites::Checkbox;

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
    // and nothing changes -- so there is nothing to repaint either.
    assert!(!app.press(Point::new(100.0, 16.0)));
    assert!(!app.release(Point::new(100.0, 16.0)));
    assert!(!app.widget::<Checkbox>(checkbox).is_checked());
    assert_eq!(toggles.get(), 0);
}

/// The label-selector case, moved verbatim from `floom-testkit/tests/testkit.rs`:
/// `tap` finds its target by the accessibility label rather than by handle, so
/// the test says what a user would -- and `assert_text` says the label on screen
/// is the one the control carries, which is the half a widget's own test cannot
/// reach.
#[test]
fn a_tap_on_a_checkbox_toggles_it() {
    let mut app = TestApp::new(Size::new(200.0, 32.0), Checkbox::new("Agree"));
    assert!(!app.widget::<Checkbox>("Agree").is_checked());

    app.tap("Agree");
    assert!(app.widget::<Checkbox>("Agree").is_checked());
    app.assert_text("Agree", "Agree");
}
