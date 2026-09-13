//! End-to-end runner tests: FakePlatform → AppRunner → SoftwarePresenter.
//!
//! These exercise the full wiring path — platform events in, CPU pixels out —
//! without a window server or a GPU. The software presenter doubles as the
//! assertion surface: its RGBA buffer is inspectable pixel by pixel.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use floem_app::{AppRunner, RunStats, WidgetHost};
use floem_layout::LayoutStyle;
use floem_platform::{FakePlatform, KeyCode, Platform, PlatformEvent, WindowConfig};
use floem_renderer_software::SoftwarePresenter;
use floem_types::{Color, Point, Size};
use floem_widgets::{Button, Container, Stack};

/// A tiny window (64×48 logical, scale 1) so pixel arithmetic stays trivial.
fn small_window() -> WindowConfig {
    WindowConfig {
        title: "runner-test".into(),
        size: Size::new(64.0, 48.0),
        resizable: true,
    }
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
    host.mount_root(
        LayoutStyle::default(),
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
        Container::new().background(floem_types::Color::WHITE),
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
        Container::new().background(floem_types::Color::WHITE),
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
        modifiers: floem_types::Modifiers::NONE,
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

// ---------------------------------------------------------------------------
// Error boundary integration
// ---------------------------------------------------------------------------

mod boundary {
    use super::*;
    use floem_error::ErrorBoundary;

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
            Stack::new(floem_core::Axis::Vertical)
                .push_layout(
                    floem_core::LayoutRequest::new().height(24.0).grow(1.0),
                    || {
                        Box::new(Button::new("boom").on_click(|| {
                            panic!("view blew up");
                        })) as Box<dyn floem_core::WidgetAny>
                    },
                )
                .push_layout(
                    floem_core::LayoutRequest::new().height(24.0).grow(1.0),
                    move || {
                        let flag = worked_flag.clone();
                        Box::new(Button::new("ok").on_click(move || {
                            flag.store(true, Ordering::SeqCst);
                        })) as Box<dyn floem_core::WidgetAny>
                    },
                ),
        );
        let boundary = ErrorBoundary::new();
        let mut runner =
            AppRunner::new(host, SoftwarePresenter::new()).with_error_boundary(boundary.clone());

        let win = platform.create_window(small_window());
        settle(&mut platform, &mut runner);

        // Click the top half ("boom").
        platform.inject_pointer_click(win, Point::new(32.0, 12.0));
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
        platform.inject_pointer_click(win, Point::new(32.0, 36.0));
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
            LayoutStyle::default(),
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
    use floem_services::Scope;
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
