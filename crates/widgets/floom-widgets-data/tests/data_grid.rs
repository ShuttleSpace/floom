// SPDX-License-Identifier: MIT
//! DataGrid tests: virtualization, keyboard/pointer selection, sorting, edit +
//! undo, and the semantics summary.
//!
//! The virtualization tests are the important ones. "Virtualized" is only a
//! claim about cost, so each of them measures something that would grow with
//! the row count if the window were not real: emitted paint ops, and the size
//! of the materialized-row pool.

use floom_core::{ElementId, Key, NamedKey, Point, Rect, Widget, WidgetAny};
use floom_types::{KeyEvent, Modifiers, PointerEvent};
use floom_virtual::Overscan;
use floom_widgets::testkit::{
    AllocatingCreateContext, RecordingUpdateContext, SceneRecorder, dispatch, semantics_snapshot,
};
use floom_widgets_data::data_grid::{DataGrid, RowPool};
use floom_widgets_data::model::{
    CellValue, Column, ColumnId, ColumnKind, RowId, SelectionMode, VecTable,
};

const HEADER: f64 = 24.0;
const ROW_H: f64 = 20.0;
const WIDTH: f64 = 300.0;
/// Room for exactly ten rows under the header.
const TEN_ROWS: f64 = HEADER + 10.0 * ROW_H;

fn rect(height: f64) -> Rect {
    Rect {
        x0: 0.0,
        y0: 0.0,
        x1: WIDTH,
        y1: height,
    }
}

fn columns() -> Vec<Column> {
    vec![
        Column::new("Name", 140.0),
        Column::new("Score", 80.0)
            .kind(ColumnKind::Number)
            .editable(true),
        Column::new("Done", 80.0).kind(ColumnKind::Bool),
    ]
}

/// A grid over `n` synthetic rows, seeded with a ten-row container.
fn grid_with(n: usize) -> DataGrid {
    let rows: Vec<Vec<CellValue>> = (0..n)
        .map(|i| {
            vec![
                CellValue::Text(format!("row {i}")),
                CellValue::Number(i as f64),
                CellValue::Bool(i % 2 == 0),
            ]
        })
        .collect();
    let mut grid = DataGrid::new(columns(), VecTable::from_rows(rows))
        .row_height(ROW_H)
        .header_height(HEADER)
        .selection_mode(SelectionMode::Multiple)
        .overscan(Overscan::Items(0));
    grid.set_container(rect(TEN_ROWS));
    grid
}

fn allocate(widget: &dyn Widget) -> ElementId {
    let mut create = AllocatingCreateContext::new();
    widget.create(&mut create)
}

fn key(grid: &mut DataGrid, id: ElementId, key: Key, modifiers: Modifiers) -> bool {
    let event = KeyEvent {
        phase: floom_core::KeyPhase::Down,
        key,
        modifiers,
    };
    dispatch(grid, id, rect(TEN_ROWS), event.into())
}

fn named(n: NamedKey) -> Key {
    Key::Named(n)
}

fn paint_ops(grid: &DataGrid, id: ElementId) -> Vec<String> {
    let mut rec = SceneRecorder::new(rect(TEN_ROWS));
    grid.paint(&mut rec, id);
    rec.finish();
    rec.ops().iter().map(|op| op.snapshot_line()).collect()
}

#[test]
fn visible_range_follows_the_container_not_the_row_count() {
    let mut small = grid_with(1_000);
    let mut large = grid_with(100_000);
    assert_eq!(small.visible_range(), 0..10);
    assert_eq!(
        large.visible_range(),
        0..10,
        "the window is a function of the box"
    );

    // A taller box shows more rows; the row count still does not matter.
    small.set_container(rect(HEADER + 20.0 * ROW_H));
    large.set_container(rect(HEADER + 20.0 * ROW_H));
    assert_eq!(small.visible_range(), 0..20);
    assert_eq!(large.visible_range(), 0..20);
}

