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
use floem_widgets::{Button, Container};

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
