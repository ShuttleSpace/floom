//! The category tests hold each category to its contract: every component in
//! it has real samples, and those samples actually render.
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

/// The components of the Actions category that fire no event at all.
///
/// A button group arranges other buttons; a list subheader is a heading.
/// Neither has anything to report, so their pages say so by listing no
/// events rather than by inventing one.
const SILENT: &[&str] = &["ButtonGroup", "ListSubheader"];

#[test]
fn every_actions_component_has_at_least_two_samples() {
    for name in ACTIONS {
        let page = registry::page_named(name).expect("page exists");
        assert!(
            page.samples.len() >= 2,
            "{name} has {} samples",
            page.samples.len()
        );
    }
}

#[test]
fn every_actions_component_states_what_it_does() {
    for name in ACTIONS {
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

#[test]
fn every_actions_page_mounts_and_paints() {
    for name in ACTIONS {
        let page = registry::page_named(name).expect("page exists");
        let mut app = TestApp::unpinned(WINDOW, page.build_shell());
        assert!(app.frame().painted > 0, "{name} painted no widgets");
    }
}
