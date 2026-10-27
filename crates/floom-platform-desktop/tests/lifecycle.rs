use floom_platform::{
    Capability, Clipboard, Ime, Modifiers, Platform, PlatformEvent, PointerButton, WindowConfig,
};
use floom_platform_desktop::{DesktopInput, DesktopPlatform};
use floom_types::{Point, Size};

#[test]
fn desktop_window_and_input_mapping() {
    let mut p = DesktopPlatform::new();
    // The soft keyboard is real on every desktop target. The *system*
    // clipboard is not yet: this adapter holds the text in process, so the
    // capability is claimed only where the implementation is a real one.
    assert!(p.capabilities().supports(Capability::IME));
    #[cfg(target_os = "macos")]
    assert!(p.capabilities().supports(Capability::CLIPBOARD));
    #[cfg(not(target_os = "macos"))]
    assert!(!p.capabilities().supports(Capability::CLIPBOARD));

    let win = p.create_window(WindowConfig {
        title: "desk".into(),
        size: Size::new(800.0, 600.0),
        resizable: true,
    });

    p.dispatch_input(
        win,
        DesktopInput::PointerButton {
            position: Point::new(10.0, 20.0),
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
    );
    p.dispatch_input(win, DesktopInput::Resized(Size::new(1024.0, 768.0)));
    p.dispatch_input(
        win,
        DesktopInput::ImeCommit {
            text: "你好".into(),
        },
    );

    let mut events = Vec::new();
    p.poll_events(&mut events);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, PlatformEvent::WindowCreated { .. }))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, PlatformEvent::PointerButton { pressed: true, .. }))
    );
    assert!(events.iter().any(|e| matches!(
        e,
        PlatformEvent::WindowResized { size, .. } if *size == Size::new(1024.0, 768.0)
    )));
    assert!(events.iter().any(|e| matches!(
        e,
        PlatformEvent::ImeCommit { text, .. } if text == "你好"
    )));
    assert_eq!(p.window(win).unwrap().size(), Size::new(1024.0, 768.0));
}

#[test]
fn desktop_clipboard_and_ime() {
    let mut p = DesktopPlatform::new();
    assert!(p.set_text("clip"));
    assert_eq!(p.get_text().as_deref(), Some("clip"));

    let win = p.create_window(WindowConfig::default());
    p.set_enabled(win, true);
    assert!(p.ime_is_enabled(win));
}

/// Runtime window control: the reference backend records what a real window
/// manager would have applied, so the parts a test can observe — the size and
/// the state — are asserted here. The winit backend forwards the same commands
/// to the OS instead.
#[test]
fn window_commands_are_applied() {
    use floom_platform::{WindowCommand, WindowState};

    let mut p = DesktopPlatform::new();
    let win = p.create_window(WindowConfig::default());
    let window = p.window_mut(win).unwrap();

    window
        .control(WindowCommand::Resize(Size::new(640.0, 480.0)))
        .unwrap();
    assert_eq!(window.size(), Size::new(640.0, 480.0));

    window.control(WindowCommand::Fullscreen(true)).unwrap();
    assert_eq!(window.state(), WindowState::Fullscreen);
    window.control(WindowCommand::Fullscreen(false)).unwrap();
    assert_eq!(window.state(), WindowState::Normal);

    window.control(WindowCommand::Minimized(true)).unwrap();
    assert_eq!(window.state(), WindowState::Minimized);
    window.control(WindowCommand::Maximized(true)).unwrap();
    assert_eq!(window.state(), WindowState::Maximized);

    for command in [
        WindowCommand::Move(Point::new(10.0, 10.0)),
        WindowCommand::AlwaysOnTop(true),
        WindowCommand::Decorations(false),
        WindowCommand::Visible(true),
        WindowCommand::Focus,
    ] {
        window.control(command).unwrap();
    }
}
