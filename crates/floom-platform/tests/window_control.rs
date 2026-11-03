use floom_platform::{FakePlatform, Platform, WindowCommand, WindowConfig};

/// The default is a refusal, and it says why: a backend that only has the
/// window it was given (a terminal cell grid, a browser canvas) has no window
/// manager to ask. Silently returning `Ok` would report a change that never
/// happened.
#[test]
fn a_backend_without_a_window_manager_refuses_the_command() {
    let mut platform = FakePlatform::new();
    let id = platform.create_window(WindowConfig::default());
    let window = platform.window_mut(id).unwrap();

    for command in [
        WindowCommand::Focus,
        WindowCommand::Resize(floom_types::Size::new(100.0, 100.0)),
        WindowCommand::AlwaysOnTop(true),
    ] {
        let err = window.control(command).unwrap_err();
        assert!(err.contains("no window manager"), "{command:?}: {err}");
    }
}
