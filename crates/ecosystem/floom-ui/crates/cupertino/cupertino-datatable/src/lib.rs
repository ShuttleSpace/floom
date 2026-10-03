use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom::reactive::RwSignal;
use floom::style::CursorStyle;
use floom::taffy::style::Overflow;
use floom::view::AnyView;
use floom::IntoView;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::datatable::{ColumnAlign, DataTableColumn, DataTableProps};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_core_theme::tokens::typography::FontWeight;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoDataTableStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub border_radius: f32,
    pub border_width: f32,
    pub min_width: f32,
    pub header_font_size: f32,
    pub font_size: f32,
    pub header_height: f32,
    pub row_height: f32,
    pub cell_padding: f32,
}

pub struct CupertinoDataTable {
    props: DataTableProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoDataTable {
    pub fn new() -> Self {
        Self {
            props: DataTableProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
        }
    }

    pub fn columns(mut self, columns: Vec<DataTableColumn>) -> Self {
        self.props.columns = columns;
        self
    }

    pub fn rows(mut self, rows: Vec<Vec<String>>) -> Self {
        self.props.rows = rows;
        self
    }

    pub fn striped(mut self, striped: bool) -> Self {
        self.props.striped = striped;
        self
    }

    pub fn dense(mut self, dense: bool) -> Self {
        self.props.dense = dense;
        self
    }

    pub fn on_row_click(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.props.on_row_click = Some(Box::new(callback));
        self
    }

    pub fn resolved_style(&self) -> CupertinoDataTableStyle {
        let theme = self.theme.get();
        let ComponentTokens::DataTable(tokens) = theme.component(ComponentId::DataTable) else {
            unreachable!("ComponentId::DataTable must resolve to DataTableTokens")
        };
        CupertinoDataTableStyle {
            design_system: "cupertino",
            surface: theme.colors().surface,
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            outline: theme.colors().outline,
            border_radius: tokens.border_radius,
            border_width: tokens.border_width,
            min_width: tokens.min_width,
            header_font_size: tokens.header_font_size,
            font_size: tokens.font_size,
            header_height: tokens.header_height,
            row_height: tokens.row_height,
            cell_padding: tokens.cell_padding,
        }
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let props = self.props;
        let on_row_click: Rc<Option<Box<dyn Fn(usize)>>> = Rc::new(props.on_row_click);

        let tokens = match theme.get().component(ComponentId::DataTable) {
            ComponentTokens::DataTable(t) => t,
            _ => unreachable!(),
        };
        let border_radius = tokens.border_radius;
        let border_width = tokens.border_width;
        let min_width = tokens.min_width;
        let header_font_size = tokens.header_font_size;
        let font_size = tokens.font_size;
        let header_height = tokens.header_height;
        let row_height = if props.dense {
            tokens.dense_row_height
        } else {
            tokens.row_height
        };
        let cell_padding = tokens.cell_padding;
        let striped = props.striped;

        let header_cells: Vec<AnyView> = props
            .columns
            .iter()
            .map(|col| {
                let a = col.align;
                Label::new(col.title.clone())
                    .style(move |s| {
                        let base = match a {
                            ColumnAlign::Start => s.justify_start(),
                            ColumnAlign::Center => s.justify_center(),
                            ColumnAlign::End => s.justify_end(),
                        };
                        let t = theme.get();
                        base.padding_horiz(cell_padding)
                            .flex_grow(1.0)
                            .color(t.colors().on_surface.to_peniko())
                            .font_size(header_font_size)
                            .font_weight(FontWeight::Medium.to_floom())
                    })
                    .into_any()
            })
            .collect();

        let header = Stack::horizontal_from_iter(header_cells).style(move |s| {
            let t = theme.get();
            s.height(header_height)
                .background(t.colors().surface_variant.to_peniko())
                .border_bottom(1.0)
                .border_color(with_alpha(t.colors().outline, 60).to_peniko())
                .items_center()
        });

        let body_rows: Vec<AnyView> = props
            .rows
            .iter()
            .enumerate()
            .map(|(r, row)| {
                let cells: Vec<AnyView> = props
                    .columns
                    .iter()
                    .zip(row.iter())
                    .map(|(col, cell)| {
                        let a = col.align;
                        Label::new(cell.clone())
                            .style(move |s| {
                                let base = match a {
                                    ColumnAlign::Start => s.justify_start(),
                                    ColumnAlign::Center => s.justify_center(),
                                    ColumnAlign::End => s.justify_end(),
                                };
                                let t = theme.get();
                                base.padding_horiz(cell_padding)
                                    .flex_grow(1.0)
                                    .color(t.colors().on_surface.to_peniko())
                                    .font_size(font_size)
                            })
                            .into_any()
                    })
                    .collect();
                let on_row_click = on_row_click.clone();
                let rh = row_height;
                Stack::horizontal_from_iter(cells)
                    .style(move |s| {
                        let t = theme.get();
                        s.height(rh)
                            .items_center()
                            .border_bottom(1.0)
                            .border_color(with_alpha(t.colors().outline, 30).to_peniko())
                            .apply_if(striped && r % 2 == 1, |s| {
                                s.background(with_alpha(t.colors().on_surface, 6).to_peniko())
                            })
                            .cursor(CursorStyle::Pointer)
                    })
                    .on_event(listener::Click, move |_, _| {
                        if let Some(cb) = on_row_click.as_ref() {
                            cb(r);
                        }
                        EventPropagation::Continue
                    })
                    .into_any()
            })
            .collect();

        let rows_view = list(
            vec![header.into_any()]
                .into_iter()
                .chain(body_rows),
        )
        .style(|s| s.gap(0.0));

        Container::new(rows_view).style(move |s| {
            let t = theme.get();
            s.min_width(min_width)
                .border(border_width)
                .border_color(t.colors().outline.to_peniko())
                .border_radius(border_radius)
                .overflow_x(Overflow::Clip)
                .overflow_y(Overflow::Clip)
                .background(t.colors().surface.to_peniko())
        })
    }
}

impl Default for CupertinoDataTable {
    fn default() -> Self {
        Self::new()
    }
}
