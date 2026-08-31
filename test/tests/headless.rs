//! `HeadlessEmbedder` backend proof-of-concept tests.
//!
//! These confirm the headless backend can stand in for a real platform
//! backend: it owns a window id, drives layout without any real window or
//! paint surface, and records the commands Floem asks the backend to run via
//! its [`floem::embedder::CommandSink`]. This is the canonical harness for
//! every future backend integration test.

use floem::embedder::{
    EmbedderCommand, EmbedderWindowId, HeadlessEmbedder, ImeHint, ImePurpose, ImeState,
};
use floem::headless::TestRoot;
use floem::prelude::*;

fn make_embedder() -> HeadlessEmbedder {
    let root = TestRoot::new();
    let view = Empty::new().style(|s| s.size(200.0, 100.0));
    HeadlessEmbedder::new(root, view)
}

#[test]
fn window_id_is_stable() {
    let embedder = make_embedder();
    let first: EmbedderWindowId = embedder.window_id();
    let second: EmbedderWindowId = embedder.window_id();
    assert_eq!(
        first, second,
        "window_id must be stable for the window's lifetime"
    );
}

#[test]
fn request_redraw_records_command() {
    let mut embedder = make_embedder();
    let id = embedder.window_id();

    embedder.request_redraw_for_test();
    let commands = embedder.take_commands();

    assert_eq!(
        commands,
        vec![EmbedderCommand::RequestRedraw { window_id: id }]
    );
}

#[test]
fn set_ime_state_records_command() {
    let mut embedder = make_embedder();
    let id = embedder.window_id();

    let state = ImeState {
        enabled: true,
        candidate_rect: None,
        purpose: ImePurpose::Number,
        hint: ImeHint::Default,
        secure: false,
        multiline: false,
        surrounding_text: None,
    };
    embedder.set_ime_state_for_test(state.clone());
    let commands = embedder.take_commands();

    assert_eq!(
        commands,
        vec![EmbedderCommand::SetImeState {
            window_id: id,
            state
        }]
    );
}

#[test]
fn command_sink_drains_all_and_clears() {
    let mut embedder = make_embedder();
    let id = embedder.window_id();

    embedder.request_redraw_for_test();
    embedder.request_redraw_for_test();
    let commands = embedder.take_commands();

    assert_eq!(commands.len(), 2);
    assert!(
        commands
            .iter()
            .all(|c| matches!(c, EmbedderCommand::RequestRedraw { window_id } if *window_id == id))
    );
    // Sink is empty after draining.
    assert!(embedder.take_commands().is_empty());
}

#[test]
fn drives_layout_without_real_window() {
    let root = TestRoot::new();
    // The root ViewId is the WindowView (window-sized); the styled Empty is its
    // child, so we assert on the child's laid-out rect.
    let child = Empty::new().style(|s| s.size(200.0, 100.0));
    let child_id = child.view_id();
    let view = child;
    let mut embedder = HeadlessEmbedder::new(root, view);
    embedder.harness_mut().rebuild();

    let rect = embedder.harness().get_layout_rect(child_id);

    assert!(
        (rect.width() - 200.0).abs() < 1.0,
        "headless layout should size the child to its style width (got {})",
        rect.width()
    );
    assert!(
        (rect.height() - 100.0).abs() < 1.0,
        "headless layout should size the child to its style height (got {})",
        rect.height()
    );
}

#[test]
fn processes_pointer_events_without_window() {
    let mut embedder = make_embedder();

    // A tap should not panic and should leave no lingering pointer capture.
    embedder.touch_down_for_test(20.0, 20.0);
    assert!(embedder.has_pointer_capture_for_test());
    embedder.touch_up_for_test(20.0, 20.0);
    assert!(!embedder.has_pointer_capture_for_test());
}

#[test]
fn headless_backend_is_capability_aware() {
    // The headless backend only advertises headless operation; a real mobile
    // backend would advertise ime / safe_area / virtual_keyboard instead.
    use floem::embedder::BackendCapabilities;
    let caps = BackendCapabilities::headless();
    assert!(caps.headless);
    assert!(!caps.ime);
    assert!(!caps.gpu_surface);
}