#[test]
fn overscan_widens_the_window_without_materializing_everything() {
    let mut grid = grid_with(100_000);
    grid.set_overscan(Overscan::Items(2));
    assert_eq!(grid.visible_range(), 0..12);
    assert_eq!(grid.materialized_count(), 12);
}

#[test]
fn paint_cost_is_flat_in_the_row_count() {
    // The sharp form of "virtualized": a 100k-row grid must emit the same
    // number of paint ops as a 1k-row grid, because only the window is drawn.
    let small = grid_with(1_000);
    let large = grid_with(100_000);
    let id = allocate(&small);
    let small_ops = paint_ops(&small, id);
    let large_ops = paint_ops(&large, id);

    assert_eq!(
        small_ops.len(),
        large_ops.len(),
        "paint ops grew with the row count — the window is not real"
    );
    assert!(
        large_ops.len() < 100,
        "expected a ten-row window of ops, got {}",
        large_ops.len()
    );
    // And the window's cells actually got drawn: every visible cell emitted its
    // own text op, so 10 rows x 3 columns on top of the 3 header titles. The
    // "row N" filter is narrower — only the Name column renders that shape.
    let name_cells = large_ops
        .iter()
        .filter(|line| line.contains("\"row "))
        .count();
    assert_eq!(name_cells, 10, "one name cell per visible row");
    let text_ops = large_ops
        .iter()
        .filter(|line| line.starts_with("Text"))
        .count();
    assert_eq!(
        text_ops,
        3 /* header titles */ + 10 * 3,
        "one text op per visible cell"
    );
}

#[test]
fn scrolling_moves_the_window_and_the_painted_rows() {
    let grid = {
        let mut g = grid_with(1_000);
        g.scroll_to(200.0);
        g
    };
    assert_eq!(grid.scroll_offset(), 200.0);
    assert_eq!(grid.visible_range(), 10..20);

    let id = allocate(&grid);
    let ops = paint_ops(&grid, id);
    assert!(
        ops.iter().any(|l| l.contains("row 10")),
        "the window must paint the row it scrolled to"
    );
    assert!(
        !ops.iter().any(|l| l.contains("row 0\"")),
        "and must not paint the row it scrolled past"
    );
}

#[test]
fn scroll_is_clamped_to_the_content() {
    let mut grid = grid_with(100);
    // 100 rows * 20 = 2000px of content in a 200px window => 1800px of travel.
    assert_eq!(grid.max_scroll(), 1800.0);
    grid.scroll_to(10_000.0);
    assert_eq!(grid.scroll_offset(), 1800.0);
    grid.scroll_to(-50.0);
    assert_eq!(grid.scroll_offset(), 0.0);

    // A model shorter than the window has nowhere to scroll at all.
    let mut short = grid_with(3);
    assert_eq!(short.max_scroll(), 0.0);
    short.scroll_to(100.0);
    assert_eq!(short.scroll_offset(), 0.0);
}

#[test]
fn the_materialized_row_pool_stays_bounded_while_scrolling() {
    // This is what a retained-mode host actually pays: element-backed rows.
    // Scrolling a 100k-row model must reuse a window-sized pool, not grow to
    // 100k elements.
    let mut grid = grid_with(100_000);
    let mut pool = RowPool::new();
    grid.sync_rows(&mut pool);

    for step in 0..2_000 {
        grid.scroll_to(f64::from(step) * 20.0);
        grid.sync_rows(&mut pool);
    }

    assert!(
        pool.high_water() <= 12,
        "pool grew to {} rows — rows are leaking instead of being recycled",
        pool.high_water()
    );
    assert_eq!(pool.in_use(), grid.materialized_count());
    assert!(
        pool.allocated() <= 12,
        "allocated {} ids for a 10-row window",
        pool.allocated()
    );
}

