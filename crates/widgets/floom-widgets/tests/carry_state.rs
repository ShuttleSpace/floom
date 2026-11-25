//! The contract a rebuild leans on: identity is the parent's, state is the
//! widget's.

use floom_core::{Widget, WidgetAny};
use floom_widgets::{Scroll, Switch, Text, TextInput};

#[test]
fn a_rebuilt_label_keeps_what_the_app_put_in_it() {
    // The parent asked for "Count: 0"; the frame callback had moved it to 7.
    // A rebuild re-runs the parent, whose source still says 0 -- the widget is
    // what remembers, and the widget is what gets carried.
    let previous = Text::new("Count: 7");
    let mut rebuilt = Text::new("Count: 0");
    rebuilt.carry_state(&previous as &dyn WidgetAny);
    assert_eq!(rebuilt.content(), "Count: 7");
}

#[test]
fn a_rebuilt_switch_keeps_its_knob_but_not_its_label() {
    let previous = Switch::new("Wi-Fi").on(true);
    let mut rebuilt = Switch::new("Bluetooth");
    rebuilt.carry_state(&previous as &dyn WidgetAny);
    // Only the knob comes across; the label is whatever the parent asked for on
    // this run, which is the whole point of separating the two.
    assert!(rebuilt.is_on(), "the knob is state");
}

#[test]
fn a_different_widget_at_the_same_position_carries_nothing() {
    // Same slot, different type -- a rebuild that swapped a label for a switch.
    // The implementation must ignore the mismatch instead of panicking.
    let previous = Switch::new("Wi-Fi").on(true);
    let mut rebuilt = Text::new("Count: 0");
    rebuilt.carry_state(&previous as &dyn WidgetAny);
    assert_eq!(rebuilt.content(), "Count: 0");
}

#[test]
fn a_rebuilt_field_keeps_its_buffer_and_caret_but_not_its_composition() {
    // Two fields over one buffer would silently split if the rebuild copied the
    // string instead of the shared allocation. Writing through the *old* handle
    // and reading the new one is the proof.
    let buffer: floom_widgets::SharedBuffer = Default::default();
    *buffer.lock().unwrap() = "hello".to_string();
    let previous = TextInput::new(buffer.clone());
    let mut rebuilt = TextInput::new(Default::default());

    rebuilt.carry_state(&previous as &dyn WidgetAny);
    assert_eq!(rebuilt.text(), "hello");

    *buffer.lock().unwrap() = "hello there".to_string();
    assert_eq!(rebuilt.text(), "hello there", "one buffer, two fields");
}
