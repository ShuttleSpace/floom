#![cfg(feature = "datatable")]
use std::sync::{Arc, Mutex};

use floom_core::{Child, ElementId, Point, Rect, Widget, WidgetAny};
use floom_types::{Color, FontWeight, Insets, PointerEvent};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder, TestEventContext};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::datatable::{ColumnAlign, DataTableColumn};
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_material::widgets::datatable::{MaterialDataTable, MaterialDataTableStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 400.0,
};

const ROW: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 52.0,
};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn kit() -> MaterialDataTable {
    MaterialDataTable::new().with_theme(theme())
}

fn tokens() -> floom_core_theme::component_tokens::DataTableTokens {
    match theme().component(floom_core_theme::component_tokens::ComponentId::DataTable) {
        floom_core_theme::component_tokens::ComponentTokens::DataTable(t) => t,
        _ => unreachable!("the data table component must resolve to DataTableTokens"),
    }
}

/// A data table with two columns and two rows.
fn table() -> MaterialDataTable {
    kit()
        .columns(vec![
            DataTableColumn {
                title: "Name".to_string(),
                align: ColumnAlign::Start,
            },
            DataTableColumn {
                title: "Role".to_string(),
                align: ColumnAlign::End,
            },
        ])
        .rows(vec![
            vec!["Ada".to_string(), "Engineer".to_string()],
            vec!["Grace".to_string(), "Admiral".to_string()],
        ])
}

/// Theme colour -> engine colour.
fn paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// One widget's paint ops.
fn ops_of(widget: &dyn WidgetAny) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

/// The fills a painted box drew, in the order it drew them.
fn fills(widget: &dyn WidgetAny) -> Vec<Color> {
    ops_of(widget)
        .iter()
        .filter_map(|op| match op {
            PaintOp::Rect { color, .. } => Some(*color),
            PaintOp::RoundedRect { color, .. } => Some(*color),
            _ => None,
        })
        .collect()
}

fn rows(table: &impl Widget) -> Vec<Child> {
    let mut frame_children = table.children();
    assert_eq!(frame_children.len(), 1, "the frame holds the rows");
    let rows = frame_children.remove(0).build();
    rows.children()
}

/// The row a clickable row holds.
fn pressable_row(pressable: &Child) -> Box<dyn WidgetAny> {
    let row = pressable.build();
    let mut children = row.children();
    assert_eq!(children.len(), 1, "the clickable row holds the row");
    children.remove(0).build()
}

/// The text a cell paints, with its size, its ink and its weight.
fn cell_text(cell: &dyn WidgetAny) -> Option<(String, f32, Color, FontWeight)> {
    let mut children = cell.children();
    assert_eq!(children.len(), 1, "a cell holds one text");
    let text = children.remove(0).build();
    ops_of(&*text).iter().find_map(|op| match op {
        PaintOp::Text {
            text,
            size,
            color,
            weight,
            ..
        } => Some((text.clone(), *size, *color, *weight)),
        _ => None,
    })
}

/// A full press-and-release, which is how the engine sees an activation.
/// Returns whether the release was consumed.
fn click(row: &mut Box<dyn WidgetAny>) -> bool {
    let at = Point::new(8.0, 16.0);
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = row.create(&mut create);
    let mut consumed = false;
    for phase in [PointerEvent::primary_down(at), PointerEvent::primary_up(at)] {
        let mut cx = TestEventContext::new(id, ROW, phase.into());
        consumed = row.event(&mut cx, id);
    }
    consumed
}

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: MaterialDataTableStyle = kit().resolved_style();
    assert_eq!(style.header_height, tokens().header_height);
    assert_eq!(style.row_height, tokens().row_height);
    assert_eq!(style.dense_row_height, tokens().dense_row_height);
    assert_eq!(style.cell_padding, tokens().cell_padding);
    assert_eq!(style.border_width, tokens().border_width);
    assert_eq!(style.border_radius, tokens().border_radius);
    assert_eq!(style.min_width, tokens().min_width);
    assert_eq!(style.font_size, tokens().font_size);
    assert_eq!(style.header_font_size, tokens().header_font_size);
    assert_eq!(style.surface, theme().colors().surface, "the frame's fill");
    assert_eq!(
        style.on_surface,
        theme().colors().on_surface,
        "the cells' ink"
    );
    assert_eq!(
        style.outline,
        theme().colors().outline,
        "the frame's outline"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
}

