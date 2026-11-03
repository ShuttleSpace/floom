//! Cupertino table, ported to the Floom 2.0 widget API.
//!

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

/// v1's dense row factor: `row_height * 0.78`.
const DENSE_ROW_FACTOR: f32 = 0.78;
/// v1's dense header factor: `header_height * 0.82`.
const DENSE_HEADER_FACTOR: f32 = 0.82;

/// The table's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoTableStyle {
    /// The design system this kit is. v1 had no style at all.
    pub design_system: &'static str,
    /// The plain rows' fill.
    pub surface: Color,
    /// The header's fill, and the striped rows' fill.
    pub surface_variant: Color,
    /// The header cells' ink.
    pub on_surface: Color,
    /// The body cells' ink.
    pub on_surface_variant: Color,
    /// The frame's outline, and the row rules.
    pub outline: Color,
    /// The header's height, after the dense factor.
    pub header_height: f32,
    /// The body rows' height, after the dense factor.
    pub row_height: f32,
    /// The inset inside every cell.
    pub cell_padding: f32,
    /// The frame's outline thickness, and the row rules' thickness.
    pub border_width: f32,
    /// The frame's corner radius.
    pub border_radius: f32,
    /// The body cells' font size.
    pub font_size: f32,
    /// The header cells' font size.
    pub header_font_size: f32,
    /// The table's minimum width.
    pub min_width: f32,
    /// The header cells' weight: the typography scale's `label.large`.
    pub header_weight: FontWeight,
    /// Whether the table is dense.
    pub dense: bool,
    /// Whether the rows are striped.
    pub striped: bool,
}

/// Cupertino's table: a header row and body rows in a shared grid.
pub struct CupertinoTable {
    props: TableProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoTable {
    pub fn new() -> Self {
        Self {
            props: TableProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
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

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoTableStyle {
        let theme = self.theme.get();
        let ComponentTokens::Table(tokens) = theme.component(ComponentId::Table) else {
            unreachable!("ComponentId::Table must resolve to TableTokens")
        };
        let dense = self.props.dense;
        CupertinoTableStyle {
            design_system: "cupertino",
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
            // v1 read the header's weight from the *typography* scale, not from
            // `TableTokens`: `typography().label.large.font_weight`.
            header_weight: to_engine_weight(theme.typography().label.large.font_weight),
            dense,
            striped: self.props.striped,
        }
    }

    /// The table: the frame, the header row, and the body rows.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let headers = self.props.headers.clone();
        let rows = self.props.rows.clone();
        let striped = self.props.striped;
        let padding = Insets::uniform(f64::from(style.cell_padding));

        // v1's `list(..)`: the header row first, then the body rows, with no
        let mut body = Stack::new(Axis::Vertical).gap(0.0);

        // The header row. v1 gave it the surface-variant fill and *no* rule
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

        // The body rows.
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
            // v1's `border_bottom(border_width).border_color(outline)`.
            let rule = Some((style.border_width, to_paint(style.outline)));
            let row_view = row_surface(cells, style.row_height, to_paint(fill), rule);
            body = body.push(move || Box::new(row_view.clone()));
        }

        // The frame: v1's `Container::new(list(..))` with an outline, a radius
        Surface::new()
            .request(LayoutRequest::new().min_width(style.min_width))
            .border(style.border_width, to_paint(style.outline))
            .radius(style.border_radius)
            .push_layout(LayoutRequest::new().grow(1.0), move || {
                Box::new(body.clone())
            })
    }
}

impl Default for CupertinoTable {
    fn default() -> Self {
        Self::new()
    }
}

/// One cell: the padded box that holds one piece of text.
///
/// v1's cell was a `Label` with `flex_grow(1.0)` and `padding(cell_padding)`.
/// 2.0's `Text` is a leaf, and a leaf's padding does not inset the text --
/// the text is drawn at the box's own origin -- so the padding lives on the
/// `Stack` that holds the text, and that stack is the cell. The `grow(1.0)`
/// that made every cell an equal share of the row lives on the *slot*, which
/// is where 2.0 reads a child's flex factor from.
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

/// One row: the filled box, its height, its rule, and the cells inside it.
///
/// v1's row was a horizontal stack of labels with a height, a fill, and a
/// bottom rule. The fill and the rule are decoration, so they live on a
/// `Surface`; the cells are the surface's own children, laid out in a row
/// because that is the surface's own axis.
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
                // v1's `items_center()`.
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

/// The theme's weight enum -> the engine's.
///
/// v1 called `to_floom()`, which returns a `parley::style::FontWeight` -- a
/// third type with the same name. 2.0's `Text::weight` takes the engine's own
/// `FontWeight`, so the nine variants are named here.
pub fn to_engine_weight(w: ThemeFontWeight) -> FontWeight {
    match w {
        ThemeFontWeight::Thin => FontWeight::Thin,
        ThemeFontWeight::ExtraLight => FontWeight::ExtraLight,
        ThemeFontWeight::Light => FontWeight::Light,
        // v1's name for 400 is `NORMAL`; the theme's is `Regular`.
        ThemeFontWeight::Regular => FontWeight::Normal,
        ThemeFontWeight::Medium => FontWeight::Medium,
        ThemeFontWeight::SemiBold => FontWeight::SemiBold,
        ThemeFontWeight::Bold => FontWeight::Bold,
        ThemeFontWeight::ExtraBold => FontWeight::ExtraBold,
        // v1's `to_floom()` sent `Black` to 800; parley's table has no 900.
        ThemeFontWeight::Black => FontWeight::Black,
    }
}
