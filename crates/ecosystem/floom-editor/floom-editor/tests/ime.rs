use floom_app::WidgetHost;
use floom_core::{Key, KeyEvent, KeyPhase, Modifiers, SemanticsRole};
use floom_editor::{TextEditor, text_editor};
use floom_layout::LayoutStyle;
use floom_types::{Event, ImeUpdate, Size};

fn viewport() -> Size {
    Size::new(600.0, 300.0)
}

/// Mounts a copy of `editor` at the end of its document.
fn mount(editor: TextEditor) -> WidgetHost {
    {
        let state = editor.state();
        let len = editor.text().len();
        state.lock().unwrap().set_caret(len);
    }
    let mut host = WidgetHost::new();
    host.set_size(viewport());
    host.mount_root(LayoutStyle::default(), editor);
    host.frame();
    host
}

fn focus_editor(host: &mut WidgetHost) {
    let id = host.root().expect("root");
    host.set_focus(host.node_id(id));
}

#[test]
fn ime_preedit_does_not_mutate_document() {
    let editor = text_editor("hi");
    let mut host = mount(editor.clone());
    focus_editor(&mut host);

    host.dispatch(Event::Ime(ImeUpdate::Preedit {
        text: "あ".to_string(),
        caret: "あ".len(),
        selection: None,
    }));
    host.frame();

    assert_eq!(editor.text(), "hi");
    assert!(
        host.ime_caret_rect().is_some(),
        "IME caret area should be reported while focused"
    );
}

#[test]
fn ime_commit_inserts_text_and_clears_preedit() {
    let editor = text_editor("hi");
    let mut host = mount(editor.clone());
    focus_editor(&mut host);

    host.dispatch(Event::Ime(ImeUpdate::Preedit {
        text: "あ".to_string(),
        caret: "あ".len(),
        selection: None,
    }));
    host.frame();
    assert_eq!(editor.text(), "hi", "preedit is not in the document");
    let preedit_painted = host.scene().iter().any(|(_, node)| match node {
        floom_scene::SceneNode::Text { glyphs, .. } => {
            host.resources()
                .get(*glyphs)
                .is_some_and(|resource| match &resource.kind {
                    floom_scene::ResourceKind::GlyphRun { text, .. } => &**text == "あ",
                    _ => false,
                })
        }
        _ => false,
    });
    assert!(preedit_painted, "preedit is shown while composing");

    // The OS finalizes the composition: empty preedit, then the committed key.
    host.dispatch(Event::Ime(ImeUpdate::Preedit {
        text: String::new(),
        caret: 0,
        selection: None,
    }));
    host.dispatch(Event::Key(KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Char('あ'),
        modifiers: Modifiers::NONE,
    }));
    host.frame();

    assert_eq!(editor.text(), "hiあ");
    let preedit_painted = host.scene().iter().any(|(_, node)| match node {
        floom_scene::SceneNode::Text { glyphs, .. } => {
            host.resources()
                .get(*glyphs)
                .is_some_and(|resource| match &resource.kind {
                    floom_scene::ResourceKind::GlyphRun { text, .. } => &**text == "あ",
                    _ => false,
                })
        }
        _ => false,
    });
    assert!(!preedit_painted, "the committed preedit is no longer drawn");
}

#[test]
fn ime_preedit_is_painted_in_the_composition_ink() {
    let editor = text_editor("hi");
    let mut host = mount(editor.clone());
    focus_editor(&mut host);

    host.dispatch(Event::Ime(ImeUpdate::Preedit {
        text: "あ".to_string(),
        caret: "あ".len(),
        selection: None,
    }));
    host.frame();

    let painted = host.scene().iter().any(|(_, node)| match node {
        floom_scene::SceneNode::Text { glyphs, .. } => {
            host.resources()
                .get(*glyphs)
                .is_some_and(|resource| match &resource.kind {
                    floom_scene::ResourceKind::GlyphRun { text, .. } => &**text == "あ",
                    _ => false,
                })
        }
        _ => false,
    });
    assert!(painted, "the composition text was not painted");
}

#[test]
fn ime_caret_area_follows_the_caret() {
    let editor = text_editor("hello\nworld");
    let mut host = mount(editor.clone());
    focus_editor(&mut host);
    host.dispatch(Event::Ime(ImeUpdate::Preedit {
        text: "あ".to_string(),
        caret: "あ".len(),
        selection: None,
    }));
    host.frame();
    let at_end = host.ime_caret_rect().expect("the area while composing");

    editor.state().lock().unwrap().set_caret(0);
    host.frame();
    let at_start = host
        .ime_caret_rect()
        .expect("the area after the caret moved");

    assert!(
        at_start.y0 < at_end.y0,
        "the popup did not follow the caret to the first line: {at_start:?} then {at_end:?}"
    );
}

#[test]
fn autofocus_requests_focus_on_mount() {
    let editor = text_editor("hi").autofocus(true);
    let host = mount(editor);
    assert!(
        host.focused().is_some(),
        "autofocus editor should receive focus on mount"
    );
}

#[test]
fn autofocus_is_off_by_default() {
    let host = mount(text_editor("hi"));
    assert!(
        host.focused().is_none(),
        "editor took the keyboard without autofocus"
    );
}

#[test]
fn the_editor_announces_an_editable_text_role() {
    let host = mount(text_editor("hello"));
    let id = host.root().expect("root");
    let hint = host
        .semantics_hint(id)
        .expect("the editor publishes a semantics hint");
    assert_eq!(hint.role, SemanticsRole::TextField);
}

#[test]
fn undo_redo_callable_via_public_api_and_updates_the_view() {
    let editor = text_editor("hi");
    let mut host = mount(editor.clone());
    focus_editor(&mut host);

    host.dispatch(Event::Key(KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Char('!'),
        modifiers: Modifiers::NONE,
    }));
    host.frame();
    assert_eq!(editor.text(), "hi!");

    assert!(editor.undo(), "undo should be available");
    host.frame();
    assert_eq!(editor.text(), "hi");

    assert!(editor.redo(), "redo should be available");
    host.frame();
    assert_eq!(editor.text(), "hi!");
}

#[test]
fn set_text_is_callable_via_public_api() {
    let editor = text_editor("hi");
    let mut host = mount(editor.clone());
    editor.set_text("bye".to_string());
    host.frame();
    assert_eq!(editor.text(), "bye");
}

#[test]
fn shared_instance_preserves_document_across_remount() {
    let editor = text_editor("hi");
    let mut host = mount(editor.clone());
    focus_editor(&mut host);
    host.dispatch(Event::Key(KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Char('!'),
        modifiers: Modifiers::NONE,
    }));
    host.frame();
    assert_eq!(editor.text(), "hi!");

    host.mount_root(LayoutStyle::default(), editor.clone());
    host.frame();
    assert_eq!(editor.text(), "hi!");
}
