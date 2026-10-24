//! Material 3 data table, ported to the Floom 2.0 widget API.
//!
//! A data table is a table whose columns carry an alignment and whose rows
//! answer a click: a header row, body rows in a shared grid, and a callback
//! that says which row was pressed.
//!
//! ## What the port uses
//!
//! The engine's vocabulary, three deep:
//!
//! * a [`Surface`] for the table's own frame -- the fill, the outline and the
//!   corner radius v1 put on the container;
//! * a vertical `Stack` for the rows;
//! * a [`Surface`] per row -- the row's fill and its rule -- holding the
//!   cells directly, each cell a `Stack` that pads the [`Text`] it holds.
//!
//! The clickable row is the engine's [`Pressable`], which is the half of v1's
//! `Decorators::on_event` that 2.0 kept: a wrapper that makes its child act
//! as one clickable surface.
//!
//! The cell is a box rather than a bare text because 2.0's `Text` is a leaf:
//! a leaf's padding does not inset the text, which is drawn at the box's own
//! origin. v1's cell was a `Label` with `padding_horiz(cell_padding)`, and
//! the text sat inside that padding; here the padding lives on the box that
//! holds the text, which is the same drawing.
//!
//! There is nothing here that a widget could own and the vocabulary does not
//! already say.
//!
//! ## What this kit answers
//!
//! * the header's height, the rows' heights, the cells' padding, the frame's
//!   border width and radius, the minimum width and the two font sizes: the
//!   tokens (`DataTableTokens`);
//! * the frame's fill: `surface`; the header's: `surface_variant`;
//! * the cells' ink: `on_surface` -- both the header's and the body cells';
//! * the frame's outline: `outline`;
//! * the rules: the outline at 60 under the header and at 30 under the rows;
//! * the striped rows' fill: the on-surface at 6.
//!
//! ## The three things v1's style did not say
//!
//! * **The dense row's height.** v1's `resolved_style()` reported the token's
//!   `row_height` even in dense mode, where the rows were drawn at
//!   `dense_row_height`. The port reports both: `row_height` is still what
//!   v1's style said, and `dense_row_height` is what the dense rows are
//!   actually drawn at.
//! * **The rules.** v1 wrote the rules' colours at the call site --
//!   `with_alpha(outline, 60)` under the header and `with_alpha(outline, 30)`
//!   under the rows -- and the style never reported them. The port's does.
//! * **`on_surface_variant`.** v1's style reported it, and v1's paint never
//!   used it: the data table's cells are all `on_surface`. The port keeps the
//!   field, because a style that drops a field v1 had is a style that cannot
//!   be compared with v1's, and names it here.
//!
//! ## The numbers v1 wrote as literals
//!
//! The rules' thickness is `1.0` -- v1 wrote `border_bottom(1.0)` rather
//! than the token's `border_width`, which happens to be 1.0 as well. The
//! alphas (60, 30 and 6) are v1's too.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The overflow clip.** v1 wrote `overflow_x(Clip).overflow_y(Clip)` on
//!   the frame. 2.0's `Surface` has no overflow clip -- `Widget::clip` is a
//!   method a widget answers for itself, and the decoration boxes do not --
//!   so the port's table does not clip. The `min_width` still applies, and a
//!   row wider than the frame draws past its edge.
//! * **The pointer cursor.** v1 put `cursor(CursorStyle::Pointer)` on every
//!   row. 2.0's `Pressable` does not answer `Widget::cursor`, so the rows
//!   show the default arrow. The row is still clickable -- and, unlike v1's,
//!   it is also reachable from the keyboard.
//! * **The callback in the props.** v1's `DataTableProps::on_row_click` is a
//!   `Box<dyn Fn(usize)>`, which is not shareable between threads -- and
//!   `DataTableProps::clone` drops it anyway. 2.0's slots are shared, so the
//!   kit keeps its own `Arc<dyn Fn(usize) + Send + Sync>` and the props'
//!   slot stays empty: `props().on_row_click` is `None` where v1's was
//!   `Some`.
//!
//! ## The theme
//!
//! v1's Material data table read the *app-wide* theme signal, so a change of
//! theme re-themes it with everything else. The Cupertino and Fluent copies
//! took a snapshot of their own design system instead, and still do. That
//! difference is v1's, and it is kept.
//!
//! ## The accessors
//!
//! v1's data table had `resolved_style()` but no `props()`, no `theme()` and
//! no `with_theme()`. The port gives all three kits the same three, because
//! a kit that cannot be given a theme cannot be tested against one.

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
use floom_widget_runtime::theme_signal;

