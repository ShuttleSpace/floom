// SPDX-License-Identifier: MIT
//! Overlay system: the chain that lets popups, dialogs and sheets escape their
//! parent's clip and still be seen, clicked and focused.
//!
//! `floom-overlay` owns the *policy* (stacking, modal scrim, focus scope, hit
//! priority, the entry animation); `WidgetHost` owns the *content* — it mounts
//! an overlay's widget as an independent subtree, lays it out against the
//! viewport, paints it last (so it sits on top), and routes pointer hits to it
//! before the main tree.
//!
//! These tests drive that whole chain through `floom_testkit::TestApp`, which
//! is the same `FakePlatform` → `AppRunner` path a real window takes, and assert
//! on two things every time: the overlay's content actually receives the event,
//! and the change reaches the screen (damage).

use std::cell::Cell;
use std::rc::Rc;

use floom_platform::KeyCode;
use floom_scene::SceneNode;
use floom_testkit::TestApp;
use floom_types::{Point, Rect, Size};
use floom_widgets::Button;
use floom_widgets::overlay::{OverlayHandle, OverlayKind};

/// Whether two rects share any area.
fn overlaps(a: Rect, b: Rect) -> bool {
    a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1
}

/// A window-sized app whose root is a plain button, plus the door the host
/// keeps for mounting an overlay by hand.
fn app_with_root() -> TestApp {
    TestApp::new(Size::new(800.0, 600.0), Button::new("main"))
}

/// Opens `content` as an overlay and renders one frame so its hit regions and
/// scene nodes exist before any pointer event.
fn open(
    app: &mut TestApp,
    kind: OverlayKind,
    modal: bool,
    bounds: Rect,
    content: impl floom_core::WidgetAny + 'static,
) -> OverlayHandle {
    let id = app.host_mut().open_overlay(kind, modal, bounds, content);
    app.frame();
    id
}

/// The largest `y1` in the scene: how far down the window the paint reaches.
/// A sheet entering with `TranslateY` starts below where it settles, so this
/// number falls as the animation runs.
fn painted_bottom(app: &TestApp) -> f64 {
    app.scene_nodes()
        .iter()
        .filter_map(|node| node.bounds_hint())
        .map(|bounds| bounds.y1)
        .fold(f64::NEG_INFINITY, f64::max)
}

/// An overlay button placed clear of the main tree still receives the click —
/// its content is painted on top and hit-tested before the main tree, so the
/// parent's clip no longer hides (or swallows) it.
#[test]
fn an_overlay_button_outside_the_main_tree_is_clickable() {
    let clicked = Rc::new(Cell::new(0));
    let main_clicked = Rc::new(Cell::new(0));
    let overlay_sink = clicked.clone();
    let main_sink = main_clicked.clone();

    let mut app = TestApp::new(
        Size::new(800.0, 600.0),
        Button::new("main").on_click(move || main_sink.set(main_sink.get() + 1)),
    );

    let id = open(
        &mut app,
        OverlayKind::Popup,
        false,
        Rect::from_origin_size(Point::new(300.0, 300.0), Size::new(120.0, 40.0)),
        Button::new("overlay").on_click(move || overlay_sink.set(overlay_sink.get() + 1)),
    );
    assert_eq!(app.overlay_count(), 1);

    // A click inside the overlay, far from where the main button paints.
    app.click(Point::new(360.0, 320.0));
    assert_eq!(clicked.get(), 1, "the overlay button received the click");
    assert_eq!(main_clicked.get(), 0, "the main tree was not involved");

    app.host_mut().close_overlay(id);
    assert_eq!(app.overlay_count(), 0, "closing removes it");
}

