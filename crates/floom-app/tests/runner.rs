//! End-to-end runner tests: FakePlatform → AppRunner → SoftwarePresenter.
//!
//! These exercise the full wiring path — platform events in, CPU pixels out —
//! without a window server or a GPU. The software presenter doubles as the
//! assertion surface: its RGBA buffer is inspectable pixel by pixel.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use floom_app::{AppRunner, RunStats, WidgetHost};
use floom_layout::LayoutStyle;
use floom_platform::{FakePlatform, KeyCode, Platform, PlatformEvent, WindowConfig};
use floom_renderer_software::SoftwarePresenter;
use floom_types::{Color, Point, Size};
use floom_widgets::{Button, Container, Stack, Text, TextField};

/// A tiny window (64×48 logical, scale 1) so pixel arithmetic stays trivial.
fn small_window() -> WindowConfig {
    WindowConfig {
        title: "runner-test".into(),
        size: Size::new(64.0, 48.0),
        resizable: true,
    }
}

/// Content changed outside the frame loop still reaches the screen, and stops
/// asking for frames once the tree settles.
///
/// This is the gap a static window exposes: nothing in the platform queue
/// announces "someone edited a widget", so without a way to ask for a frame the
/// edit would sit there until the next unrelated event.
#[test]
fn content_edited_outside_the_frame_loop_drives_frames_until_it_settles() {
    let mut platform = FakePlatform::new();
    let mut host = WidgetHost::new();
    let label = host.mount_root(LayoutStyle::default(), Text::new("before"));
    let mut runner = AppRunner::new(host, SoftwarePresenter::new());

    platform.create_window(small_window());
    settle(&mut platform, &mut runner);
    let settled_frames = runner.stats().frames;
    assert!(settled_frames >= 1, "the first frame painted");

    // An app callback rewrites the text: no platform event of its own.
    if let Some(text) = runner.host_mut().widget_mut::<Text>(label) {
        text.set_content("after");
    }
    assert!(
        runner.host().needs_frame(),
        "borrowing a widget mutably marks the tree dirty"
    );
    runner.request_frame(&mut platform);
    settle(&mut platform, &mut runner);
    assert!(
        runner.stats().frames > settled_frames,
        "the edited content reached the screen"
    );

    // A cascade keeps asking (the re-measured text re-lays-out and repaints),
    // but a tree that stopped changing stops asking.
    let after_edit = runner.stats().frames;
    for _ in 0..8 {
        runner.pump(&mut platform);
    }
    assert_eq!(
        runner.stats().frames,
        after_edit,
        "a settled tree stops requesting frames"
    );
    assert!(
        !runner.host().needs_frame(),
        "nothing left to do after the cascade settled"
    );
    assert!(
        runner.stats().auto_redraws >= 1,
        "the frame loop asked for at least one frame on its own"
    );
}

/// Pumps a few times so the queue drains and the requested frame is presented.
///
/// `request_frame` pushes a `RedrawRequested` back onto the platform queue, so
/// one pump processes an event and the next paints for it. Four pumps always
/// reaches the stable "created and presented" state for one window.
fn settle(platform: &mut FakePlatform, runner: &mut AppRunner<SoftwarePresenter>) -> RunStats {
    let mut stats = RunStats::default();
    for _ in 0..4 {
        stats = runner.pump(platform);
    }
    stats
}

/// A window born on a display of twice the density hands the host that density
/// at creation, not at the first resize.
///
/// Logical units and device pixels are different units, and a native participant
/// is placed in the second: the host is what converts, so a host still at its 1x
/// default places every OS view at half size inside a window the framework draws
/// correctly. The scale is knowable only through the window, which makes creation
/// the one moment it has to be read.
#[test]
fn a_window_born_at_two_x_hands_the_host_two_x() {
    let mut platform = FakePlatform::new().with_scale_factor(2.0);
    let mut host = WidgetHost::new();
    host.mount_root(LayoutStyle::default(), Text::new("hi"));
    let mut runner = AppRunner::new(host, SoftwarePresenter::new());

    platform.create_window(small_window());
    settle(&mut platform, &mut runner);

    assert_eq!(
        runner.host().scale(),
        2.0,
        "the host lays out in logical units but places participants in device pixels"
    );
}