#[test]
fn the_rules_are_v1_s() {
    let style = kit().resolved_style();
    assert_eq!(
        style.header_divider,
        floom_core_theme::tokens::color_ext::with_alpha(theme().colors().outline, 60),
        "the header's rule"
    );
    assert_eq!(
        style.row_divider,
        floom_core_theme::tokens::color_ext::with_alpha(theme().colors().outline, 30),
        "the rows' rule"
    );
    assert_eq!(
        style.stripe,
        floom_core_theme::tokens::color_ext::with_alpha(theme().colors().on_surface, 6),
        "the striped row's fill"
    );
    assert_eq!(style.divider_width, 1.0, "v1 wrote `border_bottom(1.0)`");
}

#[test]
fn the_dense_row_height_is_reported_alongside_v1_s_row_height() {
    let style = kit().dense(true).resolved_style();
    assert_eq!(
        style.row_height,
        tokens().row_height,
        "v1's style reported the token's row height"
    );
    assert_eq!(
        style.dense_row_height,
        tokens().dense_row_height,
        "and the port adds the height the dense rows are drawn at"
    );
}

/// The props report the table -- and the callback slot stays empty, because
/// `DataTableProps::on_row_click` is not shareable.
#[test]
fn the_props_report_the_table() {
    let kit = table().striped(true).dense(true);
    assert_eq!(kit.props().columns.len(), 2);
    assert_eq!(kit.props().rows.len(), 2);
    assert!(kit.props().striped);
    assert!(kit.props().dense);
    assert!(
        kit.props().on_row_click.is_none(),
        "the kit keeps its own shareable handle; v1's props slot could not hold one"
    );
}

#[test]
fn the_frame_is_filled_and_outlined_in_v1_s_colours() {
    let style = table().resolved_style();
    let frame = table().view();
    assert_eq!(
        fills(&frame).first(),
        Some(&paint(style.surface)),
        "v1's frame fill"
    );
    let border = frame
        .decoration()
        .border_spec()
        .expect("the frame is outlined");
    assert_eq!(border.width(), style.border_width);
    assert_eq!(border.color(), paint(style.outline));
}

#[test]
fn the_header_row_is_filled_and_ruled_in_v1_s_colours() {
    let style = table().resolved_style();
    let rows = rows(&table().view());
    let header = rows[0].build();
    let fills = fills(&*header);
    assert_eq!(fills.len(), 2, "the fill and the rule under it");
    assert_eq!(
        fills[0],
        paint(theme().colors().surface_variant),
        "v1's fill"
    );
    assert_eq!(fills[1], paint(style.header_divider), "v1's header rule");
}

#[test]
fn the_rows_carry_v1_s_rule_and_stripe() {
    let style = table().resolved_style();
    let table_rows = rows(&table().view());
    let plain_row = pressable_row(&table_rows[1]);
    let plain = fills(&*plain_row);
    assert_eq!(plain.len(), 1, "v1's plain row has no fill of its own");
    assert_eq!(plain[0], paint(style.row_divider), "v1's row rule");

    let striped = rows(&table().striped(true).view());
    let striped_row = pressable_row(&striped[2]);
    let second = fills(&*striped_row);
    assert_eq!(second.len(), 2, "the stripe's fill and the rule");
    assert_eq!(second[0], paint(style.stripe), "v1's stripe");
    assert_eq!(second[1], paint(style.row_divider), "and the rule under it");
}

#[test]
fn the_cells_are_painted_in_v1_s_ink() {
    let style = table().resolved_style();
    let rows = rows(&table().view());

    let header = rows[0].build();
    let header_cells = header.children();
    assert_eq!(header_cells.len(), 2, "one cell per column");
    let header_cell = header_cells[0].build();
    let (text, size, ink, weight) = cell_text(&*header_cell).expect("the header's text");
    assert_eq!(text, "Name");
    assert_eq!(size, style.header_font_size, "v1's header size");
    assert_eq!(ink, paint(style.on_surface), "v1's header ink");
    assert_eq!(weight, FontWeight::Medium, "v1's header weight");

    let body_row = pressable_row(&rows[1]);
    let body_cells = body_row.children();
    let body_cell = body_cells[0].build();
    let (text, size, ink, _) = cell_text(&*body_cell).expect("the cell's text");
    assert_eq!(text, "Ada");
    assert_eq!(size, style.font_size, "v1's body size");
    assert_eq!(ink, paint(style.on_surface), "v1's body ink");
}