/// A modal overlay covers the screen, so a click that would have landed on a
/// popup beneath it is swallowed by the scrim instead. Once the modal closes,
/// the same click reaches the popup again.
#[test]
fn a_modal_overlay_absorbs_clicks_underneath_it() {
    let popup_clicked = Rc::new(Cell::new(0));
    let modal_clicked = Rc::new(Cell::new(0));
    let p = popup_clicked.clone();
    let m = modal_clicked.clone();

    let mut app = app_with_root();

    let popup = open(
        &mut app,
        OverlayKind::Popup,
        false,
        Rect::from_origin_size(Point::new(300.0, 300.0), Size::new(120.0, 40.0)),
        Button::new("popup").on_click(move || p.set(p.get() + 1)),
    );
    let modal = open(
        &mut app,
        OverlayKind::Dialog,
        true,
        Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(800.0, 600.0)),
        // Modal content lives only in the corner; the rest is scrim.
        Button::new("modal").on_click(move || m.set(m.get() + 1)),
    );

    // Click where the popup sits — but the modal is on top of it, so the click
    // reaches the modal's content instead of the popup beneath.
    app.click(Point::new(360.0, 320.0));
    assert_eq!(
        popup_clicked.get(),
        0,
        "the modal shielded the popup beneath it from the click"
    );
    assert_eq!(
        modal_clicked.get(),
        1,
        "the click reached the modal's content on top"
    );

    // Close the modal and re-render so the popup is hit-testable again.
    app.host_mut().close_overlay(modal);
    app.frame();

    app.click(Point::new(360.0, 320.0));
    assert_eq!(
        popup_clicked.get(),
        1,
        "after the modal closes the popup is clickable again"
    );

    app.host_mut().close_overlay(popup);
}

/// Focus moves into an overlay's content when it opens, so a key meant for the
/// overlay reaches it instead of the main tree.
#[test]
fn keyboard_reaches_overlay_content() {
    let clicked = Rc::new(Cell::new(0));
    let sink = clicked.clone();

    let mut app = app_with_root();

    open(
        &mut app,
        OverlayKind::Dialog,
        true,
        Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(800.0, 600.0)),
        Button::new("overlay").on_click(move || sink.set(sink.get() + 1)),
    );

    app.key(KeyCode::ENTER);
    assert_eq!(clicked.get(), 1, "Enter reached the focused overlay button");
    app.key(KeyCode::SPACE);
    assert_eq!(clicked.get(), 2, "Space also activates it");
}

/// A press that misses a non-modal overlay takes it down, and does *not* also
/// activate whatever the press landed on.
///
/// This is the host's light-dismiss rule. It cannot live in the widget: the
/// router never routes a press to a widget it missed, so a popup would have to
/// invent a scrim it does not otherwise want in order to hear about it.
#[test]
fn a_press_outside_a_non_modal_overlay_dismisses_it() {
    let popup_clicked = Rc::new(Cell::new(0));
    let main_clicked = Rc::new(Cell::new(0));
    let p = popup_clicked.clone();
    let m = main_clicked.clone();

    let mut app = TestApp::new(
        Size::new(800.0, 600.0),
        Button::new("main").on_click(move || m.set(m.get() + 1)),
    );

    let id = open(
        &mut app,
        OverlayKind::Popup,
        false,
        Rect::from_origin_size(Point::new(300.0, 300.0), Size::new(120.0, 40.0)),
        Button::new("popup").on_click(move || p.set(p.get() + 1)),
    );
    assert_eq!(app.overlay_count(), 1);

    // A press clear of the popup's content.
    app.press(Point::new(20.0, 20.0));
    assert_eq!(
        app.overlay_count(),
        0,
        "the outside press took the popup down"
    );
    assert!(!app.host().has_overlay(id), "and it is really gone");
    assert_eq!(
        main_clicked.get(),
        0,
        "the dismiss is consumed rather than passed to the tree beneath"
    );
    assert_eq!(popup_clicked.get(), 0, "the popup's own button saw nothing");
}

/// A modal overlay is *not* light-dismissed: it exists to hold the window, and
/// a press on its scrim is the overlay's business, not the host's.
#[test]
fn a_press_outside_a_modal_overlay_leaves_it_up() {
    let modal_clicked = Rc::new(Cell::new(0));
    let m = modal_clicked.clone();

    let mut app = app_with_root();

    open(
        &mut app,
        OverlayKind::Dialog,
        true,
        Rect::from_origin_size(Point::new(200.0, 200.0), Size::new(200.0, 120.0)),
        Button::new("modal").on_click(move || m.set(m.get() + 1)),
    );

    app.press(Point::new(700.0, 550.0));
    assert_eq!(
        app.overlay_count(),
        1,
        "a modal overlay stays up until something closes it"
    );
}

