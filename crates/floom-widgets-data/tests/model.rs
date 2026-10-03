//! Headless data-model tests: cell values, sorting, selection, edit history.
//!
//! Nothing here touches a renderer or a window. The models are pure data, so
//! what is under test is exactly the arithmetic the widgets later read — which
//! is also why these are the tests that stay green when a paint path changes.

use std::cmp::Ordering;

use floom_widgets_data::model::{
    CellEdit, CellValue, Column, ColumnId, ColumnKind, EditHistory, EditSession, RowId,
    SelectionMode, SelectionModel, SortDirection, SortSpec, TableModel, VecTable, sorted_order,
};

fn ids(n: u64) -> Vec<RowId> {
    (0..n).map(RowId).collect()
}

fn table(rows: &[&[&str]]) -> VecTable {
    VecTable::from_rows(
        rows.iter()
            .map(|r| r.iter().map(|c| CellValue::Text((*c).into())).collect())
            .collect::<Vec<Vec<CellValue>>>(),
    )
}

// --- cell values -----------------------------------------------------------

#[test]
fn cell_display_is_stable_across_variants() {
    assert_eq!(CellValue::Empty.display(), "");
    assert_eq!(CellValue::Text("hi".into()).display(), "hi");
    assert_eq!(CellValue::Bool(true).display(), "true");
    assert_eq!(CellValue::Bool(false).display(), "false");
    // Whole numbers print without a trailing `.0`; fractions keep their digits.
    assert_eq!(CellValue::Number(42.0).display(), "42");
    assert_eq!(CellValue::Number(-7.0).display(), "-7");
    assert_eq!(CellValue::Number(1.5).display(), "1.5");
}

#[test]
fn cell_order_is_total_and_ranks_variants() {
    // Ranks are Empty < Bool < Number < Text so a heterogeneous column still
    // sorts deterministically instead of by insertion order.
    assert_eq!(CellValue::Empty.compare(&CellValue::Empty), Ordering::Equal);
    assert_eq!(
        CellValue::Empty.compare(&CellValue::Bool(false)),
        Ordering::Less
    );
    assert_eq!(
        CellValue::Bool(true).compare(&CellValue::Number(f64::NAN)),
        Ordering::Less
    );
    assert_eq!(
        CellValue::Number(1.0).compare(&CellValue::Text(String::new())),
        Ordering::Less
    );
    assert_eq!(
        CellValue::Number(1.0).compare(&CellValue::Number(2.0)),
        Ordering::Less
    );
    assert_eq!(
        CellValue::Text("a".into()).compare(&CellValue::Text("b".into())),
        Ordering::Less
    );
    // NaN is ordered equal to itself rather than panicking: a sort must never
    // be able to fail on data it did not author.
    assert_eq!(
        CellValue::Number(f64::NAN).compare(&CellValue::Number(f64::NAN)),
        Ordering::Equal
    );
}

#[test]
fn number_parsing_rejects_text_and_accepts_signs() {
    assert_eq!(
        CellValue::parse(ColumnKind::Number, " 12 "),
        Some(CellValue::Number(12.0))
    );
    assert_eq!(
        CellValue::parse(ColumnKind::Number, "-1.5"),
        Some(CellValue::Number(-1.5))
    );
    assert_eq!(CellValue::parse(ColumnKind::Number, "abc"), None);
    assert_eq!(CellValue::parse(ColumnKind::Number, ""), None);
    // A text column accepts anything, including the empty string.
    assert_eq!(
        CellValue::parse(ColumnKind::Text, ""),
        Some(CellValue::Text(String::new()))
    );
    // Booleans accept the spellings a user would plausibly type.
    for s in ["true", "TRUE", "1", "yes", "on"] {
        assert_eq!(
            CellValue::parse(ColumnKind::Bool, s),
            Some(CellValue::Bool(true)),
            "{s}"
        );
    }
    for s in ["false", "0", "no", "off"] {
        assert_eq!(
            CellValue::parse(ColumnKind::Bool, s),
            Some(CellValue::Bool(false)),
            "{s}"
        );
    }
    assert_eq!(CellValue::parse(ColumnKind::Bool, "maybe"), None);
}

// --- sorting ---------------------------------------------------------------