#[test]
fn window_creation_attaches_and_presents() {
    let mut platform = FakePlatform::new();
    let mut host = WidgetHost::new();
    host.mount_root(
        LayoutStyle::default(),
        Container::new().background(Color::rgb(255, 0, 0)),
    );
    let mut runner = AppRunner::new(host, SoftwarePresenter::new());

    let win = platform.create_window(small_window());
    settle(&mut platform, &mut runner);

    // The presenter is attached to the window's surface and painted it.
    assert!(runner.presenter_ready(), "surface attached after create");
    assert!(runner.presenter().is_attached());
    assert_eq!(runner.window_size(), Some(Size::new(64.0, 48.0)));
    assert_eq!(runner.presenter().width(), 64);
    assert_eq!(runner.presenter().height(), 48);
    assert!(runner.stats().frames >= 1, "at least one frame presented");

    // The root container paints the whole viewport red.
    let pixel = runner.presenter().pixel_rgba(32, 24).expect("in bounds");
    assert_eq!(pixel, [255, 0, 0, 255], "viewport is the container fill");

    let _ = win;
}

#[test]
fn a_click_reaches_a_button() {
    let clicked = Arc::new(AtomicBool::new(false));
    let flag = clicked.clone();

    let mut platform = FakePlatform::new();
    let mut host = WidgetHost::new();
    // Pinned to the window: this test exercises click routing, not the
    // button's intrinsic (label-fitting) size.
    host.mount_root(
        LayoutStyle {
            width: Some(64.0),
            height: Some(48.0),
            ..LayoutStyle::default()
        },
        Button::new("Go").on_click(move || flag.store(true, Ordering::SeqCst)),
    );
    let mut runner = AppRunner::new(host, SoftwarePresenter::new());

    let win = platform.create_window(small_window());
    settle(&mut platform, &mut runner);
    assert!(!clicked.load(Ordering::SeqCst), "no click yet");

    // Click the centre of the window; the button *is* the root, so it fills
    // the whole viewport.
    platform.inject_pointer_click(win, Point::new(32.0, 24.0));
    settle(&mut platform, &mut runner);

    assert!(
        clicked.load(Ordering::SeqCst),
        "the platform click was delivered to the button"
    );
    assert!(
        runner.stats().input_dispatched >= 2,
        "down + up were dispatched: {:?}",
        runner.stats()
    );
}

#[test]
fn resize_relays_to_host_and_presenter() {
    let mut platform = FakePlatform::new();
    let mut host = WidgetHost::new();
    host.mount_root(
        LayoutStyle::default(),
        Container::new().background(Color::rgb(0, 0, 255)),
    );
    let mut runner = AppRunner::new(host, SoftwarePresenter::new());

    let win = platform.create_window(small_window());
    settle(&mut platform, &mut runner);
    assert_eq!(runner.presenter().width(), 64);

    // Resize to 200×150.
    platform.push_event(PlatformEvent::WindowResized {
        window: win,
        size: Size::new(200.0, 150.0),
    });
    settle(&mut platform, &mut runner);

    assert_eq!(runner.window_size(), Some(Size::new(200.0, 150.0)));
    assert_eq!(runner.presenter().width(), 200, "buffer re-sized");
    assert_eq!(runner.presenter().height(), 150);
    // The repaint covered the new extent (blue now, not stale red garbage).
    let pixel = runner
        .presenter()
        .pixel_rgba(150, 75)
        .expect("new extent in bounds");
    assert_eq!(pixel, [0, 0, 255, 255]);
}

