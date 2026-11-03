
use std::sync::Arc;

use floom_core::{LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_widgets::Grid;

use floom_core_theme::props::grid::GridProps;
use floom_core_theme::theme::MultiTheme;
use floom_widget_runtime::theme_signal;

/// A builder for one cell, replayed each time the grid is built.
pub type CellBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialGridStyle {
    pub gap: f32,
    pub row_gap: f32,
    pub cols: usize,
}

/// Material's grid.
pub struct MaterialGrid {
    props: GridProps,
    theme: RwSignal<MultiTheme>,
    children: Vec<CellBuilder>,
}

impl MaterialGrid {
    pub fn new() -> Self {
        Self {
            props: GridProps::default(),
            theme: theme_signal(),
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

    pub fn resolved_style(&self) -> MaterialGridStyle {
        MaterialGridStyle {
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

        while !children.is_empty() && children.len() % cols != 0 {
            children.push(Arc::new(|| {
                Box::new(floom_widgets::Text::new(String::new()))
            }));
        }

        let items: Vec<(u64, CellBuilder)> = children
            .into_iter()
            .enumerate()
            .map(|(index, build)| (index as u64, build))
            .collect();

        Grid::new(cols, items, |(index, _)| *index, |(_, build)| build())
            .gap(style.gap, style.row_gap)
            .item_layout(LayoutRequest::new().grow(1.0))
    }
}

impl Default for MaterialGrid {
    fn default() -> Self {
        Self::new()
    }
}
