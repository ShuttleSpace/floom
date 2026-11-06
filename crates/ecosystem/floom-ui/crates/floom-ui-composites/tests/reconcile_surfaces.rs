use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_app::WidgetHost;
use floom_core::{Axis, Child, ChildKey, LayoutRequest, Point, Rect, Size, WidgetAny};
use floom_layout::LayoutStyle;
use floom_testkit::TestApp;
use floom_types::PointerEvent;
use floom_ui_composites::{BottomSheet, Dialog, Popup, Tabs};
use floom_widgets::{Button, Conditional, Grid, List, Stack, Text};

const VIEWPORT: Size = Size::new(240.0, 320.0);

/// Stable FNV-1a key for the string items used by the list/grid tests.
fn key_of(s: &String) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for byte in s.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}
fn sized_column(
    width: f32,
    height: f32,
    build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
) -> impl WidgetAny + 'static {
    Stack::new(Axis::Vertical).child(Child::new(
        ChildKey::index(0),
        LayoutRequest::new().width(width).height(height),
        build,
    ))
}

#[test]
fn dialog_hands_its_surface_to_a_host_overlay() {
    let dialog = Dialog::new().title("Hello").body(Child::new(
        ChildKey::index(0),
        LayoutRequest::new(),
        || Box::new(Text::new("body")),
    ));
    let mut app = TestApp::unpinned(VIEWPORT, dialog);
    let id = app.root();
    let stats = app.frame();
    assert_eq!(stats.painted, 1, "hidden dialog is alone");
    assert_eq!(
        app.overlay_count(),
        0,
        "and nothing is stacked above the tree"
    );

    app.widget_mut::<Dialog>(id).set_present(true);
    let stats = app.frame();
    // The card is no longer an in-tree subtree: it moved onto the host's overlay
    // stack, which is what lets it cover the window instead of the box it was
    // declared in.
    assert_eq!(
        app.overlay_count(),
        1,
        "the dialog surface is a host overlay"
    );
    // dialog node + card stack + title text + body text
    assert_eq!(stats.painted, 4, "dialog surface mounted as an overlay");

    app.widget_mut::<Dialog>(id).set_present(false);
    app.frame();
    assert_eq!(app.overlay_count(), 0, "closing takes it off the stack");
    assert_eq!(app.frame().painted, 1, "dialog surface fully unmounted");
}

/// The point of routing a `Dialog` through the host is that it stops being
/// confined to the box it was declared in. This mounts one inside a small
/// parent, then clicks the centred card's button — a point far outside that
/// parent, which an in-tree surface could never have reached.
#[test]
fn dialog_escapes_the_box_it_was_declared_in() {
    // `Arc`/`AtomicUsize` rather than `Rc`/`Cell`: a `Child` builder must be
    // `Send + Sync` because the host may move a subtree between threads.
    let clicked = Arc::new(AtomicUsize::new(0));
    let sink = clicked.clone();

    // A deliberately tiny parent in the top-left corner: anything laid out
    // inside it is nowhere near the viewport centre.
    let parent = sized_column(40.0, 40.0, move || {
        let sink = sink.clone();
        Box::new(Dialog::new().title("Hi").body(Child::new(
            ChildKey::index(0),
            LayoutRequest::new(),
            move || {
                let sink = sink.clone();
                Box::new(Button::new("ok").on_click(move || {
                    sink.fetch_add(1, Ordering::SeqCst);
                }))
            },
        )))
    });
    let mut app = TestApp::unpinned(VIEWPORT, parent);
    app.frame();

    let dialog = app.find_widget::<Dialog>().expect("the dialog mounted");
    app.widget_mut::<Dialog>(dialog).set_present(true);
    app.frame();
    assert_eq!(
        app.overlay_count(),
        1,
        "the dialog went onto the overlay stack"
    );

    // The card is centred on the *window*, so its button sits around the middle
    // of the 240x320 viewport — well clear of the 40x40 declaration box.
    let ok = app.node("ok");
    let bounds = app.bounds(ok);
    assert!(
        bounds.y0 > 40.0,
        "the card must be laid out against the viewport, not the parent: {bounds:?}"
    );
    assert!(app.tap(ok), "the overlay button takes the click");
    assert_eq!(clicked.load(Ordering::SeqCst), 1, "and its handler ran");
}

