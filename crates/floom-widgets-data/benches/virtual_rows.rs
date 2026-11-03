//! Benchmarks for the virtualized row path.
//!
//! These measure what the windowing claim is actually about: the cost of
//! painting and of maintaining element-backed rows must be a function of the
//! viewport, not of the model. The `paint_window` group is where that shows up
//! — its numbers should be flat across 1k / 100k rows, while `build_model`
//! scales linearly and is included as the control that proves the row count is
//! really being varied.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use floom_core::{ElementId, Rect, Widget};
use floom_widgets::testkit::SceneRecorder;
use floom_widgets_data::data_grid::{DataGrid, RowPool};
use floom_widgets_data::model::{CellValue, Column, VecTable};

const ROW_H: f64 = 20.0;
const HEADER: f64 = 24.0;

fn rect() -> Rect {
    Rect {
        x0: 0.0,
        y0: 0.0,
        x1: 300.0,
        y1: HEADER + 20.0 * ROW_H,
    }
}

fn grid(rows: usize) -> DataGrid {
    let table = VecTable::from_rows(
        (0..rows)
            .map(|i| {
                vec![
                    CellValue::Text(format!("row {i}")),
                    CellValue::Number(i as f64),
                    CellValue::Bool(i % 2 == 0),
                ]
            })
            .collect(),
    );
    let columns = vec![
        Column::new("Name", 140.0),
        Column::new("Score", 80.0),
        Column::new("Done", 80.0),
    ];
    let mut grid = DataGrid::new(columns, table)
        .row_height(ROW_H)
        .header_height(HEADER);
    grid.set_container(rect());
    grid
}

fn bench_paint_window(c: &mut Criterion) {
    let mut group = c.benchmark_group("paint_window");
    for &n in &[1_000usize, 100_000] {
        let grid = grid(n);
        let id = ElementId::new(0, 1);
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                let mut recorder = SceneRecorder::new(rect());
                grid.paint(&mut recorder, id);
                recorder.finish();
                std::hint::black_box(recorder.ops().len())
            });
        });
    }
    group.finish();
}

fn bench_scroll_and_recycle(c: &mut Criterion) {
    let n = 100_000usize;
    let mut group = c.benchmark_group("scroll_and_recycle");
    group.bench_function("scroll_100k_recycle", |b| {
        b.iter(|| {
            let mut grid = grid(n);
            let mut pool = RowPool::new();
            grid.sync_rows(&mut pool);
            for step in 0..1_000 {
                grid.scroll_to(f64::from(step) * ROW_H);
                grid.sync_rows(&mut pool);
            }
            std::hint::black_box((pool.high_water(), pool.allocated()))
        });
    });
    group.finish();
}

fn bench_build_model(c: &mut Criterion) {
    // The control: this one *should* scale with the row count, which is what
    // makes a flat `paint_window` meaningful rather than suspicious.
    let mut group = c.benchmark_group("build_model");
    for &n in &[1_000usize, 100_000] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, n| {
            b.iter(|| std::hint::black_box(grid(*n)));
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_paint_window,
    bench_scroll_and_recycle,
    bench_build_model
);
criterion_main!(benches);
