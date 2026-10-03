//! Golden example: data grid (Phase 30.1).
//!
//! Three things to see, all headless:
//!
//! 1. **Virtualization is a cost claim** — a grid showing ten rows must paint
//!    roughly the same number of ops whether the model has 1 000 rows or
//!    100 000. Two grids of *identical shape*, differing only in row count,
//!    are compared by emitted paint ops.
//! 2. **Row pooling** — `sync_rows` recycles elements through a bounded pool;
//!    its high-water mark stays at the visible window size.
//! 3. **Model operations** — sorting, selection and a cell edit with undo/redo
//!    are independent of how many rows exist.
//!
//! Run with `cargo run -p floom2-datagrid`.

use floom_core::{ElementId, Point, Rect, Widget};
use floom_types::PointerEvent;
use floom_virtual::Overscan;
use floom_widgets::testkit::{
    AllocatingCreateContext, RecordingUpdateContext, SceneRecorder, dispatch,
};
use floom_widgets_data::data_grid::{DataGrid, RowPool};
use floom_widgets_data::model::{
    CellValue, Column, ColumnId, ColumnKind, RowId,     SelectionMode, SortSpec, VecTable,
};

const HEADER: f64 = 24.0;
const ROW_H: f64 = 20.0;
/// Room for exactly ten rows under the header.
const HEIGHT: f64 = HEADER + 10.0 * ROW_H;
const WIDTH: f64 = 300.0;

fn bounds() -> Rect {
    Rect {
        x0: 0.0,
        y0: 0.0,
        x1: WIDTH,
        y1: HEIGHT,
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

/// Two grids of identical shape: same ten-row window, different model sizes.
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
    grid.set_container(bounds());
    grid
}

fn create(grid: &DataGrid) -> ElementId {
    let mut cx = AllocatingCreateContext::new();
    grid.create(&mut cx)
}

/// Paint ops after an update — the measure that would grow if the grid were
/// not virtualized.
fn paint_ops(grid: &mut DataGrid, id: ElementId) -> usize {
    let mut update = RecordingUpdateContext::new();
    grid.update(&mut update, id);
    let mut recorder = SceneRecorder::new(bounds());
    grid.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().len()
}

/// A double click at `x`, dispatched the way the UI dispatches it.
fn double_click(grid: &mut DataGrid, id: ElementId, x: f64) -> bool {
    let mut event = PointerEvent::primary_down(Point::new(x, HEADER + 25.0));
    event.click_count = 2;
    dispatch(grid, id, bounds(), event.into())
}

fn first_rows(grid: &DataGrid) -> Vec<u64> {
    grid.visible_row_ids().iter().take(3).map(|row| row.0).collect()
}

fn main() {
    // --- 1. virtualization -------------------------------------------------
    let mut small = grid_with(1_000);
    let mut large = grid_with(100_000);

    let small_id = create(&small);
    let large_id = create(&large);

    let small_ops = paint_ops(&mut small, small_id);
    let large_ops = paint_ops(&mut large, large_id);

    println!("rows behind the grid : 1,000 vs 100,000");
    println!("paint ops           : {small_ops} vs {large_ops}");
    assert_eq!(
        small_ops, large_ops,
        "a virtualized grid paints its window, not its model: {small_ops} vs {large_ops} ops"
    );

    // --- 2. row pooling ----------------------------------------------------
    let mut pool = RowPool::new();
    let mut peak = 0usize;
    for offset in (0..100_000).step_by(37) {
        large.scroll_to(offset as f64 * ROW_H);
        let live = large.sync_rows(&mut pool);
        peak = peak.max(live);
    }
    println!(
        "row pool high-water : {peak} live rows (allocated {}, in use {})",
        pool.allocated(),
        pool.in_use()
    );
    assert!(
        peak <= 16,
        "only the visible window may be materialized, peaked at {peak}"
    );
    assert!(
        pool.high_water() >= peak,
        "the pool must have supplied every live row"
    );

    // --- 3. sorting, selection, edit + undo --------------------------------
    let mut grid = grid_with(1_000);
    let id = create(&grid);
    let mut update = RecordingUpdateContext::new();
    grid.update(&mut update, id);

    println!("\nunsorted first rows   : {:?}", first_rows(&grid));

    let score = ColumnId(1);
    assert!(grid.toggle_sort(score), "the Score column is sortable");
    grid.update(&mut update, id);
    println!(
        "sorted ascending      : {:?} ({:?})",
        first_rows(&grid),
        grid.sort().map(|spec| spec.direction)
    );
    assert!(grid.sort().is_some());

    assert!(grid.toggle_sort(score), "ascending -> descending");
    grid.update(&mut update, id);
    println!("sorted descending     : {:?}", first_rows(&grid));

    assert!(
        grid.toggle_sort(score),
        "descending -> unsorted is a real change"
    );
    assert_eq!(grid.sort(), None);
    println!("third click unsorted  : {:?}", first_rows(&grid));

    grid.set_sort(Some(SortSpec::ascending(score)));
    grid.update(&mut update, id);

    // Selection addresses *displayed* rows, so it is independent of row count.
    assert!(grid.select_row(2), "selecting a row changed the selection");
    assert!(grid.select_all(), "select-all changed the selection");
    println!("selection             : select_row + select_all both reported a change");

    // A cell edit goes through an *edit session*, which is the path the UI
    // takes: a double click opens the editor on an editable column, the text is
    // typed, and committing pushes one entry onto the grid's history.
    let row = grid.display_row_at(1).expect("at least two rows");
    let before = grid.model().cell(row, score).display();

    // Read-only column: a double click selects it but opens no editor.
    assert!(double_click(&mut grid, id, 10.0), "the click was handled");
    assert!(
        grid.editing().is_none(),
        "the Name column is not editable, so no editor opens"
    );

    // Editable column (Score is at x=200).
    assert!(double_click(&mut grid, id, 200.0), "the click was handled");
    let session = grid.editing().expect("an editable cell opens an editor");
    println!(
        "editor open           : row={:?} column={:?}",
        session.row, session.column
    );
    assert_eq!(session.row, row);

    assert!(grid.edit_set_text("4242"), "typing changes the editor text");
    let committed = grid.commit_edit().expect("4242 parses as a Number cell");
    grid.update(&mut update, id);
    let edited = grid.model().cell(row, score).display();
    println!(
        "cell edit             : {before:?} -> {edited:?} (committed row {:?})",
        committed.row
    );
    assert_ne!(before, edited);

    assert!(grid.undo().is_some(), "the committed edit is undoable");
    grid.update(&mut update, id);
    println!("after undo            : {:?}", grid.model().cell(row, score).display());
    assert_eq!(grid.model().cell(row, score).display(), before);

    assert!(grid.redo().is_some(), "the committed edit is redoable");
    grid.update(&mut update, id);
    assert_eq!(grid.model().cell(row, score).display(), edited);
    println!("after redo            : {:?}", grid.model().cell(row, score).display());

    // The row identity never changes underneath the view.
    assert_eq!(grid.model().row_id(0), RowId(0));
    println!("\ndatagrid golden example: OK");
}
