use floom_editor::{Selection, TextDocument, Transaction};

#[test]
fn insert_and_selection() {
    let mut doc = TextDocument::new();
    doc.insert_text("hello");
    assert_eq!(doc.text(), "hello");
    assert_eq!(doc.selection(), Selection::caret(5));

    doc.set_selection(Selection::range(0, 5));
    doc.insert_text("hi");
    assert_eq!(doc.text(), "hi");
    assert_eq!(doc.selection(), Selection::caret(2));
}

#[test]
fn undo_redo_roundtrip() {
    let mut doc = TextDocument::from_text("ab");
    doc.set_selection(Selection::caret(2));
    doc.insert_text("c");
    assert_eq!(doc.text(), "abc");
    assert!(doc.undo());
    assert_eq!(doc.text(), "ab");
    assert!(doc.redo());
    assert_eq!(doc.text(), "abc");
}

#[test]
fn delete_backward_utf8() {
    let mut doc = TextDocument::from_text("a你");
    doc.set_selection(Selection::caret(doc.len()));
    doc.delete_selection_or_backward();
    assert_eq!(doc.text(), "a");
}

#[test]
fn transaction_replace() {
    let mut doc = TextDocument::from_text("abcdef");
    doc.commit(Transaction::replace(2, 4, "XY"));
    assert_eq!(doc.text(), "abXYef");
    assert!(doc.undo());
    assert_eq!(doc.text(), "abcdef");
}

#[test]
fn ime_composition_then_commit() {
    let mut doc = TextDocument::new();
    doc.set_ime_preedit("ni");
    assert_eq!(doc.composition(), Some("ni"));
    assert!(doc.text().is_empty());

    doc.ime_commit(Some("你"));
    assert!(doc.composition().is_none());
    assert_eq!(doc.text(), "你");
}
