//! Data: the virtualized table and the chart, both from `floem-widgets-data`.
//!
//! Both widgets are given a box by their slot and paint their *visible window*,
//! so the gallery mounts a thousand rows without paying for them.

use floem_core::{Axis, LayoutRequest, WidgetAny};
use floem_widgets::Stack;
use floem_widgets_data::data_grid::DataGrid;
use floem_widgets_data::model::{CellValue, Column, ColumnKind, SelectionMode, VecTable};
use floem_widgets_data::{AxisSpec, Chart, ChartSeries};

use super::{Demo, Section};

/// Rows the grid is given: more than fit, which is what a scroll exists for.
const ROWS: usize = 1_000;

pub(super) fn demos() -> Vec<Demo> {
    vec![
        Demo {
            id: "data-grid",
            title: "DataGrid",
            section: Section::Data,
            build: build_data_grid,
        },
        Demo {
            id: "chart",
            title: "Chart",
            section: Section::Data,
            build: build_chart,
        },
    ]
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

fn build_data_grid() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical).push_layout(
            LayoutRequest::new().width(300.0).height(220.0),
            || {
                let rows: Vec<Vec<CellValue>> = (0..ROWS)
                    .map(|i| {
                        vec![
                            CellValue::Text(format!("row {i}")),
                            CellValue::Number(i as f64),
                            CellValue::Bool(i % 2 == 0),
                        ]
                    })
                    .collect();
                Box::new(
                    DataGrid::new(columns(), VecTable::from_rows(rows))
                        .row_height(20.0)
                        .header_height(24.0)
                        .selection_mode(SelectionMode::Multiple),
                )
            },
        ),
    )
}

fn build_chart() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical).push_layout(
            LayoutRequest::new().width(280.0).height(180.0),
            || {
                Box::new(
                    Chart::new()
                        .x_axis(AxisSpec::new("quarter"))
                        .y_axis(AxisSpec::new("units").limits(0.0, 8.0))
                        .series(ChartSeries::bars(
                            "units",
                            [(0.0, 3.0), (1.0, 5.0), (2.0, 2.0), (3.0, 6.0)],
                        ))
                        .series(ChartSeries::line(
                            "trend",
                            [(0.0, 1.0), (1.0, 3.5), (2.0, 4.0), (3.0, 7.0)],
                        )),
                )
            },
        ),
    )
}
