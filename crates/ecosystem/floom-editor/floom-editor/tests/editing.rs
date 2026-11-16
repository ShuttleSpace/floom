//! Basic document editing coverage, carried over from the removed
//! `widgets/floom-editor/tests/editing.rs` suite onto the satellite API.

use std::sync::Arc;

use floom_editor::{
    Document, TextDocument,
    core::{
        cursor::CursorAffinity,
        selection::{SelRegion, Selection},
    },
};

fn caret(offset: usize) -> Selection {
    Selection::caret(offset, CursorAffinity::Forward)
}

#[test]
fn insert_and_undo_round_trip() {
    let doc: Arc<dyn Document> = Arc::new(TextDocument::new("ab"));
    doc.edit(&caret(2), "c");
    assert_eq!(doc.text().to_string(), "abc");
    assert!(doc.undo());
    assert_eq!(doc.text().to_string(), "ab");
    assert!(doc.redo());
    assert_eq!(doc.text().to_string(), "abc");
}

#[test]
fn replace_a_selection_is_one_undoable_edit() {
    let doc: Arc<dyn Document> = Arc::new(TextDocument::new("abcdef"));
    let mut selection = Selection::new();
    selection.add_region(SelRegion::new(2, 4, CursorAffinity::Forward, None));
    doc.edit(&selection, "XY");
    assert_eq!(doc.text().to_string(), "abXYef");
    assert!(doc.undo());
    assert_eq!(doc.text().to_string(), "abcdef");
}

#[test]
fn set_text_replaces_the_whole_document() {
    let doc: Arc<dyn Document> = Arc::new(TextDocument::new("hello"));
    doc.set_text("world".into());
    assert_eq!(doc.text().to_string(), "world");
}

#[test]
fn utf8_edits_keep_the_rope_consistent() {
    let doc: Arc<dyn Document> = Arc::new(TextDocument::new("a你"));
    doc.edit(&caret("a".len() + "你".len()), "！");
    assert_eq!(doc.text().to_string(), "a你！");
    assert!(doc.undo());
    assert_eq!(doc.text().to_string(), "a你");
}
