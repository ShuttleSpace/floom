//! Fluent data table, ported to the Floom 2.0 widget API.

use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, FontWeight, Insets};
use floom_widgets::{Align, Axis, Edges, Pressable, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::datatable::{ColumnAlign, DataTableColumn, DataTableProps};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

const DIVIDER_WIDTH: f32 = 1.0;
const HEADER_DIVIDER_ALPHA: u8 = 60;
const ROW_DIVIDER_ALPHA: u8 = 30;
const STRIPE_ALPHA: u8 = 6;

type RowCallback = Arc<dyn Fn(usize) + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentDataTableStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub surface_variant: Color,
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
    pub dense_row_height: f32,
    pub cell_padding: f32,
    pub header_divider: Color,
    pub row_divider: Color,
    pub stripe: Color,
    pub divider_width: f32,
    pub striped: bool,
    pub dense: bool,
}

pub struct FluentDataTable {
    props: DataTableProps,
    theme: RwSignal<MultiTheme>,
    on_row_click: Option<RowCallback>,
}

impl FluentDataTable {
    pub fn new() -> Self {
        Self {
            props: DataTableProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            on_row_click: None,
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

    pub fn on_row_click(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_row_click = Some(Arc::new(callback));
        self
    }

    pub fn props(&self) -> &DataTableProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentDataTableStyle {
        let theme = self.theme.get();
        let ComponentTokens::DataTable(tokens) = theme.component(ComponentId::DataTable) else {
            unreachable!("ComponentId::DataTable must resolve to DataTableTokens")
        };
        FluentDataTableStyle {
            design_system: "fluent",
            surface: theme.colors().surface,
            surface_variant: theme.colors().surface_variant,
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
            dense_row_height: tokens.dense_row_height,
            cell_padding: tokens.cell_padding,
            header_divider: with_alpha(theme.colors().outline, HEADER_DIVIDER_ALPHA),
            row_divider: with_alpha(theme.colors().outline, ROW_DIVIDER_ALPHA),
            stripe: with_alpha(theme.colors().on_surface, STRIPE_ALPHA),
            divider_width: DIVIDER_WIDTH,
            striped: self.props.striped,
            dense: self.props.dense,
        }
    }

    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let columns = self.props.columns.clone();
        let rows = self.props.rows.clone();
        let striped = self.props.striped;
        let on_row_click = self.on_row_click.clone();

        let row_height = if self.props.dense {
            style.dense_row_height
        } else {
            style.row_height
        };
        let padding = Insets {
            left: f64::from(style.cell_padding),
            right: f64::from(style.cell_padding),
            top: 0.0,
            bottom: 0.0,
        };
        let ink = to_paint(style.on_surface);

        let mut body = Stack::new(Axis::Vertical).gap(0.0);

        let header_cells: Vec<(LayoutRequest, Stack)> = columns
            .iter()
            .map(|col| {
                cell(
                    &col.title,
                    style.header_font_size,
                    FontWeight::Medium,
                    ink,
                    padding,
                    align_of(col.align),
                )
            })
            .collect();
        let header = row_surface(
            header_cells,
            style.header_height,
            Some(to_paint(style.surface_variant)),
            (style.divider_width, to_paint(style.header_divider)),
        );
        body = body.push(move || Box::new(header.clone()));

        for (index, values) in rows.iter().enumerate() {
            let is_stripe = striped && index % 2 == 1;
            let fill = is_stripe.then(|| to_paint(style.stripe));
            let cells: Vec<(LayoutRequest, Stack)> = columns
                .iter()
                .zip(values.iter())
                .map(|(col, value)| {
                    cell(
                        value,
                        style.font_size,
                        FontWeight::Normal,
                        ink,
                        padding,
                        align_of(col.align),
                    )
                })
                .collect();
            let row = row_surface(
                cells,
                row_height,
                fill,
                (style.divider_width, to_paint(style.row_divider)),
            );
            let on_row_click = on_row_click.clone();
            body = body.push(move || {
                let row = row.clone();
                let mut pressable = Pressable::new(Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new(),
                    move || Box::new(row.clone()),
                ));
                if let Some(callback) = on_row_click.clone() {
                    pressable = pressable.on_click(move || callback(index));
                }
                Box::new(pressable)
            });
        }

        Surface::new()
            .request(LayoutRequest::new().min_width(style.min_width))
            .background(to_paint(style.surface))
            .border(style.border_width, to_paint(style.outline))
            .radius(style.border_radius)
            .push_layout(LayoutRequest::new().grow(1.0), move || {
                Box::new(body.clone())
            })
    }
}

impl Default for FluentDataTable {
    fn default() -> Self {
        Self::new()
    }
}

fn cell(
    text: &str,
    size: f32,
    weight: FontWeight,
    ink: PaintColor,
    padding: Insets,
    align: Align,
) -> (LayoutRequest, Stack) {
    let text = text.to_string();
    let layout = LayoutRequest::new()
        .grow(1.0)
        .padding(padding)
        .main_align(align);
    let widget = Stack::new(Axis::Horizontal)
        .push(move || Box::new(Text::new(text.clone()).size(size).weight(weight).color(ink)));
    (layout, widget)
}

fn row_surface(
    cells: Vec<(LayoutRequest, Stack)>,
    height: f32,
    fill: Option<PaintColor>,
    rule: (f32, PaintColor),
) -> Surface {
    let mut row = Surface::new().request(
        LayoutRequest::new()
            .height(height)
            .stack(Axis::Horizontal, 0.0)
            .cross_align(Align::Center),
    );
    if let Some(fill) = fill {
        row = row.background(fill);
    }
    row = row.border_edges(rule.0, rule.1, Edges::BOTTOM);
    for (layout, cell) in cells {
        row = row.push_layout(layout, move || Box::new(cell.clone()));
    }
    row
}

fn align_of(align: ColumnAlign) -> Align {
    match align {
        ColumnAlign::Start => Align::Start,
        ColumnAlign::Center => Align::Center,
        ColumnAlign::End => Align::End,
    }
}
