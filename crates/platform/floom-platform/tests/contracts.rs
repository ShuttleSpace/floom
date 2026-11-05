// SPDX-License-Identifier: MIT
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
    // The clipboard is asked for explicitly: the default host platform does
    // not claim one, because its clipboard is in process.
    let mut p = FakePlatform::new().with_capabilities(PlatformCapabilities {
        caps: Capability::WINDOWING | Capability::IME | Capability::CLIPBOARD,
    });
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
fn a_stub_claims_no_soft_keyboard() {
    // A stand-in's IME is an in-memory map. Claiming the capability would say
    // that text typed on a phone reaches the framework, and the real backend
    // would have to remember to take the claim back — which is the drift the
    // capability set exists to prevent.
    let caps = PlatformCapabilities::unsupported_a11y_stub();
    assert!(
        !caps.supports(Capability::IME),
        "a stand-in has no keyboard; the real backend claims the IME"
    );
}

#[test]
fn a_capability_with_no_implementation_has_no_bit() {
    // The enforcement is the bit set itself: a bit that nothing implements
    // cannot be claimed, because there is nothing to set. These three were
    // here once, with no implementation behind any of them, and a platform
    // could claim them and be believed.
    //
    // The check is a compile-time one — the names below do not exist — so this
    // test is the *record* of the removal rather than the mechanism.
    //
    //   Capability::OPEN_URL    — no implementation anywhere
    //   Capability::FILE_DIALOG — no implementation anywhere
    //   Capability::DRM_KMS     — a cargo feature with no DRM code behind it
    //
    // A platform that grows one of these adds the bit back, and the table in
    // `capabilities.rs` says which implementation is behind it.
    let caps = PlatformCapabilities::desktop_default();
    assert!(caps.supports(Capability::WINDOWING));
    assert!(caps.supports(Capability::SOFTWARE_RENDER));
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