/// The rule's thickness. v1 wrote `1.0` rather than the token's border width.
const DIVIDER_WIDTH: f32 = 1.0;
/// The header's rule alpha. v1's `with_alpha(outline, 60)`.
const HEADER_DIVIDER_ALPHA: u8 = 60;
/// The rows' rule alpha. v1's `with_alpha(outline, 30)`.
const ROW_DIVIDER_ALPHA: u8 = 30;
/// The striped rows' fill alpha. v1's `with_alpha(on_surface, 6)`.
const STRIPE_ALPHA: u8 = 6;

/// A callback in the shape 2.0's slots need.
type RowCallback = Arc<dyn Fn(usize) + Send + Sync + 'static>;

/// The data table's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialDataTableStyle {
    pub design_system: &'static str,
    /// The frame's fill.
    pub surface: Color,
    /// The header row's fill. v1's `surface_variant`.
    pub surface_variant: Color,
    /// The cells' ink -- the header's and the body cells'.
    pub on_surface: Color,
    /// v1's style reported this; v1's paint never used it.
    pub on_surface_variant: Color,
    /// The frame's outline.
    pub outline: Color,
    pub border_radius: f32,
    pub border_width: f32,
    pub min_width: f32,
    pub header_font_size: f32,
    pub font_size: f32,
    pub header_height: f32,
    /// The token's row height -- what v1's style reported, even in dense mode.
    pub row_height: f32,
    /// The height a dense row is drawn at. v1's style never reported it.
    pub dense_row_height: f32,
    pub cell_padding: f32,
    /// The rule under the header: the outline at 60.
    pub header_divider: Color,
    /// The rule under a row: the outline at 30.
    pub row_divider: Color,
    /// The striped rows' fill: the on-surface at 6.
    pub stripe: Color,
    /// The rules' thickness. v1 wrote `1.0` rather than the token's width.
    pub divider_width: f32,
    pub striped: bool,
    pub dense: bool,
}

/// Material's data table: a header row, body rows, and a click on a row.
pub struct MaterialDataTable {
    props: DataTableProps,
    theme: RwSignal<MultiTheme>,
    /// The row callback in the shape 2.0's slots need.
    on_row_click: Option<RowCallback>,
}

impl MaterialDataTable {
    pub fn new() -> Self {
        Self {
            props: DataTableProps::default(),
            theme: theme_signal(),
            on_row_click: None,
        }
    }

    /// The columns: their titles and their alignments.
    pub fn columns(mut self, columns: Vec<DataTableColumn>) -> Self {
        self.props.columns = columns;
        self
    }

    /// The body rows; each inner vector aligns positionally with `columns`.
    pub fn rows(mut self, rows: Vec<Vec<String>>) -> Self {
        self.props.rows = rows;
        self
    }

    /// Alternate row background for readability.
    pub fn striped(mut self, striped: bool) -> Self {
        self.props.striped = striped;
        self
    }

    /// Tighter row spacing.
    pub fn dense(mut self, dense: bool) -> Self {
        self.props.dense = dense;
        self
    }

    /// Called with the row index when a row is clicked.
    ///
    /// The callback has to be `Send + Sync`: 2.0's slots are shared between
    /// threads. v1's `DataTableProps::on_row_click` is a `Box<dyn Fn(usize)>`,
    /// which is not -- and `DataTableProps::clone` drops it anyway -- so the
    /// kit keeps its own handle and the props' slot stays empty.
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