#[test]
fn sort_click_cycles_ascending_descending_cleared() {
    let c = ColumnId(1);
    assert_eq!(
        SortSpec::ascending(c).direction,
        SortDirection::Ascending,
        "the constructor must set the direction, not just the column"
    );
    assert_eq!(SortSpec::toggled(None, c), Some(SortSpec::ascending(c)));
    assert_eq!(
        SortSpec::toggled(Some(SortSpec::ascending(c)), c),
        Some(SortSpec::descending(c))
    );
    // Third click clears the sort — the display order must stay reachable.
    assert_eq!(SortSpec::toggled(Some(SortSpec::descending(c)), c), None);
    // Clicking a *different* column starts over at ascending on that column.
    assert_eq!(
        SortSpec::toggled(Some(SortSpec::descending(c)), ColumnId(0)),
        Some(SortSpec::ascending(ColumnId(0)))
    );
}

#[test]
fn sort_is_stable_so_equal_keys_keep_model_order() {
    // Two rows tie on column 0; their relative order must not flip.
    let t = table(&[&["b", "1"], &["a", "2"], &["b", "3"], &["a", "4"]]);
    let order = sorted_order(&t, Some(SortSpec::ascending(ColumnId(0))));
    assert_eq!(order, vec![1, 3, 0, 2]);
}

#[test]
fn sort_orders_numbers_numerically_not_lexically() {
    let t = VecTable::from_rows(vec![
        vec![CellValue::Number(10.0)],
        vec![CellValue::Number(9.0)],
        vec![CellValue::Number(100.0)],
    ]);
    let order = sorted_order(&t, Some(SortSpec::ascending(ColumnId(0))));
    assert_eq!(order, vec![1, 0, 2]);
    let order = sorted_order(&t, Some(SortSpec::descending(ColumnId(0))));
    assert_eq!(order, vec![2, 0, 1]);
}

#[test]
fn no_sort_is_identity_order() {
    let t = table(&[&["c"], &["a"], &["b"]]);
    assert_eq!(sorted_order(&t, None), vec![0, 1, 2]);
}

#[test]
fn sorting_an_empty_model_is_empty() {
    let t = VecTable::new();
    assert_eq!(
        sorted_order(&t, Some(SortSpec::ascending(ColumnId(0)))),
        Vec::<usize>::new()
    );
}

// --- selection -------------------------------------------------------------

#[test]
fn single_mode_click_replaces_the_selection() {
    let rows = ids(5);
    let mut sel = SelectionModel::new(SelectionMode::Single);
    assert!(sel.move_to(2, &rows));
    assert_eq!(sel.cursor(), Some(RowId(2)));
    assert_eq!(sel.selected_ids(), vec![RowId(2)]);

    assert!(sel.move_to(4, &rows));
    assert_eq!(
        sel.selected_ids(),
        vec![RowId(4)],
        "single mode keeps one row"
    );
    assert_eq!(sel.selected_count(), 1);
}

#[test]
fn keyboard_move_replaces_selection_and_extends_only_with_shift() {
    let rows = ids(5);
    let mut sel = SelectionModel::new(SelectionMode::Multiple);
    sel.move_to(1, &rows);

    sel.navigate(1, &rows, false);
    assert_eq!(sel.cursor(), Some(RowId(2)));
    assert_eq!(sel.selected_ids(), vec![RowId(2)], "plain move replaces");

    // Anchor is row 2; extending to row 4 selects the whole range.
    sel.navigate(2, &rows, true);
    assert_eq!(sel.cursor(), Some(RowId(4)));
    assert_eq!(sel.selected_ids(), vec![RowId(2), RowId(3), RowId(4)]);

    // Extending backwards from the same anchor covers the other side only.
    sel.navigate(-4, &rows, true);
    assert_eq!(sel.cursor(), Some(RowId(0)));
    assert_eq!(sel.selected_ids(), vec![RowId(0), RowId(1), RowId(2)]);
}

#[test]
fn navigation_clamps_at_both_ends() {
    let rows = ids(3);
    let mut sel = SelectionModel::new(SelectionMode::Single);
    sel.move_to(0, &rows);
    assert!(
        !sel.navigate(-1, &rows, false),
        "already at the top: no change"
    );
    assert_eq!(sel.cursor(), Some(RowId(0)));

    sel.move_to(2, &rows);
    assert!(
        !sel.navigate(9, &rows, false),
        "already at the bottom: no change"
    );
    assert_eq!(sel.cursor(), Some(RowId(2)));
}

#[test]
fn navigation_from_nothing_lands_on_the_first_or_last_row() {
    let rows = ids(4);
    let mut sel = SelectionModel::new(SelectionMode::Single);
    sel.navigate(1, &rows, false);
    assert_eq!(sel.cursor(), Some(RowId(0)), "no cursor + down = first row");

    let mut sel = SelectionModel::new(SelectionMode::Single);
    sel.navigate(-1, &rows, false);
    assert_eq!(sel.cursor(), Some(RowId(3)), "no cursor + up = last row");
}