#[test]
fn sync_rows_binds_the_rows_the_window_names() {
    let mut grid = grid_with(1_000);
    let mut pool = RowPool::new();
    grid.scroll_to(400.0); // display rows 20..30
    grid.sync_rows(&mut pool);
    assert_eq!(
        grid.visible_row_ids(),
        (20..30).map(RowId).collect::<Vec<_>>()
    );

    let bound: Vec<RowId> = grid
        .visible_row_ids()
        .into_iter()
        .filter(|r| grid.bound_element(*r).is_some())
        .collect();
    assert_eq!(bound, (20..30).map(RowId).collect::<Vec<_>>());
}

#[test]
fn header_click_cycles_the_sort_and_reorders_the_window() {
    let mut grid = grid_with(5);
    let id = allocate(&grid);
    let header_click = |grid: &mut DataGrid, x: f64| {
        dispatch(
            grid,
            id,
            rect(TEN_ROWS),
            PointerEvent::primary_down(Point::new(x, 5.0)).into(),
        )
    };
    // First click on "Score": ascending.
    assert!(header_click(&mut grid, 200.0));
    assert_eq!(grid.sort().map(|s| s.column), Some(ColumnId(1)));
    assert_eq!(grid.display_row_at(0), Some(RowId(0)));

    // Writing through the model slot re-sorts in place: no explicit resort call
    // is needed, and the new minimum must surface at the top.
    grid.set_cell(RowId(1), ColumnId(1), CellValue::Number(-5.0));
    assert_eq!(
        grid.display_row_at(0),
        Some(RowId(1)),
        "the smallest value sorts first"
    );

    // Second click: descending, so it comes back to the end.
    header_click(&mut grid, 200.0);
    assert_eq!(grid.display_row_at(4), Some(RowId(1)));

    // Third click: sort cleared, model order restored.
    header_click(&mut grid, 200.0);
    assert_eq!(grid.sort(), None);
    assert_eq!(grid.display_row_at(0), Some(RowId(0)));
    assert_eq!(grid.display_row_at(1), Some(RowId(1)));
}

#[test]
fn a_column_can_refuse_to_sort() {
    let mut grid = DataGrid::new(
        vec![Column::new("Fixed", 100.0).sortable(false)],
        VecTable::from_rows(vec![vec![CellValue::Text("a".into())]]),
    );
    assert!(!grid.toggle_sort(ColumnId(0)));
    assert_eq!(grid.sort(), None);
}

#[test]
fn clicking_selects_the_row_under_the_pointer() {
    let mut grid = grid_with(50);
    let id = allocate(&grid);
    // Row 3 occupies y in [HEADER + 60, HEADER + 80).
    assert!(dispatch(
        &mut grid,
        id,
        rect(TEN_ROWS),
        PointerEvent::primary_down(Point::new(10.0, HEADER + 65.0)).into()
    ));
    assert_eq!(grid.selection().cursor(), Some(RowId(3)));
    assert_eq!(grid.selection().selected_ids(), vec![RowId(3)]);

    // A click on the header row must not select anything.
    dispatch(
        &mut grid,
        id,
        rect(TEN_ROWS),
        PointerEvent::primary_down(Point::new(200.0, 5.0)).into(),
    );
    assert_eq!(grid.selection().cursor(), Some(RowId(3)));
}

#[test]
fn shift_click_extends_and_modifier_click_toggles() {
    let mut grid = grid_with(50);
    let id = allocate(&grid);
    let click = |grid: &mut DataGrid, y: f64, modifiers: Modifiers| {
        let mut ev = PointerEvent::primary_down(Point::new(10.0, y));
        ev.modifiers = modifiers;
        dispatch(grid, id, rect(TEN_ROWS), ev.into())
    };

    click(&mut grid, HEADER + 25.0, Modifiers::NONE); // row 1
    click(&mut grid, HEADER + 85.0, Modifiers::SHIFT); // rows 1..=4
    assert_eq!(
        grid.selection().selected_ids(),
        vec![RowId(1), RowId(2), RowId(3), RowId(4)]
    );

    click(&mut grid, HEADER + 125.0, Modifiers::META); // toggle row 6
    assert!(grid.selection().is_selected(RowId(6)));
    assert!(
        grid.selection().is_selected(RowId(2)),
        "toggle keeps the rest"
    );
}

