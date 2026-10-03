use floem::prelude::*;
use floem::view::AnyView;
use floem::views::{Decorators, list, h_stack_from_iter, Stack, Label};
use floem::reactive::{RwSignal, SignalGet};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::table::TableProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

/// Material 3 simple table: a header row plus body rows laid out in a shared
/// grid. Each cell flexes to equal width; rows may be striped and/or dense.
pub struct MdTable {
    props: TableProps,
    theme: RwSignal<MultiTheme>,
}

impl MdTable {
    pub fn new() -> Self {
        Self {
            props: TableProps::default(),
            theme: theme_signal(),
        }
    }

    /// Column header labels (defines the number of columns).
    pub fn headers(mut self, headers: Vec<String>) -> Self {
        self.props.headers = headers;
        self
    }

    /// Body rows; each inner vector aligns positionally with `headers`.
    pub fn rows(mut self, rows: Vec<Vec<String>>) -> Self {
        self.props.rows = rows;
        self
    }

    /// Compact row height.
    pub fn dense(mut self, dense: bool) -> Self {
        self.props.dense = dense;
        self
    }

    /// Alternate row background for readability.
    pub fn striped(mut self, striped: bool) -> Self {
        self.props.striped = striped;
        self
    }

    pub fn props(&self) -> &TableProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let tokens = match theme.get().component(ComponentId::Table) {
            ComponentTokens::Table(t) => t,
            _ => unreachable!(),
        };
        let header_height = tokens.header_height;
        let row_height = tokens.row_height;
        let cell_padding = tokens.cell_padding;
        let border_width = tokens.border_width;
        let border_radius = tokens.border_radius;
        let font_size = tokens.font_size;
        let header_font_size = tokens.header_font_size;
        let min_width = tokens.min_width;
        let dense = self.props.dense;
        let striped = self.props.striped;
        let headers = self.props.headers.clone();
        let rows = self.props.rows.clone();

        let rh = if dense { row_height * 0.78 } else { row_height };
        let hh = if dense {
            header_height * 0.82
        } else {
            header_height
        };

        let header_cells: Vec<AnyView> = headers
            .iter()
            .map(|h| {
                Label::new(h.clone())
                    .style(move |s| {
                        let t = theme.get();
                        s.flex_grow(1.0)
                            .padding(cell_padding)
                            .font_size(header_font_size)
                            .font_weight(t.typography().label.large.font_weight.to_floem())
                            .color(t.colors().on_surface.to_peniko())
                    })
                    .into_any()
            })
            .collect();
        let header_row = Stack::horizontal_from_iter(header_cells).style(move |s| {
            let t = theme.get();
            s.height(hh)
                .background(t.colors().surface_variant.to_peniko())
                .items_center()
        });

        let mut all_rows: Vec<AnyView> = Vec::new();
        all_rows.push(header_row.into_any());
        for (ri, row) in rows.iter().enumerate() {
            let cells: Vec<AnyView> = row
                .iter()
                .map(|c| {
                    Label::new(c.clone())
                        .style(move |s| {
                            let t = theme.get();
                            s.flex_grow(1.0)
                                .padding(cell_padding)
                                .font_size(font_size)
                                .color(t.colors().on_surface_variant.to_peniko())
                        })
                        .into_any()
                })
                .collect();
            let is_stripe = striped && ri % 2 == 1;
            let row_view = Stack::horizontal_from_iter(cells).style(move |s| {
                let t = theme.get();
                let bg = if is_stripe {
                    t.colors().surface_variant
                } else {
                    t.colors().surface
                };
                s.height(rh)
                    .background(bg.to_peniko())
                    .items_center()
                    .border_bottom(border_width)
                    .border_color(t.colors().outline.to_peniko())
            });
            all_rows.push(row_view.into_any());
        }

        Container::new(list(all_rows)).style(move |s| {
            let t = theme.get();
            s.border(border_width)
                .border_color(t.colors().outline.to_peniko())
                .border_radius(border_radius)
                .min_width(min_width)
        })
    }
}

impl Default for MdTable {
    fn default() -> Self {
        Self::new()
    }
}
