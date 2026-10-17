//! Tests for the harness itself.
//!
//! A test harness is only worth what its failures are worth: if a selector
//! silently resolved to nothing, or an actor swallowed the event, every suite
//! built on it would pass for the wrong reason. So these check the parts the
//! suites rely on — that a label finds the node that actually says it, that a
//! tap delivers press *and* release, that damage is reported, and that a
//! tree which never settles fails instead of hanging.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use floom_core::{Axis, Child, ChildKey, LayoutRequest, OverlayKind, SemanticsRole};
use floom_testkit::{DEFAULT_SIZE, Selector, TestApp};
use floom_types::{Point, Rect, Size};
use floom_widgets::{Button, Checkbox, Stack, Switch, Text};

/// A button that counts its clicks.
///
/// The counter is atomic rather than an `Rc<Cell>` because a `Child` builder
/// has to be `Send + Sync` — it may be called from anywhere the reconciler
/// runs.
fn counting_button(sink: Arc<AtomicUsize>) -> impl floom_core::WidgetAny + 'static {
    Stack::new(Axis::Vertical).child(Child::new(
        ChildKey::index(0),
        LayoutRequest::new(),
        move || {
            let sink = sink.clone();
            Box::new(Button::new("Save").on_click(move || {
                sink.fetch_add(1, Ordering::SeqCst);
            }))
        },
    ))
}

/// A column holding a label and a button — the smallest tree with two things
/// worth telling apart.
fn labelled_button(label: &'static str) -> impl floom_core::WidgetAny + 'static {
    Stack::new(Axis::Vertical)
        .child(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
            Box::new(Text::new("header"))
        }))
        .child(Child::new(
            ChildKey::index(1),
            LayoutRequest::new(),
            move || Box::new(Button::new(label)),
        ))
}

#[test]
fn a_label_finds_the_node_that_announces_it() {
    let app = TestApp::new(Size::new(200.0, 80.0), labelled_button("Save"));

    let header = app.node("header");
    let save = app.node("Save");
    assert_ne!(header, save, "two labels, two nodes");

    // And the node really is the widget that claims the label.
    assert_eq!(app.widget::<Text>(header).content(), "header");
    assert_eq!(app.widget::<Button>(save).label(), "Save");

    assert_eq!(app.label("Save").as_deref(), Some("Save"));
    assert_eq!(app.find("Nothing says this"), None);
}

#[test]
#[should_panic(expected = "no node for")]
fn a_missing_label_fails_with_what_is_on_screen() {
    let app = TestApp::new(Size::new(200.0, 80.0), labelled_button("Save"));
    app.node("Discard");
}

#[test]
fn a_role_finds_the_first_node_publishing_it() {
    let app = TestApp::new(Size::new(200.0, 80.0), labelled_button("Save"));

    let button = app.find(Selector::role(SemanticsRole::Button)).unwrap();
    assert_eq!(app.widget::<Button>(button).label(), "Save");

    // The structural stack publishes nothing, so it is not a candidate.
    assert_eq!(app.find(Selector::role(SemanticsRole::Dialog)), None);
}

#[test]
fn a_tap_delivers_press_and_release_and_reports_damage() {
    let clicks = Arc::new(AtomicUsize::new(0));
    let mut app = TestApp::new(Size::new(200.0, 80.0), counting_button(Arc::clone(&clicks)));

    assert!(app.tap("Save"), "the click repainted");
    assert_eq!(
        clicks.load(Ordering::SeqCst),
        1,
        "the release completed the click"
    );

    // The centre of the box really is inside it, or the tap would have missed.
    let centre = app.centre("Save");
    let bounds = app.bounds("Save");
    assert!(bounds.contains(centre));
}

#[test]
fn a_press_alone_does_not_complete_a_click() {
    let clicks = Arc::new(AtomicUsize::new(0));
    let mut app = TestApp::new(Size::new(200.0, 80.0), counting_button(Arc::clone(&clicks)));

    app.press(app.centre("Save"));
    assert_eq!(
        clicks.load(Ordering::SeqCst),
        0,
        "a press is not yet a click"
    );
    app.release(app.centre("Save"));
    assert_eq!(clicks.load(Ordering::SeqCst), 1);
}

