
use floom_app::{AppRunner, RunStats, WidgetHost};
use floom_layout::{AbsoluteInset, LayoutStyle, Length, Position};
use floom_native::{FakeNativeProvider, NativeCommandLog, NativeViewKind};
use floom_platform::{FakePlatform, KeyCode, Platform, PlatformEvent, WindowConfig};
use floom_renderer_software::SoftwarePresenter;
use floom_testkit::TestApp;
use floom_types::{Point, Size};
use floom_ui_composites::TextField;
use floom_widgets::primitives::Container;
use floom_widgets::NativeSurface;

// --- scaffolding -----------------------------------------------------------

/// A 400x300 window whose root is an empty box, so nodes can be placed at exact
/// coordinates. Participants live and die by their rects, and a flow layout
/// would make every assertion here an assertion about the layout engine.
fn empty_app() -> TestApp {
    TestApp::new(Size::new(400.0, 300.0), Container::new())
}

/// A provider plus the handle a test keeps to ask what it was told. The host
/// takes the provider by value, so the handle is the only way back.
fn provider() -> (FakeNativeProvider, NativeCommandLog) {
    let mut provider = FakeNativeProvider::new();
    let log = provider.watch();
    (provider, log)
}

/// A box at `rect`, taken out of the parent's flow so two nodes can overlap.
fn at(x: f32, y: f32, width: f32, height: f32) -> LayoutStyle {
    LayoutStyle {
        position: Position::Absolute,
        inset: AbsoluteInset {
            left: Some(x),
            top: Some(y),
            right: None,
            bottom: None,
        },
        width: Some(Length::Px(width)),
        height: Some(Length::Px(height)),
        ..LayoutStyle::default()
    }
}

/// A tiny window (64x48 logical, scale 1) so pixel arithmetic stays trivial.
fn small_window() -> WindowConfig {
    WindowConfig {
        title: "runner-test".into(),
        size: Size::new(64.0, 48.0),
        resizable: true,
    }
}

fn settle(platform: &mut FakePlatform, runner: &mut AppRunner<SoftwarePresenter>) -> RunStats {
    let mut stats = RunStats::default();
    for _ in 0..4 {
        stats = runner.pump(platform);
    }
    stats
}

// --- typing reaches the focused field --------------------------------------

#[test]
fn typing_reaches_the_focused_text_field() {
    let mut app = TestApp::new(Size::new(200.0, 32.0), TextField::new().placeholder("name"));
    let field = app.root();
    assert_eq!(app.widget::<TextField>(field).text(), "");

    // Nothing holds focus yet, so the character has nowhere to go.
    assert!(!app.key(KeyCode::from_char('a')));
    assert_eq!(app.widget::<TextField>(field).text(), "");

    // Clicking focuses it...
    app.press(Point::new(100.0, 16.0));
    app.release(Point::new(100.0, 16.0));
    assert!(app.is_focused(field), "pressing a text field focuses it");

    // ...and now committed text lands in the value, caret advancing with it.
    assert!(app.commit("hi"), "typing repainted the field");
    assert_eq!(app.widget::<TextField>(field).text(), "hi");
    assert_eq!(app.widget::<TextField>(field).caret(), 2);

    // Backspace removes the character before the caret.
    assert!(app.key(KeyCode::BACKSPACE));
    assert_eq!(app.widget::<TextField>(field).text(), "h");
    assert_eq!(app.widget::<TextField>(field).caret(), 1);

    // A plain character key arrives as text input too, not as a shortcut.
    assert!(app.key(KeyCode::from_char('i')));
    assert_eq!(app.widget::<TextField>(field).text(), "hi");

    app.settle();
}

// --- the runner routes an OS composition to the focused field --------------

/// The IME's half of the round trip: the platform turns keystrokes into
/// `ImeCommit`/`ImePreedit`, the runner routes them to
/// the focused field, which owns the `floom_core::ime` state machine and
/// renders the marked text inline, and hands the OS the caret rect to anchor
/// its popup -- then clears that area when the composition commits.
#[test]
fn ime_preedit_updates_focused_field_then_commit_finalizes() {
    let mut platform = FakePlatform::new();
    let mut host = WidgetHost::new();
    // Pinned to the window: this test exercises IME routing, not the field's
    // intrinsic (content-fitting) size, so clicks land squarely on it.
    let field = host.mount_root(
        LayoutStyle {
            width: Some(Length::Px(64.0)),
            height: Some(Length::Px(48.0)),
            ..LayoutStyle::default()
        },
        TextField::new(),
    );
    let mut runner = AppRunner::new(host, SoftwarePresenter::new());

    let win = platform.create_window(small_window());
    settle(&mut platform, &mut runner);

    // Focus the field by clicking it (the field is the root, so it fills the
    // whole 64x48 viewport). IME composition is meaningless without a target.
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

// --- a native participant takes the candidate window away ------------------

/// Focusing a participant blurs the framework element and stops the framework
/// publishing an IME caret: two owners of the candidate window makes it jump
/// between them (decision D4).
#[test]
fn focusing_a_participant_blurs_the_framework_and_masks_the_caret() {
    let mut app = empty_app();
    let root = app.root();
    let field = app
        .host_mut()
        .mount(root, at(0.0, 0.0, 200.0, 40.0), TextField::new());
    let surface = app.host_mut().mount(
        root,
        at(0.0, 60.0, 200.0, 40.0),
        NativeSurface::new(NativeViewKind::TextInput).focusable(true),
    );
    let (fake, log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::TextInput])
        .unwrap();
    app.frame();

    // The framework composes, and publishes a caret for the OS popup.
    app.focus(field);
    app.preedit("にほん");
    assert!(
        app.host().ime_caret_rect().is_some(),
        "a composing text field publishes its caret"
    );

    // The native text field takes focus.
    app.focus(surface);
    assert_eq!(log.focuses(), 1, "the provider was told to focus");
    assert!(
        app.host().focused().is_none(),
        "the framework element let go, rather than sharing"
    );
    assert!(app.host().native_focused().is_some());
    assert!(
        app.host().ime_caret_rect().is_none(),
        "and stopped publishing a caret while the participant owns the popup"
    );
}

/// Handing focus back restores both halves: the framework element it displaced
/// gets the keyboard again, and the caret comes back with it.
#[test]
fn handing_focus_back_restores_the_framework() {
    let mut app = empty_app();
    let root = app.root();
    let field = app
        .host_mut()
        .mount(root, at(0.0, 0.0, 200.0, 40.0), TextField::new());
    let surface = app.host_mut().mount(
        root,
        at(0.0, 60.0, 200.0, 40.0),
        NativeSurface::new(NativeViewKind::TextInput).focusable(true),
    );
    let (fake, log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::TextInput])
        .unwrap();
    app.frame();

    app.focus(field);
    app.preedit("にほん");
    app.focus(surface);
    assert!(app.host().focused().is_none());

    app.focus(field);
    assert_eq!(log.blurs(), 1, "the provider was told to give focus up");
    assert!(
        app.host().focused().is_some(),
        "focus came back to the framework element it displaced"
    );
    assert!(app.host().ime_caret_rect().is_some(), "caret and all");
    assert_eq!(app.host().focused(), Some(app.host().node_id(field)));
}
