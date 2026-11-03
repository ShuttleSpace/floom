
use floom_core::Widget;
use floom_widgets::{Axis, Stack};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::MultiTheme;

use floom_widget_md_grid::MaterialGrid;

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn grid(cells: usize, cols: usize) -> MaterialGrid {
    let mut g = MaterialGrid::new()
        .with_theme(theme())
        .cols(cols)
        .gap(8.0)
        .row_gap(12.0);
    for i in 0..cells {
        g = g.child(move || Box::new(floom_widgets::Text::new(format!("cell {i}"))));
    }
    g
}

/// The columns reach the engine grid.
#[test]
fn the_columns_reach_the_engine_grid() {
    let g = grid(4, 2);
    assert_eq!(g.resolved_style().cols, 2);
    assert_eq!(g.view().columns(), 2);
}

/// The gaps reach the engine grid: the row gap on the grid, the column gap on
/// each row.
#[test]
fn the_gaps_reach_the_engine_grid() {
    let g = grid(4, 2).view();
    let request = g.layout_request().expect("a grid asks for its own space");
    assert_eq!(request.axis, Some(Axis::Vertical), "rows stack vertically");
    assert_eq!(request.gap, Some(12.0), "with the row gap between them");

    let row = g.children()[0].build();
    let row = row.as_any().downcast_ref::<Stack>().unwrap().clone();
    let row_request = row.layout_request().expect("a row asks for its own space");
    assert_eq!(row_request.axis, Some(Axis::Horizontal), "cells run across");
    assert_eq!(
        row_request.gap,
        Some(8.0),
        "with the column gap between them"
    );
}

/// Every cell grows, which is what makes them equal width.
#[test]
fn every_cell_grows() {
    let g = grid(4, 2).view();
    let row = g.children()[0].build();
    let row = row.as_any().downcast_ref::<Stack>().unwrap().clone();
    for cell in row.children() {
        assert_eq!(
            cell.layout.flex_grow,
            Some(1.0),
            "v1 wrapped every cell so it grew: `flex_grow(1.0)`"
        );
    }
}

/// The last row is padded, so its cells are the same width as the rows above.
#[test]
fn the_last_row_is_padded() {
    let g = grid(3, 2).view();
    let rows = g.children();
    assert_eq!(rows.len(), 2, "two rows");
    for row in &rows {
        let row = row.build();
        let row = row.as_any().downcast_ref::<Stack>().unwrap().clone();
        assert_eq!(row.child_count(), 2, "every row holds a full set of cells");
    }
}

/// A grid whose cells divide evenly is not padded.
#[test]
fn an_even_grid_is_not_padded() {
    let g = grid(4, 2).view();
    let rows = g.children();
    assert_eq!(rows.len(), 2);
    for row in &rows {
        let row = row.build();
        let row = row.as_any().downcast_ref::<Stack>().unwrap().clone();
        assert_eq!(row.child_count(), 2);
    }
}