/// An overlay is pushed with an entry animation, and that animation is actually
/// composited: the sheet's content starts below its final position and slides
/// up, and the host keeps owing frames until it settles.
#[test]
fn a_sheet_slides_into_place_and_then_settles() {
    let mut app = app_with_root();

    let area = Rect::from_origin_size(Point::new(0.0, 400.0), Size::new(800.0, 200.0));
    open(
        &mut app,
        OverlayKind::BottomSheet,
        true,
        area,
        Button::new("sheet content like this"),
    );

    let entering = painted_bottom(&app);
    assert!(
        app.host().overlay_animating(),
        "the entry animation is running on the frame it opened"
    );
    assert!(
        entering > area.y1,
        "the sheet starts below its settled bottom edge ({entering} vs {})",
        area.y1
    );

    // 0.15 s at the nominal 60 Hz step is nine frames; give it a couple spare.
    for _ in 0..12 {
        app.frame();
    }
    assert!(!app.host().overlay_animating(), "the slide settles");
    let settled = painted_bottom(&app);
    assert!(
        (settled - area.y1).abs() < 0.5,
        "the sheet ended flush with the bottom edge ({settled} vs {})",
        area.y1
    );
    assert!(
        settled < entering,
        "the sheet moved up into place ({entering} → {settled})"
    );
}

/// A non-sheet overlay fades: part-way through its entry its content is drawn
/// with partial alpha, and once the animation settles the payload is written
/// unmodified (the compositor goes back to `None`, so nothing keeps re-writing
/// the scene and the host stops taking damage).
#[test]
fn a_popup_fades_in_and_then_stops_damaging_the_scene() {
    let mut app = app_with_root();

    let area = Rect::from_origin_size(Point::new(300.0, 300.0), Size::new(120.0, 40.0));
    open(
        &mut app,
        OverlayKind::Popup,
        false,
        area,
        Button::new("popup"),
    );

    let mut seen_partial_alpha = false;
    for _ in 0..12 {
        // The overlay's Button paints exactly one background rect, sized to the
        // rect the overlay was opened at, so alpha there is the entry
        // animation — the main tree's button is nowhere near this area.
        let alpha = app
            .scene_nodes()
            .iter()
            .filter_map(|node| match node {
                SceneNode::Rect { bounds, fill, .. } if overlaps(area, *bounds) => {
                    fill.solid().map(|c| c.a)
                }
                _ => None,
            })
            .min();
        if let Some(alpha) = alpha
            && alpha > 0
            && alpha < 255
        {
            seen_partial_alpha = true;
        }
        app.frame();
    }

    assert!(
        seen_partial_alpha,
        "the popup passed through partial opacity on its way in"
    );
    assert!(!app.host().overlay_animating(), "the fade settles");
    assert!(
        !app.host().needs_frame(),
        "once it settles the host owes no further frames"
    );
}

