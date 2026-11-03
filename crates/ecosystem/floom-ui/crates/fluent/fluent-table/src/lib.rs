//! Fluent table, ported to the Floom 2.0 widget API.

use floom_core::LayoutRequest;
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, FontWeight, Insets};
use floom_widgets::{Align, Axis, Edges, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::table::TableProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::typography::FontWeight as ThemeFontWeight;

const DENSE_ROW_FACTOR: f32 = 0.78;
const DENSE_HEADER_FACTOR: f32 = 0.82;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentTableStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub surface_variant: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub header_height: f32,
    pub row_height: f32,
    pub cell_padding: f32,
    pub border_width: f32,
    pub border_radius: f32,
    pub font_size: f32,
    pub header_font_size: f32,
    pub min_width: f32,
    pub header_weight: FontWeight,
    pub dense: bool,
    pub striped: bool,
}

pub struct FluentTable {
    props: TableProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentTable {
    pub fn new() -> Self {
        Self {
            props: TableProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
        }
    }

    pub fn headers(mut self, headers: Vec<String>) -> Self {
        self.props.headers = headers;
        self
    }

    pub fn rows(mut self, rows: Vec<Vec<String>>) -> Self {
        self.props.rows = rows;
        self
    }

    pub fn dense(mut self, dense: bool) -> Self {
        self.props.dense = dense;
        self
    }

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

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentTableStyle {
        let theme = self.theme.get();
        let ComponentTokens::Table(tokens) = theme.component(ComponentId::Table) else {
            unreachable!("ComponentId::Table must resolve to TableTokens")
        };
        let dense = self.props.dense;
        FluentTableStyle {
            design_system: "fluent",
            surface: theme.colors().surface,
            surface_variant: theme.colors().surface_variant,
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            outline: theme.colors().outline,
            header_height: if dense {
                tokens.header_height * DENSE_HEADER_FACTOR
            } else {
                tokens.header_height
            },
            row_height: if dense {
                tokens.row_height * DENSE_ROW_FACTOR
            } else {
                tokens.row_height
            },
            cell_padding: tokens.cell_padding,
            border_width: tokens.border_width,
            border_radius: tokens.border_radius,
            font_size: tokens.font_size,
            header_font_size: tokens.header_font_size,
            min_width: tokens.min_width,
            header_weight: to_engine_weight(theme.typography().label.large.font_weight),
            dense,
            striped: self.props.striped,
        }
    }

    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let headers = self.props.headers.clone();
        let rows = self.props.rows.clone();
        let striped = self.props.striped;
        let padding = Insets::uniform(f64::from(style.cell_padding));

        let mut body = Stack::new(Axis::Vertical).gap(0.0);

        let header_cells: Vec<(LayoutRequest, Stack)> = headers
            .iter()
            .map(|header| {
                cell(
                    header,
                    style.header_font_size,
                    style.header_weight,
                    to_paint(style.on_surface),
                    padding,
                )
            })
            .collect();
        let header = row_surface(
            header_cells,
            style.header_height,
            to_paint(style.surface_variant),
            None,
        );
        body = body.push(move || Box::new(header.clone()));

        for (index, values) in rows.iter().enumerate() {
            let is_stripe = striped && index % 2 == 1;
            let fill = if is_stripe {
                style.surface_variant
            } else {
                style.surface
            };
            let cells: Vec<(LayoutRequest, Stack)> = values
                .iter()
                .map(|value| {
                    cell(
                        value,
                        style.font_size,
                        FontWeight::Normal,
                        to_paint(style.on_surface_variant),
                        padding,
                    )
                })
                .collect();
            let rule = Some((style.border_width, to_paint(style.outline)));
            let row_view = row_surface(cells, style.row_height, to_paint(fill), rule);
            body = body.push(move || Box::new(row_view.clone()));
        }

        Surface::new()
            .request(LayoutRequest::new().min_width(style.min_width))
            .border(style.border_width, to_paint(style.outline))
            .radius(style.border_radius)
            .push_layout(LayoutRequest::new().grow(1.0), move || {
                Box::new(body.clone())
            })
    }
}

impl Default for FluentTable {
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
) -> (LayoutRequest, Stack) {
    let text = text.to_string();
    let layout = LayoutRequest::new().grow(1.0).padding(padding);
    let widget = Stack::new(Axis::Horizontal)
        .push(move || Box::new(Text::new(text.clone()).size(size).weight(weight).color(ink)));
    (layout, widget)
}

fn row_surface(
    cells: Vec<(LayoutRequest, Stack)>,
    height: f32,
    fill: PaintColor,
    rule: Option<(f32, PaintColor)>,
) -> Surface {
    let mut row = Surface::new()
        .request(
            LayoutRequest::new()
                .height(height)
                .stack(Axis::Horizontal, 0.0)
                .cross_align(Align::Center),
        )
        .background(fill);
    if let Some((width, colour)) = rule {
        row = row.border_edges(width, colour, Edges::BOTTOM);
    }
    for (layout, cell) in cells {
        row = row.push_layout(layout, move || Box::new(cell.clone()));
    }
    row
}

pub fn to_engine_weight(w: ThemeFontWeight) -> FontWeight {
    match w {
        ThemeFontWeight::Thin => FontWeight::Thin,
        ThemeFontWeight::ExtraLight => FontWeight::ExtraLight,
        ThemeFontWeight::Light => FontWeight::Light,
        ThemeFontWeight::Regular => FontWeight::Normal,
        ThemeFontWeight::Medium => FontWeight::Medium,
        ThemeFontWeight::SemiBold => FontWeight::SemiBold,
        ThemeFontWeight::Bold => FontWeight::Bold,
        ThemeFontWeight::ExtraBold => FontWeight::ExtraBold,
        ThemeFontWeight::Black => FontWeight::Black,
    }
}