/// The header comes first and the last row is the last row.
#[test]
fn the_header_comes_first_and_the_last_row_is_the_last_row() {
    let rows = rows(&table().view());
    assert_eq!(rows.len(), 3, "the header and the two body rows");
    let header = rows[0].build();
    let header_cells = header.children();
    let header_cell = header_cells[0].build();
    assert_eq!(
        cell_text(&*header_cell).expect("text").0,
        "Name",
        "the header is first"
    );
    let last = pressable_row(&rows[2]);
    let last_cells = last.children();
    let last_cell = last_cells[0].build();
    assert_eq!(
        cell_text(&*last_cell).expect("text").0,
        "Grace",
        "and the last row is the last row"
    );
}

/// The cells take an equal share of the row and are padded on the horizontal
/// axis only, as v1 wrote them.
#[test]
fn the_cells_are_padded_on_the_horizontal_axis_only() {
    let style = table().resolved_style();
    let rows = rows(&table().view());
    let pressable = rows[1].build();
    let row_slot = pressable.children().remove(0);
    let row = row_slot.build();
    let cell = &row.children()[0];
    assert_eq!(
        cell.layout.padding,
        Some(Insets {
            left: f64::from(style.cell_padding),
            right: f64::from(style.cell_padding),
            top: 0.0,
            bottom: 0.0,
        }),
        "v1's `padding_horiz(cell_padding)`"
    );
    assert_eq!(
        cell.layout.flex_grow,
        Some(1.0),
        "and every cell takes an equal share of the row"
    );
}

/// The columns' alignments reach the cells.
#[test]
fn the_columns_alignments_reach_the_cells() {
    use floom_widgets::Align;
    let rows = rows(&table().view());
    let header = rows[0].build();
    let header_cells = header.children();
    assert_eq!(
        header_cells[0].layout.main_align,
        Some(Align::Start),
        "the first column starts"
    );
    assert_eq!(
        header_cells[1].layout.main_align,
        Some(Align::End),
        "and the second ends"
    );
}

#[test]
fn the_rows_declare_v1_s_heights() {
    let style = table().resolved_style();
    let table_rows = rows(&table().view());
    let body = pressable_row(&table_rows[1]);
    let request = body.layout_request().expect("a request");
    assert_eq!(request.height.and_then(|h| h.px()), Some(style.row_height));

    let dense = rows(&table().dense(true).view());
    let dense_body = pressable_row(&dense[1]);
    let request = dense_body.layout_request().expect("a request");
    assert_eq!(
        request.height.and_then(|h| h.px()),
        Some(style.dense_row_height),
        "a dense row is drawn at the dense height"
    );
}

/// A clicked row answers with its index.
#[test]
fn a_clicked_row_answers_with_its_index() {
    let clicked: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = clicked.clone();
    let rows = rows(
        &table()
            .on_row_click(move |index| sink.lock().unwrap().push(index))
            .view(),
    );
    let mut first = rows[1].build();
    assert!(click(&mut first), "the click is consumed");
    let mut second = rows[2].build();
    assert!(click(&mut second), "the click is consumed");
    assert_eq!(
        *clicked.lock().unwrap(),
        vec![0, 1],
        "the rows answer with their own index, counted from zero"
    );
}

#[test]
fn a_table_with_no_callback_still_consumes_the_click() {
    let rows = rows(&table().view());
    let mut first = rows[1].build();
    assert!(click(&mut first), "the pressable consumes it");
    assert_eq!(rows.len(), 3, "and the table is still there");
}

#[test]
fn the_theme_is_the_app_wide_one() {
    assert_eq!(
        MaterialDataTable::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material data table read the app-wide signal, not a snapshot"
    );
}