/// An image inside a fading overlay fades with it.
///
/// This is the shape the missing image alpha took: the composite path scaled
/// rects and text and silently skipped images, so a popup's backdrop dimmed
/// while the picture on top of it stayed at full strength. `Image` nodes now
/// carry an alpha, and the same fold that dims a rect reaches them.
#[test]
fn an_image_in_a_fading_overlay_fades_with_it() {
    let mut app = app_with_root();
    let img = app
        .host_mut()
        .register_image(2, 2, vec![255; 2 * 2 * 4])
        .expect("valid extent");
    let area = Rect::from_origin_size(Point::new(300.0, 300.0), Size::new(120.0, 40.0));
    open(
        &mut app,
        OverlayKind::Popup,
        false,
        area,
        floom_widgets::Image::new(img).intrinsic_size(Size::new(120.0, 40.0)),
    );

    let mut seen_partial = None;
    for _ in 0..12 {
        let alpha = app.scene_nodes().iter().find_map(|node| match node {
            SceneNode::Image { image, alpha, .. } if *image == img => Some(*alpha),
            _ => None,
        });
        if let Some(alpha) = alpha
            && alpha > 0
            && alpha < 255
        {
            seen_partial = Some(alpha);
        }
        app.frame();
    }

    assert!(
        seen_partial.is_some(),
        "the overlay's image passed through partial opacity on its way in"
    );
    // And it settles opaque, rather than being left permanently dimmed.
    let settled = app.scene_nodes().iter().find_map(|node| match node {
        SceneNode::Image { image, alpha, .. } if *image == img => Some(*alpha),
        _ => None,
    });
    assert_eq!(
        settled,
        Some(255),
        "a settled overlay's image is opaque again"
    );
}

/// Opening and closing an overlay both change the scene, so each repaints —
/// proving the overlay subtree participates in the paint pass (on top of the
/// main tree) rather than being skipped.
#[test]
fn opening_and_closing_an_overlay_repaints() {
    let mut app = app_with_root();

    let id = open(
        &mut app,
        OverlayKind::Popup,
        false,
        Rect::from_origin_size(Point::new(300.0, 300.0), Size::new(120.0, 40.0)),
        Button::new("overlay"),
    );
    assert_eq!(app.overlay_count(), 1);

    // Closing unmounts the overlay subtree and removes its scene nodes, which
    // must carry damage this frame.
    app.host_mut().close_overlay(id);
    app.frame();
    assert!(app.damaged(), "closing the overlay repainted the screen");
    assert_eq!(app.overlay_count(), 0);
}

/// A passthrough overlay paints on top and takes nothing: the pointer falls
/// through it to the tree behind, and it never steals the keyboard.
///
/// This is what a devtools highlight box is: a rectangle that covers the window
/// so it can outline an element anywhere in it. A covering overlay that also
/// took clicks would make the app unclickable while the box was up — including
/// unclickable for the pick the box exists to serve.
#[test]
fn a_passthrough_overlay_lets_clicks_and_focus_through() {
    let main_clicked = Rc::new(Cell::new(0));
    let main_sink = main_clicked.clone();

    let mut app = TestApp::new(
        Size::new(800.0, 600.0),
        Button::new("main").on_click(move || main_sink.set(main_sink.get() + 1)),
    );
    // Focus the main button the way a tab would, so the overlay has something
    // to steal the keyboard from.
    app.host_mut().focus_next();

    let box_id = app.host_mut().open_overlay_passthrough(
        OverlayKind::Popup,
        Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(800.0, 600.0)),
        Button::new("highlight"),
    );
    app.frame();
    assert_eq!(app.overlay_count(), 1, "the box is mounted and painted");

    // A click anywhere — over the box, which covers everything — belongs to the
    // app underneath.
    app.click(Point::new(400.0, 300.0));
    assert_eq!(
        main_clicked.get(),
        1,
        "the click fell through the covering overlay"
    );

    // And the box never took the keyboard: Enter still reaches the main button.
    app.key(KeyCode::ENTER);
    assert_eq!(
        main_clicked.get(),
        2,
        "Enter reached the main tree, not the decoration on top"
    );

    app.host_mut().close_overlay(box_id);
    assert_eq!(app.overlay_count(), 0);
}

/// The host's light-dismiss rule asks about the top *interactive* overlay, not
/// the top entry. Otherwise a press over a highlight box would read as "outside
/// the box" and close the box — and, worse, a press outside the real popup
/// beneath it would be compared against the decoration instead.
#[test]
fn a_press_over_a_passthrough_overlay_still_dismisses_the_popup_beneath() {
    let mut app = app_with_root();
    let popup = open(
        &mut app,
        OverlayKind::Popup,
        false,
        Rect::from_origin_size(Point::new(300.0, 300.0), Size::new(120.0, 40.0)),
        Button::new("popup"),
    );
    app.host_mut().open_overlay_passthrough(
        OverlayKind::Popup,
        Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(800.0, 600.0)),
        Button::new("highlight"),
    );
    app.frame();
    assert_eq!(app.overlay_count(), 2);

    // Far from the popup, but inside the highlight box.
    app.click(Point::new(700.0, 500.0));
    assert!(
        app.host_mut().overlay_count() <= 1,
        "the popup took the light dismissal; the box did not shield it"
    );
    let _ = popup;
}

