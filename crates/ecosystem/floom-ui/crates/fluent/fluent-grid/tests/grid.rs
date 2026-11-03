//! Fluent grid: the columns, the gaps, and the padded last row.

use floom_core::Widget;
use floom_widgets::{Axis, Stack};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::MultiTheme;

use floom_widget_fluent_grid::FluentGrid;

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn grid(cells: usize, cols: usize) -> FluentGrid {
    let mut g = FluentGrid::new()
        .with_theme(theme())
        .cols(cols)
        .gap(8.0)
        .row_gap(12.0);
    for i in 0..cells {
        g = g.child(move || Box::new(floom_widgets::Text::new(format!("cell {i}"))));
    }
    g
}

#[test]
fn the_columns_reach_the_engine_grid() {
    let g = grid(4, 2);
    assert_eq!(g.resolved_style().cols, 2);
    assert_eq!(g.view().columns(), 2);
}

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