#[test]
fn closing_the_window_detaches_the_surface() {
    let mut platform = FakePlatform::new();
    let mut host = WidgetHost::new();
    host.mount_root(
        LayoutStyle::default(),
        Container::new().background(floom_types::Color::WHITE),
    );
    let mut runner = AppRunner::new(host, SoftwarePresenter::new());

    let win = platform.create_window(small_window());
    settle(&mut platform, &mut runner);
    assert!(runner.presenter_ready());

    platform.push_event(PlatformEvent::WindowClosed { window: win });
    settle(&mut platform, &mut runner);

    assert_eq!(runner.stats().windows_closed, 1);
    assert!(!runner.presenter_ready(), "surface detached on close");
    assert!(!runner.presenter().is_attached());
    assert_eq!(runner.primary_window(), None);
}

#[test]
fn keyboard_ime_and_suspend_flow_through() {
    let mut platform = FakePlatform::new();
    let mut host = WidgetHost::new();
    host.mount_root(
        LayoutStyle::default(),
        Container::new().background(floom_types::Color::WHITE),
    );
    let mut runner = AppRunner::new(host, SoftwarePresenter::new());

    let win = platform.create_window(small_window());
    settle(&mut platform, &mut runner);
    let before = runner.stats().input_dispatched;

    // One key press…
    platform.push_event(PlatformEvent::Key {
        window: win,
        code: KeyCode::from_char('a'),
        pressed: true,
        repeat: false,
        modifiers: floom_types::Modifiers::NONE,
    });
    // …and committed IME text ("hi" → two character key-downs).
    platform.push_event(PlatformEvent::ImeCommit {
        window: win,
        text: "hi".into(),
    });
    settle(&mut platform, &mut runner);

    assert_eq!(
        runner.stats().input_dispatched - before,
        3,
        "1 key + 2 committed characters"
    );

    // Suspending takes the surface away; it comes back with the window.
    platform.suspend();
    settle(&mut platform, &mut runner);
    assert!(!runner.presenter_ready(), "suspend detaches the surface");

    platform.resume();
    settle(&mut platform, &mut runner);
    assert!(runner.presenter_ready(), "resume re-attaches the surface");
}

/// An IME preedit lands on the focused text field and reports its caret rect
/// to the platform; committing finalizes the text and dismisses the popup.
///
/// This is the gap R1 closes: the runner used to drop `ImePreedit` entirely,
/// so a composition could never reach a widget. Now the runner routes the
/// preedit to the focused field, which owns the `floom_ime` state machine and
/// renders the marked text inline, and hands the OS the caret rect to anchor
/// its popup — then clears that area when the composition commits.
#[test]
fn ime_preedit_updates_focused_field_then_commit_finalizes() {
    let mut platform = FakePlatform::new();
    let mut host = WidgetHost::new();
    // Pinned to the window: this test exercises IME routing, not the field's
    // intrinsic (content-fitting) size, so clicks land squarely on it.
    let field = host.mount_root(
        LayoutStyle {
            width: Some(64.0),
            height: Some(48.0),
            ..LayoutStyle::default()
        },
        TextField::new(),
    );
    let mut runner = AppRunner::new(host, SoftwarePresenter::new());

    let win = platform.create_window(small_window());
    settle(&mut platform, &mut runner);

    // Focus the field by clicking it (the field is the root, so it fills the
    // whole 64×48 viewport). IME composition is meaningless without a target.
    platform.inject_pointer_click(win, Point::new(32.0, 24.0));
    settle(&mut platform, &mut runner);
    assert_eq!(
        runner.host().focused(),
        Some(runner.host().node_id(field)),
        "the field is focused after the click"
    );

    // A preedit arrives: the focused field owns the composition state.
    platform.push_event(PlatformEvent::ImePreedit {
        window: win,
        text: "あ".into(),
    });
    settle(&mut platform, &mut runner);

    let tf = runner
        .host()
        .widget::<TextField>(field)
        .expect("field alive");
    assert_eq!(tf.preedit(), "あ", "preedit text reached the field");
    assert!(tf.is_composing(), "a composition is in flight");
    assert_eq!(tf.text(), "", "nothing committed yet");

    // The composition caret rect was reported to the platform so the OS popup
    // can anchor above the right glyph.
    let area = platform.ime_cursor_area().expect("caret area reported");
    assert!(
        area.1.width > 0.0 && area.1.height > 0.0,
        "composing caret area is non-zero: {:?}",
        area
    );

    // Committing finalizes the text and clears the composition.
    platform.push_event(PlatformEvent::ImeCommit {
        window: win,
        text: "ok".into(),
    });
    settle(&mut platform, &mut runner);

    let tf = runner
        .host()
        .widget::<TextField>(field)
        .expect("field alive");
    assert_eq!(tf.text(), "ok", "committed text landed in the field");
    assert!(!tf.is_composing(), "composition ended on commit");
    assert_eq!(tf.preedit(), "", "preedit cleared on commit");

    // The runner dismissed the OS popup by pushing a zero-size cursor area.
    let area = platform.ime_cursor_area().expect("area still reported");
    assert_eq!(
        (area.1.width, area.1.height),
        (0.0, 0.0),
        "popup dismissed (zero-size caret area): {:?}",
        area
    );
}