// The two IME cases that were here moved to `floom-ui-composites/tests/text_field.rs`
// with the widget: a `floom-testkit` test cannot mount a widget from the satellite
// `floom-ui` workspace, and the field is the only thing in the tree that has a
// value or a composition for those two assertions to be about. They still test
// this crate -- `floom-ui-composites` depends on `floom-testkit`, so the same `TestApp`
// delivers the typed text and the preedit underneath.

#[test]
fn a_tap_on_a_checkbox_toggles_it() {
    let mut app = TestApp::new(Size::new(200.0, 32.0), Checkbox::new("Agree"));
    assert!(!app.widget::<Checkbox>("Agree").is_checked());

    app.tap("Agree");
    assert!(app.widget::<Checkbox>("Agree").is_checked());
    app.assert_text("Agree", "Agree");
}

#[test]
fn settling_an_already_settled_tree_finds_nothing_to_do() {
    let mut app = TestApp::new(Size::new(200.0, 32.0), Switch::new("Wi-Fi"));

    // The actor drained the frames its own interaction asked for, so there is
    // nothing left over — settling is a no-op rather than a second pass.
    assert!(app.tap("Wi-Fi"));
    assert!(!app.settle());
    assert!(!app.damaged(), "a settled tree has nothing left to paint");
    assert!(!app.host().needs_frame());
}

/// The case worth naming: a change made the way an app callback makes one —
/// straight into the host, with no platform event to announce it. The tree owes
/// a frame and the platform has nothing queued to deliver it, so waiting for
/// the platform to speak would wait forever. `settle` asks for the frame
/// instead, and rides out the overlay's entry animation while it is at it.
///
/// The overlay goes up through the harness's own door rather than through a
/// popup widget: what is under test is `settle`, and the overlay only has to
/// be something the tree owes frames for.
#[test]
fn settling_runs_the_frames_an_out_of_band_change_owes() {
    let mut app = TestApp::new(Size::new(240.0, 320.0), Button::new("anchor"));
    assert_eq!(app.overlay_count(), 0, "nothing is up yet");

    app.host_mut().open_overlay(
        OverlayKind::Popup,
        false,
        Rect::from_origin_size(Point::new(8.0, 8.0), Size::new(120.0, 40.0)),
        Text::new("item"),
    );
    assert!(app.settle(), "settling ran the frames the change owed");
    assert_eq!(app.overlay_count(), 1, "and the overlay is up");
    assert!(!app.damaged());
    assert!(!app.host().needs_frame());
}

#[test]
fn selectors_accept_a_node_handle_as_well_as_a_label() {
    let app = TestApp::new(Size::new(200.0, 80.0), labelled_button("Save"));
    let root = app.root();

    // `Into<Selector>` means the same call sites work with either.
    assert_eq!(app.node(root), root);
    assert_eq!(app.node("Save"), app.find_widget::<Button>().unwrap());
    assert_eq!(app.widget::<Button>("Save").label(), "Save");
}

#[test]
fn the_scene_is_readable_after_a_frame() {
    let mut app = TestApp::new(DEFAULT_SIZE, labelled_button("Save"));
    app.frame();

    let nodes = app.scene_nodes();
    assert!(!nodes.is_empty(), "a painted frame produced scene nodes");
    assert_eq!(nodes.len(), app.scene_ids().len());

    // The button's own fill is in there, somewhere near its box.
    let bounds = app.bounds("Save");
    let painted_over_the_button = nodes.iter().any(|node| {
        node.bounds_hint()
            .is_some_and(|hint| hint.x0 < bounds.x1 && bounds.x0 < hint.x1)
    });
    assert!(painted_over_the_button, "the button was painted");
}

#[test]
fn resizing_reaches_the_tree() {
    let mut app = TestApp::new(Size::new(200.0, 80.0), labelled_button("Save"));
    let before = app.bounds("Save");

    app.resize(Size::new(400.0, 80.0));
    app.settle();
    assert_eq!(app.size(), Size::new(400.0, 80.0));
    assert_eq!(
        app.bounds("Save"),
        before,
        "a vertical column keeps its button where it was"
    );
}

#[test]
fn a_point_outside_everything_hits_nothing() {
    let mut app = TestApp::new(Size::new(200.0, 80.0), labelled_button("Save"));

    // Far below the column: no widget claims it, so no widget repaints.
    assert!(
        !app.click(Point::new(10.0, 70.0)),
        "a click on empty space changes nothing"
    );
}
