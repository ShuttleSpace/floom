//! Public surface the satellite crate must export after the editor move.

#[test]
fn text_editor_type_is_exported() {
    let name = std::any::type_name::<floom_editor::TextEditor>();
    assert!(
        name.contains("TextEditor"),
        "floom-editor must export TextEditor, got {name}"
    );
}

#[test]
fn editor_type_is_exported() {
    let name = std::any::type_name::<floom_editor::editor::Editor>();
    assert!(
        name.contains("Editor"),
        "floom-editor must export editor::Editor, got {name}"
    );
}
