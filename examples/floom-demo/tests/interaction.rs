//! The gallery's interactive samples, proven to move.
//!
//! A gallery that shows a button has to show a button that does something.
//! These tests press the interactive samples and insist that the press reached
//! the component: the button counts its taps, and the checkbox records whether
//! it is ticked.

use floom::floom_types::Size;
use floom_testkit::{Selector, TestApp};

use floom_demo::{probe, registry};

const WINDOW: Size = Size::new(1280.0, 860.0);

/// Pressing the button sample counts the press.
#[test]
fn tapping_the_button_sample_counts_the_tap() {
    probe::reset();

    let page = registry::page_named("Button").expect("the Button page");
    let mut app = TestApp::unpinned(WINDOW, page.build_shell());
    app.frame();

    assert_eq!(probe::button_taps(), 0, "the counter starts at zero");

    let button = app.node(Selector::label("Tap me"));
    assert!(app.tap(button), "the tap did not reach the button");
    app.frame();

    assert_eq!(
        probe::button_taps(),
        1,
        "tapping the button did not reach its handler"
    );
}

/// Pressing the checkbox sample records the new ticked state.
#[test]
fn tapping_the_checkbox_sample_records_the_state() {
    probe::reset();

    let page = registry::page_named("Checkbox").expect("the Checkbox page");
    let mut app = TestApp::unpinned(WINDOW, page.build_shell());
    app.frame();

    assert!(!probe::checkbox_ticked(), "the checkbox starts unticked");

    let checkbox = app.node(Selector::label("Tick me"));
    assert!(app.tap(checkbox), "the tap did not reach the checkbox");
    app.frame();

    assert!(
        probe::checkbox_ticked(),
        "ticking the checkbox did not reach its handler"
    );
}
