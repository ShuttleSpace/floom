//! The category tests hold each category to its contract: every component in
//! it has real samples, those samples actually render, and each component
//! states what it does.
//!
//! A page with no samples is a page that shows a name and nothing else, which
//! is not a gallery. Every component therefore needs at least two samples --
//! one showing its common configuration and one showing a variation -- and
//! each of them has to mount and paint.

use floom::floom_types::Size;
use floom_testkit::TestApp;

use floom_demo::registry;

const WINDOW: Size = Size::new(1280.0, 860.0);

/// The components of the Actions category, by the name a reader sees.
const ACTIONS: &[&str] = &[
    "Button",
    "ButtonGroup",
    "CheckboxBtn",
    "ChipGroup",
    "ConfirmEdit",
    "Counter",
    "ExpansionPanels",
    "FloatingActionButton",
    "IconButton",
    "ItemGroup",
    "ListGroup",
    "ListSubheader",
    "Radio",
    "RadioGroup",
    "Rating",
    "SpeedDial",
    "Stepper",
    "Switch",
];

/// The components of the Forms category, by the name a reader sees.
const FORMS: &[&str] = &[
    "Autocomplete",
    "Checkbox",
    "ColorPicker",
    "Combobox",
    "DatePicker",
    "FileInput",
    "Form",
    "NumberInput",
    "OTPInput",
    "RangeSlider",
    "Select",
    "Slider",
    "TextField",
    "Textarea",
    "TimePicker",
    "ValidationMessages",
];

/// The components that fire no event at all.
///
/// A button group arranges other buttons; a list subheader is a heading; a
/// validation-message list is told what to show rather than reporting that
/// something went wrong. None of them has anything to report, so their pages
/// say so by listing no events rather than by inventing one.
const SILENT: &[&str] = &["ButtonGroup", "ListSubheader", "ValidationMessages"];

/// Every component in a category shows at least two samples.
fn assert_two_samples_each(names: &[&str]) {
    for name in names {
        let page = registry::page_named(name).expect("page exists");
        assert!(
            page.samples.len() >= 2,
            "{name} has {} samples",
            page.samples.len()
        );
    }
}

/// Every component in a category says how to build it, and what it fires.
fn assert_states_what_it_does(names: &[&str]) {
    for name in names {
        let page = registry::page_named(name).expect("page exists");
        assert!(
            !page.api.methods.is_empty(),
            "{name} lists no methods; a reader cannot tell how to build it"
        );
        if SILENT.contains(name) {
            assert!(
                page.api.events.is_empty(),
                "{name} fires no event, so it should list none"
            );
        } else {
            assert!(
                !page.api.events.is_empty(),
                "{name} lists no events; a reader cannot tell what it fires"
            );
        }
    }
}

/// Every page in a category mounts and paints.
fn assert_mounts_and_paints(names: &[&str]) {
    for name in names {
        let page = registry::page_named(name).expect("page exists");
        let mut app = TestApp::unpinned(WINDOW, page.build_shell());
        assert!(app.frame().painted > 0, "{name} painted no widgets");
    }
}

#[test]
fn every_actions_component_has_at_least_two_samples() {
    assert_two_samples_each(ACTIONS);
}

#[test]
fn every_actions_component_states_what_it_does() {
    assert_states_what_it_does(ACTIONS);
}

#[test]
fn every_actions_page_mounts_and_paints() {
    assert_mounts_and_paints(ACTIONS);
}

#[test]
fn every_forms_component_has_at_least_two_samples() {
    assert_two_samples_each(FORMS);
}

#[test]
fn every_forms_component_states_what_it_does() {
    assert_states_what_it_does(FORMS);
}

#[test]
fn every_forms_page_mounts_and_paints() {
    assert_mounts_and_paints(FORMS);
}