#[test]
fn toggle_picks_up_and_drops_individual_rows() {
    let rows = ids(4);
    let mut sel = SelectionModel::new(SelectionMode::Multiple);
    sel.move_to(0, &rows);
    assert!(sel.toggle_at(2, &rows));
    assert_eq!(sel.selected_ids(), vec![RowId(0), RowId(2)]);
    assert!(sel.toggle_at(0, &rows), "toggling a selected row drops it");
    assert_eq!(sel.selected_ids(), vec![RowId(2)]);
}

#[test]
fn single_mode_ignores_multi_select_requests() {
    let rows = ids(4);
    let mut sel = SelectionModel::new(SelectionMode::Single);
    sel.move_to(0, &rows);
    sel.extend_to(3, &rows);
    assert_eq!(
        sel.selected_ids(),
        vec![RowId(3)],
        "single mode cannot hold a range"
    );
    assert!(
        !sel.select_all(&rows),
        "single mode has nothing to select-all into"
    );
    assert_eq!(sel.selected_count(), 1);
}

#[test]
fn narrowing_multiple_to_single_collapses_to_the_cursor() {
    let rows = ids(4);
    let mut sel = SelectionModel::new(SelectionMode::Multiple);
    sel.select_all(&rows);
    assert_eq!(sel.selected_count(), 4);
    assert!(sel.set_mode(SelectionMode::Single));
    assert_eq!(sel.selected_count(), 1);
    assert_eq!(sel.cursor(), Some(RowId(0)));
}

#[test]
fn reselecting_after_rows_shrink_drops_vanished_ids() {
    // A filter or a model reload can remove selected rows; stale ids would let
    // `selected_count` report rows that no longer exist.
    let mut sel = SelectionModel::new(SelectionMode::Multiple);
    sel.select_all(&ids(6));
    assert_eq!(sel.selected_count(), 6);

    let surviving = vec![RowId(1), RowId(4), RowId(5)];
    assert!(sel.clamp_to(&surviving));
    assert_eq!(sel.selected_ids(), vec![RowId(1), RowId(4), RowId(5)]);
    assert_eq!(
        sel.cursor(),
        Some(RowId(1)),
        "cursor snaps to the first survivor"
    );
}

#[test]
fn select_all_reports_whether_anything_changed() {
    let rows = ids(3);
    let mut sel = SelectionModel::new(SelectionMode::Multiple);
    assert!(sel.select_all(&rows), "first select-all changes state");
    assert!(!sel.select_all(&rows), "second is a no-op");
    assert!(sel.clear());
    assert!(!sel.clear());
    assert!(!sel.is_selected(RowId(0)));
    assert_eq!(sel.cursor(), None);
}

// --- edit history ----------------------------------------------------------

fn edit(row: u64, before: &str, after: &str) -> CellEdit {
    CellEdit {
        row: RowId(row),
        column: ColumnId(0),
        before: CellValue::Text(before.into()),
        after: CellValue::Text(after.into()),
    }
}

#[test]
fn edit_history_undoes_and_redoes_in_order() {
    let mut h = EditHistory::new();
    assert!(h.push(edit(0, "a", "b")));
    assert!(h.push(edit(1, "c", "d")));
    assert!(h.can_undo());
    assert!(!h.can_redo());

    let last = h.undo().expect("undo pops the newest edit");
    assert_eq!(last.before, CellValue::Text("c".into()));
    assert_eq!(last.after, CellValue::Text("d".into()));
    assert!(h.can_redo());

    let first = h.undo().expect("second undo pops the older edit");
    assert_eq!(first.row, RowId(0));
    assert!(!h.can_undo());
    assert_eq!(h.undo(), None, "undo on an empty stack is a no-op");

    // Redo replays oldest-first, which is the order they were undone in.
    assert_eq!(h.redo().map(|e| e.row), Some(RowId(0)));
    assert_eq!(h.redo().map(|e| e.row), Some(RowId(1)));
    assert_eq!(h.redo(), None);
}

#[test]
fn recording_a_new_edit_discards_the_redo_branch() {
    // Standard editor semantics: once you type after undoing, the undone
    // future is unreachable and must not be replayable.
    let mut h = EditHistory::new();
    h.push(edit(0, "a", "b"));
    h.undo();
    assert!(h.can_redo());
    h.push(edit(0, "a", "z"));
    assert!(!h.can_redo());
    assert_eq!(h.redo(), None);
}

#[test]
fn a_no_op_edit_never_enters_the_history() {
    // Otherwise "undo" would consume a press and appear to do nothing.
    let mut h = EditHistory::new();
    assert!(!h.push(edit(0, "a", "a")));
    assert!(!h.can_undo());
    assert_eq!(h.undo_len(), 0);
}