    pub fn resolved_style(&self) -> MaterialDataTableStyle {
        let theme = self.theme.get();
        let ComponentTokens::DataTable(tokens) = theme.component(ComponentId::DataTable) else {
            unreachable!("ComponentId::DataTable must resolve to DataTableTokens")
        };
        MaterialDataTableStyle {
            design_system: "material",
            surface: theme.colors().surface,
            // v1's header row was filled with the surface variant.
            surface_variant: theme.colors().surface_variant,
            on_surface: theme.colors().on_surface,
            // v1's style reported this and v1's paint never used it.
            on_surface_variant: theme.colors().on_surface_variant,
            outline: theme.colors().outline,
            border_radius: tokens.border_radius,
            border_width: tokens.border_width,
            min_width: tokens.min_width,
            header_font_size: tokens.header_font_size,
            font_size: tokens.font_size,
            header_height: tokens.header_height,
            // v1 reported the token's row height even in dense mode, where the
            // rows were drawn at `dense_row_height`.
            row_height: tokens.row_height,
            dense_row_height: tokens.dense_row_height,
            cell_padding: tokens.cell_padding,
            // v1 wrote the rules' colours at the call site.
            header_divider: with_alpha(theme.colors().outline, HEADER_DIVIDER_ALPHA),
            row_divider: with_alpha(theme.colors().outline, ROW_DIVIDER_ALPHA),
            stripe: with_alpha(theme.colors().on_surface, STRIPE_ALPHA),
            divider_width: DIVIDER_WIDTH,
            striped: self.props.striped,
            dense: self.props.dense,
        }
    }

    /// The table: the frame, the header row, and the clickable body rows.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let columns = self.props.columns.clone();
        let rows = self.props.rows.clone();
        let striped = self.props.striped;
        let on_row_click = self.on_row_click.clone();

        // v1's dense switch, at the call site.
        let row_height = if self.props.dense {
            style.dense_row_height
        } else {
            style.row_height
        };
        // v1's `padding_horiz(cell_padding)`.
        let padding = Insets {
            left: f64::from(style.cell_padding),
            right: f64::from(style.cell_padding),
            top: 0.0,
            bottom: 0.0,
        };
        let ink = to_paint(style.on_surface);

        // v1's `list(vec![header].chain(body_rows))`: the header first, then
        // the body rows, with no gap between them.
        let mut body = Stack::new(Axis::Vertical).gap(0.0);

        // The header row: the surface variant, and a rule under it.
        let header_cells: Vec<(LayoutRequest, Stack)> = columns
            .iter()
            .map(|col| {
                cell(
                    &col.title,
                    style.header_font_size,
                    // v1's header was medium weight.
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
            // v1's header row was filled with the surface variant.
            Some(to_paint(style.surface_variant)),
            (style.divider_width, to_paint(style.header_divider)),
        );
        body = body.push(move || Box::new(header.clone()));

        // The body rows, each one clickable.
        for (index, values) in rows.iter().enumerate() {
            let is_stripe = striped && index % 2 == 1;
            // v1's `apply_if(striped && r % 2 == 1, ..)`: only a striped row has
            // a fill of its own.
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
                // The `Pressable` is built here rather than moved in: it is
                // not `Clone`, and its callback is a `Box<dyn Fn()>` that has
                // to be made where the row is.
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

        // The frame: v1's `Container::new(rows_view)` with a fill, an outline,
        // a radius and a minimum width.
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

impl Default for MaterialDataTable {
    fn default() -> Self {
        Self::new()
    }
}

/// One cell: the padded box that holds one piece of text.
///
/// v1's cell was a `Label` with `flex_grow(1.0)`, `padding_horiz(..)` and a
/// `justify_*`. 2.0's `Text` is a leaf, and a leaf's padding does not inset
/// the text -- the text is drawn at the box's own origin -- so the padding
/// lives on the `Stack` that holds the text, and that stack is the cell. The
/// `grow(1.0)` that made every cell an equal share of the row, and the
/// alignment, live on the *slot*, which is where 2.0 reads a child's flex
/// factor and alignment from.
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

/// One row: the filled box, its height, its rule, and the cells inside it.
///
/// v1's row was a horizontal stack of labels with a height, a rule, and -- on
/// a striped row -- a fill. The fill and the rule are decoration, so they
/// live on a `Surface`; the cells are the surface's own children, laid out in
/// a row because that is the surface's own axis.
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
            // v1's `items_center()`.
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

/// A column's alignment, as the engine's main-axis alignment.
fn align_of(align: ColumnAlign) -> Align {
    match align {
        ColumnAlign::Start => Align::Start,
        ColumnAlign::Center => Align::Center,
        ColumnAlign::End => Align::End,
    }
}
