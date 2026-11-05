// SPDX-License-Identifier: MIT
//! The text-field case used to live here too, and so did the checkbox's. Both
//! moved to `floom-ui-composites/tests/text_field_host.rs` and
//! `floom-ui-composites/tests/checkbox_host.rs` with the widgets, because the root
//! workspace cannot reach a crate in the satellite `floom-ui` workspace. The
//! switch is what stayed, so the switch is what the file is about now.

use floom_testkit::TestApp;
use floom_types::Size;
use floom_widgets::Switch;

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
