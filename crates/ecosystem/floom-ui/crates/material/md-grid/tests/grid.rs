//! Material grid: the columns, the gaps, and the padded last row.
//!
//! ## The claim
//!
//! A grid is rows of equal-width cells. v1 chunked the children itself, and
//! 2.0's `Grid` does the chunking -- so the kit is a thin adapter, and the
//! assertions are about what reaches the engine.
//!
//! ## The last row
//!
//! v1 padded the last row with empty cells, so that every row held the same
//! number of cells and each cell grew by a share of the row. Without the
//! padding, the last row's cells would divide the whole width between them and
//! be wider than the rows above. That is the one thing the kit has to do
//! itself, and it is asserted here.

use floom_core::Widget;
use floom_widgets::{Axis, Stack};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::MultiTheme;

use floom_widget_md_grid::MaterialGrid;

/// The Material theme, with Material selected.
///
/// `MultiTheme::new_for(ds)` does *not* select `ds` -- it builds all three and
/// `activate` is a documented no-op -- and a `MultiTheme` reads the *active*
/// design system at read time. So a test that wants Material's own values has to
/// select Material, and `set_design_system` writes a thread-local, so the
/// selection is this test's alone.
fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn grid(cells: usize, cols: usize) -> MaterialGrid {
    let mut g = MaterialGrid::new().with_theme(theme()).cols(cols).gap(8.0).row_gap(12.0);
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
    assert_eq!(row_request.gap, Some(8.0), "with the column gap between them");
}

/// Every cell grows, which is what makes them equal width.
#[test]
fn every_cell_grows() {
    let g = grid(4, 2).view();
    let row = g.children()[0].build();
    let row = row.as_any().downcast_ref::<Stack>().unwrap().clone();
    for cell in row.children() {
        // The grow is on the cell's *slot* -- the request it is mounted with --
        // and not on the widget it builds, which is what the engine's `Grid`
        // applies `item_layout` to.
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
    // Three cells in two columns: the last row holds one real cell, and v1
    // padded it with an empty one so the row still had two.
    let g = grid(3, 2).view();
    let rows = g.children();
    assert_eq!(rows.len(), 2, "two rows");
    for row in &rows {
        let row = row.build();
        let row = row.as_any().downcast_ref::<Stack>().unwrap().clone();
        assert_eq!(
            row.child_count(),
            2,
            "every row holds a full set of cells"
        );
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
