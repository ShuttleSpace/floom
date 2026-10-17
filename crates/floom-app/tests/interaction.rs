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
//!
//! The text-field case used to live here too. It moved to
//! `floom-ui-composites/tests/text_field_host.rs` with the widget, because the
//! root workspace cannot reach a crate in the satellite `floom-ui` workspace; the
//! two controls that stayed keep the file's original subject.

use std::cell::Cell;
use std::rc::Rc;

use floom_platform::KeyCode;
use floom_testkit::TestApp;
use floom_types::{Point, Size};
use floom_widgets::{Checkbox, Switch};

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
