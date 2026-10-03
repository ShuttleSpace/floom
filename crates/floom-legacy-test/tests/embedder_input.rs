//! Embedder input translation tests.
//!
//! These verify that neutral [`EmbedderEvent`] values fed through
//! [`HeadlessEmbedder`] drive the real Floom runtime the same way native
//! events would: focus navigation, text insertion, IME composition commit,
//! and pointer (touch) capture. This is the contract every backend
//! (Android, iOS, HarmonyOS, winit) must satisfy.

use floom::embedder::{
    EmbedderEvent, EmbedderWindowId, FocusEvent, ImeEvent, Theme, WindowLifecycleEvent,
};
use floom::headless::TestRoot;
use floom::prelude::*;
use floom::views::TextInput;
use ui_events::keyboard::{Code, Key, KeyState, KeyboardEvent, Location, Modifiers, NamedKey};

fn tab_down() -> KeyboardEvent {
    KeyboardEvent {
        key: Key::Named(NamedKey::Tab),
        code: Code::Tab,
        modifiers: Modifiers::default(),
        location: Location::Standard,
        is_composing: false,
        repeat: false,
        state: KeyState::Down,
    }
}

fn char_down(ch: &str) -> KeyboardEvent {
    KeyboardEvent {
        key: Key::Character(ch.to_string()),
        code: Code::KeyA,
        modifiers: Modifiers::default(),
        location: Location::Standard,
        is_composing: false,
        repeat: false,
        state: KeyState::Down,
    }
}

#[test]
fn keyboard_tab_event_focuses_first_navigable() {
    let root = TestRoot::new();
    let a = Empty::new().style(|s| s.size(80.0, 40.0).keyboard_navigable());
    let a_id = a.view_id();
    let b = Empty::new().style(|s| s.size(80.0, 40.0).keyboard_navigable());
    let b_id = b.view_id();
    let view = Stack::horizontal((a, b)).style(|s| s.size(160.0, 40.0));
    let mut embedder = floom::embedder::HeadlessEmbedder::new(root, view);

    embedder.send(EmbedderEvent::Keyboard {
        window_id: embedder.window_id(),
        event: tab_down(),
        synthetic: false,
    });

    assert!(
        embedder.harness().is_focused(a_id),
        "Tab through the embedder should focus the first navigable view"
    );
    assert!(!embedder.harness().is_focused(b_id));
}

#[test]
fn keyboard_character_event_inserts_text_into_focused_input() {
    let text = RwSignal::new(String::new());
    let root = TestRoot::new();
    let view = TextInput::new(text).style(|s| s.size(200.0, 40.0));
    let mut embedder = floom::embedder::HeadlessEmbedder::new(root, view);

    // Focus the text input by tapping it.
    embedder.touch_down_for_test(20.0, 20.0);
    embedder.touch_up_for_test(20.0, 20.0);

    embedder.send(EmbedderEvent::Keyboard {
        window_id: embedder.window_id(),
        event: char_down("a"),
        synthetic: false,
    });

    assert_eq!(
        text.get(),
        "a",
        "A character key routed through the embedder should insert into the focused input"
    );
}

#[test]
fn synthetic_keyboard_events_are_ignored() {
    let text = RwSignal::new(String::new());
    let root = TestRoot::new();
    let view = TextInput::new(text).style(|s| s.size(200.0, 40.0));
    let mut embedder = floom::embedder::HeadlessEmbedder::new(root, view);

    embedder.touch_down_for_test(20.0, 20.0);
    embedder.touch_up_for_test(20.0, 20.0);

    embedder.send(EmbedderEvent::Keyboard {
        window_id: embedder.window_id(),
        event: char_down("z"),
        synthetic: true,
    });

    assert_eq!(
        text.get(),
        "",
        "Synthetic key events must be ignored by the embedder boundary"
    );
}

#[test]
fn ime_commit_event_updates_focused_input() {
    let text = RwSignal::new(String::new());
    let root = TestRoot::new();
    let view = TextInput::new(text).style(|s| s.size(200.0, 40.0));
    let mut embedder = floom::embedder::HeadlessEmbedder::new(root, view);

    // Focus the input so IME commits are accepted.
    embedder.touch_down_for_test(20.0, 20.0);
    embedder.touch_up_for_test(20.0, 20.0);

    embedder.send(EmbedderEvent::Ime {
        window_id: embedder.window_id(),
        event: ImeEvent::CommitText("hello".to_string()),
    });

    assert_eq!(
        text.get(),
        "hello",
        "An IME CommitText routed through the embedder should commit into the focused input"
    );
}