#[test]
fn double_click_opens_an_editor_only_on_an_editable_cell() {
    let mut grid = grid_with(50);
    let id = allocate(&grid);
    let double = |grid: &mut DataGrid, x: f64| {
        let mut ev = PointerEvent::primary_down(Point::new(x, HEADER + 25.0));
        ev.click_count = 2;
        dispatch(grid, id, rect(TEN_ROWS), ev.into())
    };

    // x=10 lands in the read-only "Name" column: the selection still happens,
    // but no editor opens.
    assert!(double(&mut grid, 10.0));
    assert!(grid.editing().is_none());

    // x=200 lands in the editable "Score" column.
    assert!(double(&mut grid, 200.0));
    let session = grid.editing().expect("an editable cell opens an editor");
    assert_eq!(session.row, RowId(1));
    assert_eq!(session.column, ColumnId(1));
    assert_eq!(session.text, "1");
}

#[test]
fn arrow_keys_move_the_cursor_and_keep_it_scrolled_into_view() {
    let mut grid = grid_with(1_000);
    let id = allocate(&grid);

    assert!(key(
        &mut grid,
        id,
        named(NamedKey::ArrowDown),
        Modifiers::NONE
    ));
    assert_eq!(grid.selection().cursor(), Some(RowId(0)));

    // Walk past the bottom of the window; the viewport must follow.
    for _ in 0..14 {
        key(&mut grid, id, named(NamedKey::ArrowDown), Modifiers::NONE);
    }
    assert_eq!(grid.selection().cursor(), Some(RowId(14)));
    let range = grid.visible_range();
    assert!(
        range.contains(&14),
        "cursor row 14 is outside the window {range:?} — it did not scroll"
    );

    key(&mut grid, id, named(NamedKey::Home), Modifiers::NONE);
    assert_eq!(grid.selection().cursor(), Some(RowId(0)));
    assert_eq!(grid.scroll_offset(), 0.0);

    key(&mut grid, id, named(NamedKey::End), Modifiers::NONE);
    assert_eq!(grid.selection().cursor(), Some(RowId(999)));
    assert_eq!(grid.scroll_offset(), grid.max_scroll());
}

#[test]
fn shift_arrow_extends_the_selection() {
    let mut grid = grid_with(50);
    let id = allocate(&grid);
    key(&mut grid, id, named(NamedKey::ArrowDown), Modifiers::NONE); // row 0
    key(&mut grid, id, named(NamedKey::ArrowDown), Modifiers::SHIFT);
    key(&mut grid, id, named(NamedKey::ArrowDown), Modifiers::SHIFT);
    assert_eq!(
        grid.selection().selected_ids(),
        vec![RowId(0), RowId(1), RowId(2)]
    );
}

#[test]
fn page_keys_move_by_a_window_and_control_a_selects_everything() {
    let mut grid = grid_with(1_000);
    let id = allocate(&grid);
    key(&mut grid, id, named(NamedKey::ArrowDown), Modifiers::NONE);

    // 10 visible rows per page (overscan is off in the fixture).
    key(&mut grid, id, named(NamedKey::PageDown), Modifiers::NONE);
    assert_eq!(grid.selection().cursor(), Some(RowId(10)));
    key(&mut grid, id, named(NamedKey::PageUp), Modifiers::NONE);
    assert_eq!(grid.selection().cursor(), Some(RowId(0)));

    key(&mut grid, id, Key::Char('a'), Modifiers::CONTROL);
    assert_eq!(grid.selection().selected_count(), 1_000);
    key(&mut grid, id, Key::Char('a'), Modifiers::META);
    assert_eq!(grid.selection().selected_count(), 1_000);
}

