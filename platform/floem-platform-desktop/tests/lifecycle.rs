use floem_platform::{
    Capability, Clipboard, Ime, Modifiers, Platform, PlatformEvent, PointerButton, WindowConfig,
};
use floem_platform_desktop::{DesktopInput, DesktopPlatform};
use floem_types::{Point, Size};

#[test]
fn desktop_window_and_input_mapping() {
    let mut p = DesktopPlatform::new();
    assert!(p.capabilities().supports(Capability::CLIPBOARD));
    assert!(p.capabilities().supports(Capability::IME));

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
