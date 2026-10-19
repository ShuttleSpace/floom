//! Fluent grid, ported to the Floom 2.0 widget API.
//!
//! A grid is a set of cells in rows of equal width. v1 built the rows itself:
//! chunk the children, wrap each cell so it grows, and stack the rows with a
//! gap. 2.0's [`Grid`] does the chunking, so the kit is a thin adapter over it.
//!
//! ## The padding of the last row
//!
//! v1 padded the last row with empty cells so that every row held the same
//! number of cells, and each cell grew by a share of the row. Without that, the
//! last row's cells would divide the whole width between them and be wider
//! than the rows above -- so the kit pads the item list before handing it over.
//!
//! ## The cells
//!
//! v1 wrapped each cell in a box that grew (`flex_grow(1.0)`). 2.0 says the same
//! thing as the cell's own layout request, and the engine's `Grid` applies it
//! to every cell.

use std::sync::Arc;

use floom_core::{LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_widgets::Grid;

use floom_core_theme::props::grid::GridProps;
use floom_core_theme::theme::MultiTheme;

/// A builder for one cell, replayed each time the grid is built.
pub type CellBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentGridStyle {
    pub gap: f32,
    pub row_gap: f32,
    pub cols: usize,
}

/// Fluent's grid.
pub struct FluentGrid {
    props: GridProps,
    theme: RwSignal<MultiTheme>,
    children: Vec<CellBuilder>,
}

impl FluentGrid {
    pub fn new() -> Self {
        Self {
            props: GridProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            children: Vec::new(),
        }
    }

    /// Number of equal-width columns per row.
    pub fn cols(mut self, cols: usize) -> Self {
        self.props.cols = cols;
        self
    }

    /// Gap between columns within a row.
    pub fn gap(mut self, gap: f32) -> Self {
        self.props.gap = gap;
        self
    }

    /// Gap between rows.
    pub fn row_gap(mut self, row_gap: f32) -> Self {
        self.props.row_gap = row_gap;
        self
    }

    /// Add one cell to the grid (repeatable).
    pub fn child(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.children.push(Arc::new(build));
        self
    }

    pub fn props(&self) -> &GridProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentGridStyle {
        FluentGridStyle {
            gap: self.props.gap,
            row_gap: self.props.row_gap,
            cols: self.props.cols,
        }
    }

    /// The grid: rows of equal-width cells.
    pub fn view(self) -> Grid<(u64, CellBuilder)> {
        let style = self.resolved_style();
        let cols = style.cols.max(1);
        let mut children = self.children;

        // v1 padded the last row with empty cells, so that every row held the
        // same number of cells and each cell grew by a share of the row.
        while !children.is_empty() && children.len() % cols != 0 {
            children.push(Arc::new(|| Box::new(floom_widgets::Text::new(String::new()))));
        }

        // The index is the cell's key, so a row remounts only when its
        // membership changes.
        let items: Vec<(u64, CellBuilder)> = children
            .into_iter()
            .enumerate()
            .map(|(index, build)| (index as u64, build))
            .collect();

        Grid::new(cols, items, |(index, _)| *index, |(_, build)| build())
            .gap(style.gap, style.row_gap)
            // v1 wrapped every cell so it grew: `flex_grow(1.0)`.
            .item_layout(LayoutRequest::new().grow(1.0))
    }
}

impl Default for FluentGrid {
    fn default() -> Self {
        Self::new()
    }
}
