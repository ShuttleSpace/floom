//! The ported Fluent table -- a header row and body rows in a shared

use floom_core::{ElementId, Rect, Widget, WidgetAny};
use floom_types::{Color, FontWeight};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_table::{to_engine_weight, FluentTable, FluentTableStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 400.0,
};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn kit() -> FluentTable {
    FluentTable::new().with_theme(theme())
}

fn tokens() -> floom_core_theme::component_tokens::TableTokens {
    match theme().component(floom_core_theme::component_tokens::ComponentId::Table) {
        floom_core_theme::component_tokens::ComponentTokens::Table(t) => t,
        _ => unreachable!("the table component must resolve to TableTokens"),
    }
}

fn table() -> FluentTable {
    kit()
        .headers(vec!["Name".to_string(), "Role".to_string()])
        .rows(vec![
            vec!["Ada".to_string(), "Engineer".to_string()],
            vec!["Grace".to_string(), "Admiral".to_string()],
        ])
}

fn paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn ops_of(widget: &dyn WidgetAny) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

fn rows(table: &impl Widget) -> Vec<Box<dyn WidgetAny>> {
    let mut frame_children = table.children();
    assert_eq!(frame_children.len(), 1, "the frame holds the rows");
    let rows = frame_children.remove(0).build();
    rows.children().into_iter().map(|c| c.build()).collect()
}

fn cells(row: &Box<dyn WidgetAny>) -> Vec<Box<dyn WidgetAny>> {
    row.children().into_iter().map(|c| c.build()).collect()
}

fn fills(row: &Box<dyn WidgetAny>) -> Vec<Color> {
    ops_of(&**row)
        .iter()
        .filter_map(|op| match op {
            PaintOp::Rect { color, .. } => Some(*color),
            PaintOp::RoundedRect { color, .. } => Some(*color),
            _ => None,
        })
        .collect()
}

fn cell_text(cell: &Box<dyn WidgetAny>) -> Option<(String, f32, Color, FontWeight)> {
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


#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: FluentTableStyle = kit().resolved_style();
    assert_eq!(style.header_height, tokens().header_height);
    assert_eq!(style.row_height, tokens().row_height);
    assert_eq!(style.cell_padding, tokens().cell_padding);
    assert_eq!(style.border_width, tokens().border_width);
    assert_eq!(style.border_radius, tokens().border_radius);
    assert_eq!(style.font_size, tokens().font_size);
    assert_eq!(style.header_font_size, tokens().header_font_size);
    assert_eq!(style.min_width, tokens().min_width);
    assert_eq!(
        style.surface,
        theme().colors().surface,
        "the plain rows' fill"
    );
    assert_eq!(
        style.surface_variant,
        theme().colors().surface_variant,
        "the header's fill"
    );
    assert_eq!(
        style.on_surface,
        theme().colors().on_surface,
        "the header's ink"
    );
    assert_eq!(
        style.on_surface_variant,
        theme().colors().on_surface_variant,
        "the body cells' ink"
    );
    assert_eq!(
        style.outline,
        theme().colors().outline,
        "the frame and the rules"
    );
    assert_eq!(
        style.header_weight,
        to_engine_weight(theme().typography().label.large.font_weight),
        "v1 read the header's weight from the typography scale"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "fluent");
}

#[test]
fn a_dense_table_is_shorter_by_v1_s_factors() {
    let style = kit().dense(true).resolved_style();
    assert!(
        (style.row_height - tokens().row_height * 0.78).abs() < 1e-4,
        "the dense row height: {}",
        style.row_height
    );
    assert!(
        (style.header_height - tokens().header_height * 0.82).abs() < 1e-4,
        "the dense header height: {}",
        style.header_height
    );
    assert!(style.dense);
}

#[test]
fn the_props_report_the_table() {
    let kit = table().dense(true).striped(true);
    assert_eq!(kit.props().headers.len(), 2);
    assert_eq!(kit.props().rows.len(), 2);
    assert!(kit.props().dense);
    assert!(kit.props().striped);
}