#[test]
fn ime_preedit_event_does_not_panic() {
    let text = RwSignal::new(String::new());
    let root = TestRoot::new();
    let view = TextInput::new(text).style(|s| s.size(200.0, 40.0));
    let mut embedder = floom::embedder::HeadlessEmbedder::new(root, view);

    embedder.touch_down_for_test(20.0, 20.0);
    embedder.touch_up_for_test(20.0, 20.0);

    embedder.send(EmbedderEvent::Ime {
        window_id: embedder.window_id(),
        event: ImeEvent::PreeditChanged {
            text: "ni".to_string(),
            cursor_byte_range: Some((0, 2)),
        },
    });
    embedder.send(EmbedderEvent::Ime {
        window_id: embedder.window_id(),
        event: ImeEvent::EnabledChanged(true),
    });

    // Preedit alone should not have committed text.
    assert_eq!(text.get(), "");
}

#[test]
fn touch_pointer_capture_lifecycle() {
    let root = TestRoot::new();
    let view = Empty::new().style(|s| s.size(200.0, 200.0));
    let mut embedder = floom::embedder::HeadlessEmbedder::new(root, view);

    embedder.touch_down_for_test(50.0, 50.0);
    assert!(
        embedder.has_pointer_capture_for_test(),
        "A touch down should establish implicit pointer capture"
    );

    embedder.touch_up_for_test(50.0, 50.0);
    assert!(
        !embedder.has_pointer_capture_for_test(),
        "A touch up should release pointer capture"
    );
}

#[test]
fn focus_event_routes_without_emitting_commands() {
    let root = TestRoot::new();
    let view = Empty::new().style(|s| s.size(100.0, 100.0));
    let mut embedder = floom::embedder::HeadlessEmbedder::new(root, view);

    embedder.send(EmbedderEvent::Focus {
        window_id: embedder.window_id(),
        event: FocusEvent::WindowFocused(true),
    });
    embedder.send(EmbedderEvent::Focus {
        window_id: embedder.window_id(),
        event: FocusEvent::WindowFocused(false),
    });

    // Focus changes are applied to the window; they do not emit embedder commands.
    assert!(embedder.take_commands().is_empty());
}

#[test]
fn window_theme_and_resize_events_route() {
    let root = TestRoot::new();
    let view = Empty::new().style(|s| s.size(100.0, 100.0));
    let mut embedder = floom::embedder::HeadlessEmbedder::new(root, view);

    embedder.send(EmbedderEvent::Window {
        window_id: embedder.window_id(),
        event: WindowLifecycleEvent::ThemeChanged(Theme::Dark),
    });
    embedder.send(EmbedderEvent::Window {
        window_id: embedder.window_id(),
        event: WindowLifecycleEvent::Resized(floom::peniko::kurbo::Size::new(320.0, 480.0)),
    });

    // No command emission expected for these lifecycle events yet.
    assert!(embedder.take_commands().is_empty());
}

#[test]
fn close_requested_event_routes_without_panic() {
    let root = TestRoot::new();
    let view = Empty::new().style(|s| s.size(100.0, 100.0));
    let mut embedder = floom::embedder::HeadlessEmbedder::new(root, view);

    embedder.send(EmbedderEvent::Window {
        window_id: embedder.window_id(),
        event: WindowLifecycleEvent::CloseRequested,
    });

    assert!(embedder.take_commands().is_empty());
}

#[test]
fn redraw_requested_event_routes_without_panic() {
    let root = TestRoot::new();
    let view = Empty::new().style(|s| s.size(100.0, 100.0));
    let mut embedder = floom::embedder::HeadlessEmbedder::new(root, view);
    let id: EmbedderWindowId = embedder.window_id();

    embedder.send(EmbedderEvent::RedrawRequested { window_id: id });
    embedder.send(EmbedderEvent::RedrawRequested { window_id: id });
}