// ---------------------------------------------------------------------------
// Error boundary integration
// ---------------------------------------------------------------------------

mod boundary {
    use super::*;
    use floom_error::ErrorBoundary;

    #[test]
    fn a_panicking_button_callback_becomes_a_report_not_a_crash() {
        // One button panics, one works — the app must survive the first and
        // the second still functions afterwards.
        let worked = Arc::new(AtomicBool::new(false));
        let worked_flag = worked.clone();

        let mut platform = FakePlatform::new();
        let mut host = WidgetHost::new();
        host.mount_root(
            LayoutStyle::default(),
            Stack::new(floom_core::Axis::Vertical)
                .push_layout(
                    floom_core::LayoutRequest::new().height(24.0).grow(1.0),
                    || {
                        Box::new(Button::new("boom").on_click(|| {
                            panic!("view blew up");
                        })) as Box<dyn floom_core::WidgetAny>
                    },
                )
                .push_layout(
                    floom_core::LayoutRequest::new().height(24.0).grow(1.0),
                    move || {
                        let flag = worked_flag.clone();
                        Box::new(Button::new("ok").on_click(move || {
                            flag.store(true, Ordering::SeqCst);
                        })) as Box<dyn floom_core::WidgetAny>
                    },
                ),
        );
        let boundary = ErrorBoundary::new();
        let mut runner =
            AppRunner::new(host, SoftwarePresenter::new()).with_error_boundary(boundary.clone());

        let win = platform.create_window(small_window());
        settle(&mut platform, &mut runner);

        // Click the top half ("boom"). The buttons are label-sized now, so
        // aim well inside their shared width.
        platform.inject_pointer_click(win, Point::new(16.0, 12.0));
        settle(&mut platform, &mut runner);

        assert!(
            !worked.load(Ordering::SeqCst),
            "sanity: the ok button was not the one clicked"
        );
        let reports = boundary.reports();
        assert_eq!(reports.len(), 1, "exactly one report: {:?}", reports);
        assert_eq!(reports[0].message, "view blew up");
        assert_eq!(reports[0].context, "pointer button");
        assert_eq!(runner.stats().errors_caught, 1);

        // The runner is still alive: click the bottom half ("ok").
        platform.inject_pointer_click(win, Point::new(16.0, 36.0));
        settle(&mut platform, &mut runner);
        assert!(
            worked.load(Ordering::SeqCst),
            "the app kept working after the caught panic"
        );
        assert_eq!(boundary.reports().len(), 1, "no new failures");
    }

