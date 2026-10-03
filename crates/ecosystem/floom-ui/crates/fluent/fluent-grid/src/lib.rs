use floom::prelude::*;
use floom::view::AnyView;
use floom::views::{Empty, Stack};
use floom_core_theme::props::grid::GridProps;
use floom_core_theme::theme::MultiTheme;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentGridStyle {
    pub gap: f32,
    pub row_gap: f32,
    pub cols: usize,
}

pub struct FluentGrid {
    props: GridProps,
    theme: RwSignal<MultiTheme>,
    children: Vec<AnyView>,
}

impl FluentGrid {
    pub fn new() -> Self {
        Self {
            props: GridProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Fluent)),
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
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.children.push(child.into_any());
        self
    }

    pub fn props(&self) -> &GridProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn resolved_style(&self) -> FluentGridStyle {
        FluentGridStyle {
            gap: self.props.gap,
            row_gap: self.props.row_gap,
            cols: self.props.cols,
        }
    }

    /// Build a responsive column grid. Children are wrapped into rows of `cols`,
    /// each row laying its cells out as equal-width flex items.
    pub fn view(self) -> impl View {
        let props = self.props;
        let cols = props.cols.max(1);
        let gap = props.gap;
        let row_gap = props.row_gap;
        let children = self.children;

        let mut rows: Vec<AnyView> = Vec::new();
        let mut chunk: Vec<AnyView> = Vec::new();
        for (i, c) in children.into_iter().enumerate() {
            if i > 0 && i % cols == 0 {
                rows.push(build_row(std::mem::take(&mut chunk), gap));
            }
            chunk.push(c);
        }
        if !chunk.is_empty() {
            while chunk.len() < cols {
                chunk.push(Empty::new().into_any());
            }
            rows.push(build_row(chunk, gap));
        }

        if rows.is_empty() {
            return Empty::new().into_any();
        }

        list(rows).style(move |s| s.gap(row_gap)).into_any()
    }
}

fn build_row(cells: Vec<AnyView>, gap: f32) -> AnyView {
    let cells: Vec<AnyView> = cells
        .into_iter()
        .map(|c| Container::new(c).style(|s| s.flex_grow(1.0)).into_any())
        .collect();
    Stack::horizontal_from_iter(cells)
        .style(move |s| s.gap(gap).width_pct(100.0))
        .into_any()
}

impl Default for FluentGrid {
    fn default() -> Self {
        Self::new()
    }
}
