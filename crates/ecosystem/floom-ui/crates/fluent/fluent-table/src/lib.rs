//! Fluent table, ported to the Floom 2.0 widget API.
//!
//! A table is a header row plus body rows in a shared grid: every cell takes
//! an equal share of the row's width, the rows may be striped, and the whole
//! table may be dense.
//!
//! ## What the port uses
//!
//! The engine's vocabulary, three deep:
//!
//! * a [`Surface`] for the table's own frame -- the outline and the corner
//!   radius v1 put on the container;
//! * a vertical `Stack` for the rows;
//! * a [`Surface`] per row -- the row's fill and its bottom rule -- holding
//!   the cells directly, each cell a `Stack` that pads the [`Text`] it holds.
//!
//! The cell is a box rather than a bare text because 2.0's `Text` is a leaf:
//! a leaf's padding does not inset the text, which is drawn at the box's own
//! origin. v1's cell was a `Label` with `padding(cell_padding)`, and the text
//! sat inside that padding; here the padding lives on the box that holds the
//! text, which is the same drawing.
//!
//! There is nothing here that a widget could own and the vocabulary does not
//! already say.
//!
//! ## What this kit answers
//!
//! * the header's and the rows' heights, the cells' padding, the frame's
//!   border width and radius, the minimum width and the two font sizes: the
//!   tokens (`TableTokens`);
//! * the header's fill: `surface_variant`; the plain rows': `surface`; the
//!   striped rows': `surface_variant`;
//! * the header cells' ink: `on_surface`; the body cells': `on_surface_variant`;
//! * the frame and the row rules: `outline`;
//! * the header cells' weight: the typography scale's `label.large`.
//!
//! ## The dense factors v1 wrote as literals
//!
//! v1 shrank a dense row to `row_height * 0.78` and a dense header to
//! `header_height * 0.82`. The port does the same, and the style reports the
//! heights the table is actually drawn at -- which is what v1's paint did and
//! what v1's (absent) style could not say.
//!
//! ## The style v1 did not have
//!
//! v1's table had no style struct at all: the tokens were read inside
//! `view()` and never reported. The port's style reports them, because a
//! component whose numbers cannot be read is a component that cannot be
//! tested against a theme.
//!
//! ## The theme
//!
//! v1's Fluent table took a *snapshot* of its own design system rather than
//! reading the app-wide signal, so a change of theme does not reach it. That
//! difference is v1's, and it is kept.
//!
//! The snapshot names a design system it does not pin: `MultiTheme` reads
//! the *current* design system at read time, so a Fluent table in an app
//! whose active design system is Material shows Material's colours. That is
//! v1's behaviour too -- the same `MultiTheme` v1 snapshotted -- and it is
//! kept rather than papered over.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The header has no rule under it.** v1 put `border_bottom` on the body
//!   rows only, so the header sits directly on the frame's own outline. A
//!   table whose header is separated from the body by a rule is a different
//!   component, so the port keeps v1's answer.
//! * **The frame has no fill.** v1's container carried the outline and the
//!   radius and no background; the rows supply the colour. The port's frame
//!   is a `Surface` with no `background(..)` for the same reason.
//! * **The accessors.** v1's table had `props()` and `theme()` but no
//!   `resolved_style()` and no `with_theme()`. The port gives all three kits
//!   the same three, because a kit that cannot be given a theme cannot be
//!   tested against one.

use floom_core::LayoutRequest;
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, FontWeight, Insets};
use floom_widgets::{Align, Axis, Edges, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::table::TableProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::typography::FontWeight as ThemeFontWeight;

/// v1's dense row factor: `row_height * 0.78`.
const DENSE_ROW_FACTOR: f32 = 0.78;
/// v1's dense header factor: `header_height * 0.82`.
const DENSE_HEADER_FACTOR: f32 = 0.82;

/// The table's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentTableStyle {
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

/// Fluent's table: a header row and body rows in a shared grid.
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
            // v1's dense factors, applied to the tokens.
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
        // gap between them.
        let mut body = Stack::new(Axis::Vertical).gap(0.0);

        // The header row. v1 gave it the surface-variant fill and *no* rule
        // under it.
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
        // and a minimum width -- and no fill of its own.
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
    let widget = Stack::new(Axis::Horizontal).push(move || {
        Box::new(Text::new(text.clone()).size(size).weight(weight).color(ink))
    });
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

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