    #[test]
    fn without_a_boundary_a_panic_propagates() {
        // Documents the default: no boundary, no catching. We cannot let the
        // panic escape the test, so drive `handle` under catch_unwind and
        // verify the stats never see a caught error.
        use std::panic::{AssertUnwindSafe, catch_unwind};

        let mut platform = FakePlatform::new();
        let mut host = WidgetHost::new();
        host.mount_root(
            LayoutStyle {
                width: Some(64.0),
                height: Some(48.0),
                ..LayoutStyle::default()
            },
            Button::new("boom").on_click(|| panic!("unhandled")),
        );
        let mut runner = AppRunner::new(host, SoftwarePresenter::new());
        let win = platform.create_window(small_window());
        settle(&mut platform, &mut runner);

        let result = catch_unwind(AssertUnwindSafe(|| {
            platform.inject_pointer_click(win, Point::new(32.0, 24.0));
            settle(&mut platform, &mut runner);
        }));
        assert!(result.is_err(), "the panic propagated out of the runner");
        assert_eq!(runner.stats().errors_caught, 0);
    }
}

// ---------------------------------------------------------------------------
// Service scopes
// ---------------------------------------------------------------------------

mod services {
    use super::*;
    use floom_services::Scope;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Session {
        user: &'static str,
    }

    /// Disposal writes into a shared flag; `Rc`/`RefCell` are fine here
    /// because service disposal runs on the UI thread by design.
    struct DisposalLog(Rc<RefCell<Vec<&'static str>>>);

    #[test]
    fn app_services_resolve_and_outlive_windows() {
        let mut platform = FakePlatform::new();
        let mut host = WidgetHost::new();
        host.mount_root(LayoutStyle::default(), Container::new());
        let mut runner = AppRunner::new(host, SoftwarePresenter::new());

        runner.services().provide(Session { user: "app-level" });

        let win = platform.create_window(small_window());
        settle(&mut platform, &mut runner);

        // The window scope resolves through the parent chain without a local
        // registration (window_scope, not the bare own-registry).
        assert!(runner.window_scope(win).get::<Session>().is_some());

        // Closing the window must NOT dispose app-scoped services.
        platform.push_event(PlatformEvent::WindowClosed { window: win });
        settle(&mut platform, &mut runner);
        assert!(runner.services().get::<Session>().is_some());
    }

    #[test]
    fn window_scoped_services_dispose_when_the_window_closes() {
        let mut platform = FakePlatform::new();
        let mut host = WidgetHost::new();
        host.mount_root(LayoutStyle::default(), Container::new());
        let mut runner = AppRunner::new(host, SoftwarePresenter::new());

        let win = platform.create_window(small_window());
        settle(&mut platform, &mut runner);

        let log = Rc::new(RefCell::new(Vec::new()));
        let log_flag = log.clone();
        runner.window_scope(win).services().provide_with_disposal(
            Session {
                user: "window-only",
            },
            Some(move |_| log_flag.borrow_mut().push("session disposed")),
        );

        platform.push_event(PlatformEvent::WindowClosed { window: win });
        settle(&mut platform, &mut runner);

        assert_eq!(
            *log.borrow(),
            vec!["session disposed"],
            "closing the window disposed its scope"
        );
    }

    #[test]
    fn child_scopes_compose_for_route_level_state() {
        // The app composes deeper scopes itself (a route, a dialog); this
        // pins the pattern the runner's window scopes follow.
        let root = Scope::root();
        root.services().provide(Session { user: "ada" });

        let route = Scope::child(root.services());
        route
            .services()
            .provide(DisposalLog(Rc::new(RefCell::new(Vec::new()))));

        assert_eq!(route.require::<Session>().user, "ada", "parent resolves");
        assert!(route.get::<DisposalLog>().is_some());
        assert!(
            root.services().get::<DisposalLog>().is_none(),
            "no leak upward"
        );
    }
}