#[test]
fn edit_session_parses_through_the_column_kind() {
    let mut s = EditSession::begin(RowId(0), ColumnId(0), CellValue::Number(7.0));
    assert_eq!(
        s.text, "7",
        "the session seeds with the current value as text"
    );
    assert!(s.set_text("12"));
    assert_eq!(
        s.to_value(ColumnKind::Number),
        Some(CellValue::Number(12.0))
    );

    assert!(s.set_text("oops"));
    assert_eq!(
        s.to_value(ColumnKind::Number),
        None,
        "unparseable input is refused"
    );
    assert_eq!(
        s.finish(ColumnKind::Number),
        None,
        "and cannot be committed"
    );

    // Anything goes in a text column.
    assert_eq!(
        s.finish(ColumnKind::Text),
        Some(CellEdit {
            row: RowId(0),
            column: ColumnId(0),
            before: CellValue::Number(7.0),
            after: CellValue::Text("oops".into()),
        })
    );
}

#[test]
fn edit_session_typing_moves_the_caret_and_reports_nothing_from_an_unchanged_commit() {
    let mut s = EditSession::begin(RowId(2), ColumnId(1), CellValue::Text("ab".into()));
    assert_eq!(s.caret, 2, "the caret starts at the end");
    assert!(s.insert('c'));
    assert_eq!(s.text, "abc");
    assert!(s.backspace());
    assert_eq!(s.text, "ab");
    assert!(s.backspace(), "deleting the second character");
    assert_eq!(s.text, "a");

    // Walking the caret to the start turns backspace into a no-op, which it has
    // to report rather than silently "succeeding" — and the caret cannot be
    // walked off either end of the text.
    assert!(s.move_caret(-1));
    assert!(!s.move_caret(-1), "there is no character before the first");
    assert!(!s.backspace());
    assert_eq!(s.text, "a");
    assert!(s.move_caret(1));
    assert!(!s.move_caret(1), "nor one past the last");
    assert_eq!(
        s.finish(ColumnKind::Text),
        Some(CellEdit {
            row: RowId(2),
            column: ColumnId(1),
            before: CellValue::Text("ab".into()),
            after: CellValue::Text("a".into()),
        })
    );

    // Committing without typing produces no edit at all.
    let s = EditSession::begin(RowId(0), ColumnId(0), CellValue::Text("same".into()));
    assert_eq!(s.finish(ColumnKind::Text), None);
}

// --- model -----------------------------------------------------------------

#[test]
fn vec_table_is_read_only_until_its_cells_are_written() {
    let mut t = table(&[&["a"], &["b"]]);
    assert_eq!(t.row_count(), 2);
    assert_eq!(t.cell(RowId(1), ColumnId(0)), CellValue::Text("b".into()));
    assert!(t.set_cell(RowId(1), ColumnId(0), CellValue::Text("z".into())));
    assert_eq!(t.cell(RowId(1), ColumnId(0)), CellValue::Text("z".into()));
    // A column that does not exist is refused rather than silently dropped.
    assert!(!t.set_cell(RowId(1), ColumnId(9), CellValue::Empty));
    // An out-of-range row is refused too.
    assert!(!t.set_cell(RowId(99), ColumnId(0), CellValue::Empty));
}

#[test]
fn default_table_model_refuses_writes() {
    struct ReadOnly;
    impl TableModel for ReadOnly {
        fn row_count(&self) -> usize {
            1
        }
        fn row_id(&self, _index: usize) -> RowId {
            RowId(0)
        }
        fn cell(&self, _row: RowId, _column: ColumnId) -> CellValue {
            CellValue::Text("x".into())
        }
    }
    let mut m = ReadOnly;
    assert!(!m.set_cell(RowId(0), ColumnId(0), CellValue::Empty));
    assert_eq!(m.cell(RowId(0), ColumnId(0)), CellValue::Text("x".into()));
}

#[test]
fn column_builders_carry_the_edit_policy() {
    let c = Column::new("Name", 120.0);
    assert_eq!(c.title, "Name");
    assert_eq!(c.width, 120.0);
    assert_eq!(c.kind, ColumnKind::Text);
    assert!(c.sortable, "columns sort by default");
    assert!(!c.editable, "columns are read-only by default");

    let c = c.editable(true).sortable(false).kind(ColumnKind::Number);
    assert!(c.editable);
    assert!(!c.sortable);
    assert_eq!(c.kind, ColumnKind::Number);
}

#[test]
fn column_ids_are_their_position() {
    assert_eq!(ColumnId(3).index(), 3);
}