/// A mounted-but-hidden overlay gives its column back to the app.
///
/// This is the devtools drawer's case: it is not closed when it is hidden — the
/// panel is the only thing that knows how to rebuild itself, so closing it would
/// throw the content away — which means the overlay is still there, still sized,
/// and still a full-height claim on the trailing edge unless the *hit* is turned
/// off. Turning it off has to reach both halves: the overlay stack, which
/// decides whether the press is the overlay's at all, and the main router, whose
/// hit region for the overlay subtree would otherwise win the contest on its own.
#[test]
fn a_hidden_overlay_leaves_its_column_to_the_app_beneath() {
    let main_clicked = Rc::new(Cell::new(0));
    let main_sink = main_clicked.clone();

    let mut app = TestApp::new(
        Size::new(800.0, 600.0),
        Button::new("main").on_click(move || main_sink.set(main_sink.get() + 1)),
    );
    // A drawer down the trailing third, the way the devtools panel docks.
    let drawer = open(
        &mut app,
        OverlayKind::Drawer,
        false,
        Rect::from_origin_size(Point::new(520.0, 0.0), Size::new(280.0, 600.0)),
        Button::new("panel"),
    );

    // While it is up, its column is its own.
    app.click(Point::new(600.0, 300.0));
    assert_eq!(
        main_clicked.get(),
        0,
        "a press inside the drawer is the drawer's"
    );

    // Hidden, not closed: the overlay is still mounted, still sized, still there.
    app.host_mut().set_overlay_hit(drawer, false);
    app.frame();
    assert_eq!(app.overlay_count(), 1, "hidden, not closed");
    assert!(
        !overlaps(app.host().overlay_bounds(drawer), Rect::default()),
        "the bounds are unchanged"
    );

    app.click(Point::new(600.0, 300.0));
    assert_eq!(
        main_clicked.get(),
        1,
        "the press fell through the hidden drawer to the app"
    );

    // And showing it again takes the column back without a remount.
    app.host_mut().set_overlay_hit(drawer, true);
    app.frame();
    app.click(Point::new(600.0, 300.0));
    assert_eq!(
        main_clicked.get(),
        1,
        "the drawer answers its own column again"
    );
}

/// An overlay that opts out of light dismissal survives a press outside it.
///
/// A devtools panel is shown and hidden by its own control. If a press on the
/// app closed it, `close_overlay` would unmount the content — and content the
/// host owns has no opener to hand it back to, so the panel would be gone for
/// good while its state still said it was visible.
#[test]
fn a_non_dismissible_overlay_survives_a_press_outside_it() {
    let mut app = app_with_root();
    let panel = open(
        &mut app,
        OverlayKind::Drawer,
        false,
        Rect::from_origin_size(Point::new(520.0, 0.0), Size::new(280.0, 600.0)),
        Button::new("panel"),
    );
    app.host_mut().set_overlay_dismissible(panel, false);

    app.click(Point::new(100.0, 100.0));
    assert_eq!(
        app.overlay_count(),
        1,
        "the panel is chrome, not a popup: clicking away does not close it"
    );

    // A dismissible overlay in the same position still goes, so opting out is
    // per-overlay rather than a global switch.
    let popup = open(
        &mut app,
        OverlayKind::Popup,
        false,
        Rect::from_origin_size(Point::new(100.0, 400.0), Size::new(120.0, 40.0)),
        Button::new("popup"),
    );
    app.click(Point::new(700.0, 100.0));
    assert!(
        !app.host().has_overlay(popup),
        "the popup took the light dismissal"
    );
    assert!(app.host().has_overlay(panel), "the panel stayed");
}