#[test]
fn tabs_swap_the_active_panel_on_click() {
    let tabs = Tabs::new()
        .tab(
            "One",
            Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                Box::new(Text::new("panel one"))
            }),
        )
        .tab(
            "Two",
            Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                Box::new(Text::new("panel two"))
            }),
        );
    let mut app = TestApp::unpinned(VIEWPORT, tabs);
    let id = app.root();
    let stats = app.frame();
    // tabs + bar(stack) + 2 buttons + active panel
    assert_eq!(stats.painted, 5);

    let selection = app.widget::<Tabs>(id).selection();
    assert_eq!(selection.load(Ordering::SeqCst), 0, "starts on tab zero");

    let labels_before = app.labels();
    assert!(
        labels_before.iter().any(|l| l == "panel one"),
        "first panel is visible"
    );
    assert!(
        !labels_before.iter().any(|l| l == "panel two"),
        "second panel is not mounted yet"
    );

    // Click the inactive tab to switch.
    let other = app.node("Two");
    assert!(app.tap(other), "tab button must be hittable");
    app.frame();

    assert_eq!(
        selection.load(Ordering::SeqCst),
        1,
        "clicking the tab bar flips the selection"
    );

    let labels_after = app.labels();
    assert!(
        labels_after.iter().any(|l| l == "panel two"),
        "active panel swapped in"
    );
    assert!(
        !labels_after.iter().any(|l| l == "panel one"),
        "old panel swapped out"
    );
    // Live widget count is unchanged by a swap (bar + 2 buttons + panel).
    assert_eq!(app.frame().painted, 5);
}

#[test]
fn popup_mounts_and_dismisses_its_layer() {
    let popup = Popup::new().label("menu").content(Child::new(
        ChildKey::index(0),
        LayoutRequest::new(),
        || Box::new(Text::new("item")),
    ));
    let mut app = TestApp::unpinned(VIEWPORT, popup);
    let id = app.root();
    assert_eq!(app.frame().painted, 1, "hidden popup is alone");
    assert_eq!(
        app.overlay_count(),
        0,
        "and nothing is stacked above the tree"
    );

    app.widget_mut::<Popup>(id).set_present(true);
    app.frame();
    assert_eq!(app.overlay_count(), 1, "the popup layer is a host overlay");
    // popup node + content
    assert_eq!(app.frame().painted, 2, "popup layer mounted");

    app.widget_mut::<Popup>(id).dismiss();
    app.frame();
    assert_eq!(app.overlay_count(), 0, "dismissing takes it off the stack");
    assert_eq!(app.frame().painted, 1, "popup layer unmounted");
}

/// A press outside a non-modal overlay is the host's light dismiss, and it runs
/// the ordinary close path rather than just dropping the entry: the opener's
/// `overlay_closed` fires, so the popup clears the `present` that put it there.
/// Skipping that would leave the opener believing its menu is still up, and the
/// next `set_present(true)` would then be a no-op.
#[test]
fn an_outside_press_dismisses_a_popup_and_clears_its_present() {
    // The popup anchors its overlay to its own box, so it needs a parent
    // smaller than the viewport for "outside" to mean anything.
    let parent = sized_column(80.0, 40.0, || {
        Box::new(Popup::new().label("menu").content(Child::new(
            ChildKey::index(0),
            LayoutRequest::new(),
            || Box::new(Text::new("item")),
        )))
    });
    let mut app = TestApp::unpinned(VIEWPORT, parent);
    app.frame();

    let popup = app.find_widget::<Popup>().expect("popup mounted");
    app.widget_mut::<Popup>(popup).set_present(true);
    app.frame();
    assert_eq!(app.overlay_count(), 1, "the popup is up");

    // Far from the popup's 80 x 40 box in the corner.
    app.press(Point::new(200.0, 300.0));
    assert_eq!(app.overlay_count(), 0, "the outside press dismissed it");
    assert!(
        !app.widget::<Popup>(popup).is_present(),
        "the opener was told, so it no longer believes the menu is open"
    );
}

#[test]
fn bottom_sheet_mounts_a_bottom_layer_when_shown() {
    let sheet = BottomSheet::new()
        .title("Share")
        .height(160.0)
        .body(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
            Box::new(Text::new("body"))
        }));
    let mut app = TestApp::unpinned(VIEWPORT, sheet);
    let id = app.root();
    assert_eq!(app.frame().painted, 1, "hidden sheet is alone");
    assert_eq!(
        app.overlay_count(),
        0,
        "and nothing is stacked above the tree"
    );

    app.widget_mut::<BottomSheet>(id).set_present(true);
    app.frame();
    assert_eq!(app.overlay_count(), 1, "the sheet is a host overlay");
    // sheet node + bottom column + card + title + body
    assert_eq!(app.frame().painted, 5, "sheet layer fully mounted");

    app.widget_mut::<BottomSheet>(id).set_present(false);
    app.frame();
    assert_eq!(app.overlay_count(), 0, "closing takes it off the stack");
    assert_eq!(app.frame().painted, 1, "sheet layer fully unmounted");
}