#[test]
fn the_frame_is_outlined_in_v1_s_colour() {
    let frame = table().view();
    let border = frame
        .decoration()
        .border_spec()
        .expect("the frame is outlined");
    assert_eq!(border.width(), tokens().border_width);
    assert_eq!(border.color(), paint(theme().colors().outline));
    assert_eq!(border.edges(), floom_core::Edges::ALL, "all four sides");
    assert_eq!(frame.decoration().corner_radius(), tokens().border_radius);

    let ops = ops_of(&frame);
    assert!(
        ops.iter().any(|op| matches!(
            op,
            PaintOp::Path {
                stroke: Some(_),
                ..
            }
        )),
        "the frame is stroked: {ops:?}"
    );
    assert!(
        !ops.iter()
            .any(|op| matches!(op, PaintOp::Rect { .. } | PaintOp::RoundedRect { .. })),
        "and v1's frame had no fill of its own: {ops:?}"
    );
}

#[test]
fn the_header_row_is_filled_in_v1_s_colour() {
    let style = table().resolved_style();
    let table_rows = rows(&table().view());
    assert_eq!(table_rows.len(), 3, "the header and the two body rows");
    let header = &table_rows[0];
    let fills = fills(header);
    assert_eq!(fills.len(), 1, "the header's fill and nothing else");
    assert_eq!(fills[0], paint(style.surface_variant), "v1's header fill");
}

#[test]
fn the_rows_are_filled_in_v1_s_colours() {
    let style = table().resolved_style();
    let table_rows = rows(&table().view());
    assert_eq!(
        fills(&table_rows[1])[0],
        paint(style.surface),
        "the first row"
    );
    assert_eq!(
        fills(&table_rows[2])[0],
        paint(style.surface),
        "the second row"
    );

    let striped = rows(&table().striped(true).view());
    assert_eq!(
        fills(&striped[1])[0],
        paint(style.surface),
        "the first row is not striped"
    );
    assert_eq!(
        fills(&striped[2])[0],
        paint(style.surface_variant),
        "the second row is"
    );
}

#[test]
fn the_body_rows_carry_v1_s_rule() {
    let style = table().resolved_style();
    let rows = rows(&table().view());
    let fills = fills(&rows[1]);
    assert_eq!(fills.len(), 2, "the fill and the rule under it");
    assert_eq!(fills[1], paint(style.outline), "v1's rule colour");
}

#[test]
fn the_cells_are_painted_in_v1_s_ink() {
    let style = table().resolved_style();
    let rows = rows(&table().view());

    let header_cells = cells(&rows[0]);
    assert_eq!(header_cells.len(), 2, "one cell per header");
    let (text, size, ink, weight) = cell_text(&header_cells[0]).expect("the header's text");
    assert_eq!(text, "Name");
    assert_eq!(size, style.header_font_size, "v1's header size");
    assert_eq!(ink, paint(style.on_surface), "v1's header ink");
    assert_eq!(weight, style.header_weight, "v1's header weight");

    let body_cells = cells(&rows[1]);
    assert_eq!(body_cells.len(), 2, "one cell per value");
    let (text, size, ink, _) = cell_text(&body_cells[0]).expect("the cell's text");
    assert_eq!(text, "Ada");
    assert_eq!(size, style.font_size, "v1's body size");
    assert_eq!(ink, paint(style.on_surface_variant), "v1's body ink");
}


#[test]
fn the_header_comes_first_and_the_last_row_is_the_last_row() {
    let rows = rows(&table().view());
    assert_eq!(rows.len(), 3);
    let header_cells = cells(&rows[0]);
    assert_eq!(
        cell_text(&header_cells[0]).expect("text").0,
        "Name",
        "the header is first"
    );
    let last_cells = cells(&rows[2]);
    assert_eq!(
        cell_text(&last_cells[0]).expect("text").0,
        "Grace",
        "and the last row is the last row"
    );
}

#[test]
fn the_frame_declares_the_minimum_width() {
    let frame = table().view();
    let request = frame.layout_request().expect("a request");
    assert_eq!(
        request.min_width.and_then(|w| w.px()),
        Some(tokens().min_width),
        "v1's `min_width`"
    );
}

#[test]
fn the_rows_declare_v1_s_heights() {
    let style = table().resolved_style();
    let rows = rows(&table().view());
    let header = rows[0].layout_request().expect("a request");
    assert_eq!(
        header.height.and_then(|h| h.px()),
        Some(style.header_height)
    );
    let body = rows[1].layout_request().expect("a request");
    assert_eq!(body.height.and_then(|h| h.px()), Some(style.row_height));
}


#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    assert_ne!(
        FluentTable::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Fluent table took a snapshot, not the app-wide signal"
    );
}