#[test]
fn left_and_right_move_the_active_column() {
    let mut grid = grid_with(10);
    let id = allocate(&grid);
    assert_eq!(grid.active_column(), ColumnId(0));
    key(&mut grid, id, named(NamedKey::ArrowRight), Modifiers::NONE);
    assert_eq!(grid.active_column(), ColumnId(1));
    key(&mut grid, id, named(NamedKey::ArrowLeft), Modifiers::NONE);
    assert_eq!(grid.active_column(), ColumnId(0));
    // Clamped at both ends.
    key(&mut grid, id, named(NamedKey::ArrowLeft), Modifiers::NONE);
    assert_eq!(grid.active_column(), ColumnId(0));
}

#[test]
fn a_non_modifier_key_only_moves_the_cursor_without_scrolling_the_page() {
    // Guard against a key handler that treats every key as navigation.
    let mut grid = grid_with(1_000);
    let id = allocate(&grid);
    assert!(!key(&mut grid, id, Key::Char('q'), Modifiers::NONE));
    assert_eq!(grid.selection().cursor(), None);
}

#[test]
fn enter_opens_an_editor_type_commits_and_undo_restores() {
    let mut grid = grid_with(50);
    let id = allocate(&grid);
    key(&mut grid, id, named(NamedKey::ArrowDown), Modifiers::NONE); // row 0
    key(&mut grid, id, named(NamedKey::ArrowRight), Modifiers::NONE); // Score column

    assert!(key(&mut grid, id, named(NamedKey::Enter), Modifiers::NONE));
    assert!(grid.editing().is_some());

    // Replace the seeded text.
    grid.edit_set_text("42");
    assert!(key(&mut grid, id, named(NamedKey::Enter), Modifiers::NONE));
    assert!(grid.editing().is_none(), "committing closes the editor");
    assert_eq!(
        grid.model().cell(RowId(0), ColumnId(1)),
        CellValue::Number(42.0)
    );
    assert!(grid.can_undo());

    assert!(key(&mut grid, id, Key::Char('z'), Modifiers::CONTROL));
    assert_eq!(
        grid.model().cell(RowId(0), ColumnId(1)),
        CellValue::Number(0.0),
        "undo restores the previous value"
    );
    assert!(key(
        &mut grid,
        id,
        Key::Char('z'),
        Modifiers::CONTROL.union(Modifiers::SHIFT)
    ));
    assert_eq!(
        grid.model().cell(RowId(0), ColumnId(1)),
        CellValue::Number(42.0),
        "redo replays it"
    );
}

#[test]
fn escape_abandons_the_edit_without_touching_the_model() {
    let mut grid = grid_with(50);
    let id = allocate(&grid);
    key(&mut grid, id, named(NamedKey::ArrowDown), Modifiers::NONE);
    key(&mut grid, id, named(NamedKey::ArrowRight), Modifiers::NONE);
    key(&mut grid, id, named(NamedKey::Enter), Modifiers::NONE);
    grid.edit_set_text("999");

    key(&mut grid, id, named(NamedKey::Escape), Modifiers::NONE);
    assert!(grid.editing().is_none());
    assert_eq!(
        grid.model().cell(RowId(0), ColumnId(1)),
        CellValue::Number(0.0)
    );
    assert!(!grid.can_undo(), "an abandoned edit is not undoable");
}

#[test]
fn typing_into_the_editor_inserts_and_backspaces() {
    let mut grid = grid_with(50);
    let id = allocate(&grid);
    key(&mut grid, id, named(NamedKey::ArrowDown), Modifiers::NONE);
    key(&mut grid, id, named(NamedKey::ArrowRight), Modifiers::NONE);
    key(&mut grid, id, named(NamedKey::Enter), Modifiers::NONE);

    grid.edit_set_text("1");
    assert!(key(&mut grid, id, Key::Char('2'), Modifiers::NONE));
    assert_eq!(grid.editing().unwrap().text, "12");
    assert!(key(
        &mut grid,
        id,
        named(NamedKey::Backspace),
        Modifiers::NONE
    ));
    assert_eq!(grid.editing().unwrap().text, "1");
    // While editing, arrow keys are text navigation, not grid navigation.
    let cursor_before = grid.selection().cursor();
    key(&mut grid, id, named(NamedKey::ArrowDown), Modifiers::NONE);
    assert_eq!(grid.selection().cursor(), cursor_before);
}

