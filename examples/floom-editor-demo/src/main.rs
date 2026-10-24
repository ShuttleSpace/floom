//! Golden example: text editing primitives (Phase 30.1).
//!
//! `floom-editor-demo` is the shared document model behind text fields and editors.
//! Everything interesting about it is headless and worth seeing:
//!
//! 1. **Transactions** — [`Transaction`] is a list of replace ops. Applying one
//!    returns its *inverse*, which is what makes undo exact rather than
//!    best-effort: `apply` changes the text without touching history, `commit`
//!    applies *and* records the inverse.
//! 2. **Selection** — a selection is a range with a head, and replacing the
//!    selection replaces exactly the selected text.
//! 3. **Undo/redo** — the document owns a stack of inverses; an app that wants
//!    its own history uses [`UndoManager`] with the inverses `apply` returns.
//! 4. **IME composition** — pre-edit text is *not* part of the document until
//!    it is committed, which is what stops a half-typed composition from
//!    landing in undo history.
//!
//! Run with `cargo run -p floom-editor-demo`.

use floom_editor::{Selection, TextDocument, Transaction, UndoManager};

fn main() {
    // --- 1. transactions and their inverse ---------------------------------
    let mut doc = TextDocument::from_text("hello world");
    println!("start            : {:?}", doc.text());

    // `apply` changes the text and hands back the exact inverse. No history.
    let inverse = doc.apply(&Transaction::insert(5, ","));
    println!("after insert     : {:?}", doc.text());
    assert_eq!(doc.text(), "hello, world");

    doc.apply(&inverse);
    println!("inverse applied  : {:?}", doc.text());
    assert_eq!(doc.text(), "hello world", "the inverse is exact");

    // `commit` applies *and* records, so the document can undo it itself.
    doc.commit(Transaction::insert(5, ","));
    println!("committed        : {:?}", doc.text());
    assert_eq!(doc.text(), "hello, world");

    // --- 2. selection ------------------------------------------------------
    doc.set_selection(Selection::range(0, 5));
    let sel = doc.selection();
    println!("selection        : {}..{}", sel.start(), sel.end());
    assert_eq!((sel.start(), sel.end()), (0, 5));

    // Replacing the selection replaces exactly the selected text.
    doc.commit(Transaction::replace(sel.start(), sel.end(), "hi"));
    println!("replaced         : {:?}", doc.text());
    assert_eq!(doc.text(), "hi, world");

    // --- 3. undo / redo on the document ------------------------------------
    assert!(doc.undo(), "the last commit is undoable");
    println!("\ndocument undo    : {:?}", doc.text());
    assert_eq!(doc.text(), "hello, world");
    assert!(doc.redo(), "and redoable");
    println!("document redo    : {:?}", doc.text());
    assert_eq!(doc.text(), "hi, world");

    // Typing goes through the selection, so it is undoable too.
    doc.set_selection(Selection::caret(doc.len()));
    doc.insert_text("!");
    println!("typed            : {:?}", doc.text());
    assert_eq!(doc.text(), "hi, world!");
    assert!(doc.undo());
    assert_eq!(doc.text(), "hi, world");

    // An app-owned history: `apply` gives the inverse, the manager replays it.
    let mut history = UndoManager::new();
    let edit = Transaction::insert(doc.len(), "?");
    let inverse = doc.apply(&edit);
    history.push(inverse);
    println!("app-managed edit : {:?}", doc.text());
    assert_eq!(doc.text(), "hi, world?");
    assert!(history.can_undo());
    assert!(history.undo(&mut doc));
    println!("app-managed undo : {:?}", doc.text());
    assert_eq!(doc.text(), "hi, world");
    assert!(!history.can_undo(), "the app history is now empty");
    assert!(history.can_redo());
    assert!(history.redo(&mut doc));
    assert_eq!(doc.text(), "hi, world?");

    // --- 4. IME composition -------------------------------------------------
    // A pre-edit is visible to the renderer but must not enter the document or
    // the undo stack until it is committed.
    let before = doc.text();
    doc.set_ime_preedit("にほん");
    println!("\nime preedit      : {:?}", doc.composition());
    assert_eq!(doc.composition(), Some("にほん"));
    assert_eq!(
        doc.text(),
        before,
        "a pre-edit is not part of the document yet"
    );

    doc.ime_commit(Some("にほん"));
    println!("ime committed    : {:?}", doc.text());
    assert_eq!(doc.text(), "hi, world?にほん");
    assert!(
        doc.composition().is_none(),
        "committing clears the composition"
    );

    println!("\neditor golden example: OK");
}
