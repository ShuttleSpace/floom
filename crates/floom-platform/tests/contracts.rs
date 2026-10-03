use floom_platform::{
    Capability, Clipboard, FakePlatform, Ime, Platform, PlatformCapabilities, PlatformEvent,
    WindowConfig,
};
use floom_types::{Point, Size};

#[test]
fn fake_platform_window_lifecycle() {
    let mut p = FakePlatform::new();
    assert!(p.capabilities().supports(Capability::WINDOWING));

    let win = p.create_window(WindowConfig {
        title: "test".into(),
        size: Size::new(640.0, 480.0),
        resizable: true,
    });

    let mut events = Vec::new();
    p.poll_events(&mut events);
    assert!(matches!(
        events.first(),
        Some(PlatformEvent::WindowCreated { window, .. }) if *window == win
    ));

    p.request_frame(win);
    events.clear();
    p.poll_events(&mut events);
    assert!(matches!(
        events.as_slice(),
        [PlatformEvent::RedrawRequested { window }] if *window == win
    ));

    p.window_mut(win).unwrap().set_title("renamed");
    assert_eq!(p.window(win).unwrap().title(), "renamed");
}

#[test]
fn fake_clipboard_and_ime_capability_gated() {
    let mut p = FakePlatform::new();
    assert!(p.set_text("hi"));
    assert_eq!(p.get_text().as_deref(), Some("hi"));

    let mut no_clip = FakePlatform::new().with_capabilities(PlatformCapabilities {
        caps: Capability::WINDOWING,
    });
    assert!(!no_clip.set_text("x"));
    assert!(no_clip.get_text().is_none());

    let win = p.create_window(WindowConfig::default());
    p.set_enabled(win, true);
    p.set_cursor_area(win, Point::ZERO, Size::new(10.0, 10.0));
}

#[test]
fn a11y_stub_capabilities_explicit() {
    let caps = PlatformCapabilities::unsupported_a11y_stub();
    assert!(!caps.supports(Capability::ACCESSIBILITY));
    assert!(caps.supports(Capability::TOUCH));
}

#[test]
fn core_manifest_has_no_os_backends() {
    let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    for forbidden in ["winit", "objc", "android", "napi", "wayland", "x11", "wgpu"] {
        assert!(
            !manifest.to_lowercase().contains(forbidden),
            "floom-platform core must not depend on {forbidden}"
        );
    }
}