#[test]
fn an_unparsable_value_is_refused_and_the_editor_stays_open() {
    let mut grid = grid_with(50);
    let id = allocate(&grid);
    key(&mut grid, id, named(NamedKey::ArrowDown), Modifiers::NONE);
    key(&mut grid, id, named(NamedKey::ArrowRight), Modifiers::NONE);
    key(&mut grid, id, named(NamedKey::Enter), Modifiers::NONE);
    grid.edit_set_text("not a number");

    key(&mut grid, id, named(NamedKey::Enter), Modifiers::NONE);
    assert!(
        grid.editing().is_some(),
        "a refused commit must leave the editor open so the user can fix it"
    );
    assert_eq!(
        grid.model().cell(RowId(0), ColumnId(1)),
        CellValue::Number(0.0)
    );
    assert!(!grid.can_undo());
}

#[test]
fn undo_and_redo_are_no_ops_when_the_stacks_are_empty() {
    let mut grid = grid_with(10);
    let id = allocate(&grid);
    assert!(!grid.can_undo());
    assert!(!key(&mut grid, id, Key::Char('z'), Modifiers::CONTROL));
    assert!(!key(&mut grid, id, Key::Char('y'), Modifiers::CONTROL));
}

#[test]
fn update_marks_the_element_dirty_once_per_change() {
    // A hundred rows, not ten: the ten-row fixture fits its box exactly, so a
    // scroll on it would be a no-op and could not prove anything.
    let mut grid = grid_with(100);
    let id = allocate(&grid);

    let mut cx = RecordingUpdateContext::new();
    grid.update(&mut cx, id);
    assert_eq!(cx.dirty_count(), 1, "the first update must repaint");

    let mut cx = RecordingUpdateContext::new();
    grid.update(&mut cx, id);
    assert_eq!(cx.dirty_count(), 0, "an unchanged grid must not repaint");

    assert!(grid.scroll_to(200.0), "the fixture must be scrollable");
    let mut cx = RecordingUpdateContext::new();
    grid.update(&mut cx, id);
    assert_eq!(cx.dirty_count(), 1, "scrolling must repaint");

    // A scroll that changes nothing is not a change, and must stay quiet.
    assert!(!grid.scroll_to(200.0));
    let mut cx = RecordingUpdateContext::new();
    grid.update(&mut cx, id);
    assert_eq!(cx.dirty_count(), 0, "a no-op scroll must not repaint");
}

#[test]
fn semantics_summarize_the_model_and_the_selection() {
    let grid = grid_with(1_234);
    let id = allocate(&grid);
    let label = semantics_snapshot(&grid, id, rect(TEN_ROWS));
    assert!(label.contains("1234 rows"), "got {label:?}");
    assert!(label.contains("3 columns"), "got {label:?}");
    assert!(
        label.contains("Name"),
        "column titles belong in the label: {label:?}"
    );

    let mut selected = grid_with(1_234);
    let id2 = allocate(&selected);
    selected.select_all();
    let label = semantics_snapshot(&selected, id2, rect(TEN_ROWS));
    assert!(label.contains("1234 selected"), "got {label:?}");
}

#[test]
fn a_grid_is_type_erasable_like_every_other_widget() {
    let grid = grid_with(3);
    let erased: &dyn WidgetAny = &grid;
    assert!(erased.as_any().downcast_ref::<DataGrid>().is_some());
}

#[test]
fn layout_request_fills_its_slot() {
    let grid = grid_with(3);
    let request = grid
        .layout_request()
        .expect("a grid has an opinion on its box");
    assert_eq!(request.flex_grow, Some(1.0));
}
