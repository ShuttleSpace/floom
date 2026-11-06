//! The ported Fluent pagination -- the page buttons, and the arrows either

use std::sync::{Arc, Mutex};

use floom_core::{Axis, ElementId, Length, Point, Rect, Widget, WidgetAny};
use floom_types::Color;
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder, TestEventContext};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_pagination::{FluentPagination, FluentPaginationStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 400.0,
};

const AT: Point = Point { x: 20.0, y: 20.0 };

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn kit() -> FluentPagination {
    FluentPagination::new().with_theme(theme())
}

fn paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn ops_of<W: Widget + ?Sized>(widget: &W) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

fn column_of(kit: FluentPagination) -> Box<dyn WidgetAny> {
    let outer = kit.view();
    outer.children().remove(0).build()
}

fn control_at(kit: FluentPagination, index: usize) -> Box<dyn WidgetAny> {
    let column = column_of(kit);
    column.children().remove(index).build()
}

fn box_of(control: &dyn WidgetAny) -> Box<dyn WidgetAny> {
    control.children().remove(0).build()
}

fn label_of(control: &dyn WidgetAny) -> Box<dyn WidgetAny> {
    let box_ = box_of(control);
    box_.children().remove(0).build()
}

fn tap(control: &mut dyn WidgetAny) {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = control.create(&mut create);
    for phase in [
        floom_types::PointerEvent::primary_down(AT),
        floom_types::PointerEvent::primary_up(AT),
    ] {
        let mut cx = TestEventContext::new(id, BOX, phase.into());
        control.event(&mut cx, id);
    }
}

#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: FluentPaginationStyle = kit().resolved_style();
    let tokens = match theme().component(ComponentId::Pagination) {
        ComponentTokens::Pagination(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(
        style.button_size, tokens.button_size,
        "the token's button size"
    );
    assert_eq!(style.gap, tokens.gap, "the token's gap");
    assert_eq!(style.radius, theme().radius().md, "the theme's md radius");
    assert_eq!(
        style.selected_bg,
        theme().colors().primary,
        "the selected fill"
    );
    assert_eq!(
        style.selected_fg,
        theme().colors().on_primary,
        "the selected ink"
    );
    assert_eq!(
        style.unselected_fg,
        theme().colors().on_surface,
        "the unselected ink"
    );
    assert_eq!(
        style.unselected_bg,
        floom_core_theme::tokens::color::Color::rgba(0, 0, 0, 0),
        "the unselected fill: nothing at all"
    );
    assert_eq!(
        style.arrow_fg,
        theme().colors().on_surface,
        "the arrow's ink"
    );
    assert_eq!(
        style.disabled_arrow_fg,
        floom_core_theme::tokens::color_ext::with_alpha(theme().colors().on_surface, 38),
        "and the disabled arrow's ink"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "fluent");
}

#[test]
fn the_selected_page_is_filled_in_primary() {
    let style = kit().resolved_style();
    let control = control_at(kit().total(3).current(2), 2);
    let ops = ops_of(&*box_of(control.as_ref()));
    let (fill, radius) = ops
        .iter()
        .find_map(|op| match op {
            PaintOp::RoundedRect { color, radius, .. } => Some((*color, *radius)),
            _ => None,
        })
        .expect("the page is painted");
    assert_eq!(fill, paint(style.selected_bg), "the selected page's fill");
    assert_eq!(radius, style.radius, "and the theme's md radius");

    let label = label_of(control.as_ref());
    let ops = ops_of(&*label);
    let (text, ink) = ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Text { text, color, .. } => Some((text.clone(), *color)),
            _ => None,
        })
        .expect("the page number is painted");
    assert_eq!(text, "2", "the page it names");
    assert_eq!(ink, paint(style.selected_fg), "and the selected ink");
}

#[test]
fn the_other_pages_are_filled_with_nothing() {
    let style = kit().resolved_style();
    let control = control_at(kit().total(3).current(2), 1);
    let ops = ops_of(&*box_of(control.as_ref()));
    let fill = ops.iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(
        fill,
        Some(Color::rgba(0, 0, 0, 0)),
        "v1's unselected page: a transparent fill"
    );

    let label = label_of(control.as_ref());
    let ops = ops_of(&*label);
    let ink = ops.iter().find_map(|op| match op {
        PaintOp::Text { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(
        ink,
        Some(paint(style.unselected_fg)),
        "and the unselected ink"
    );
}

#[test]
fn the_arrows_are_inked_in_on_surface() {
    let style = kit().resolved_style();
    let control = control_at(kit().total(3).current(2), 0);
    let ops = ops_of(&*box_of(control.as_ref()));
    assert!(
        ops.is_empty(),
        "v1's arrow had no fill: it was a colour, and a colour is the ink"
    );

    let label = label_of(control.as_ref());
    let ops = ops_of(&*label);
    let (text, ink) = ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Text { text, color, .. } => Some((text.clone(), *color)),
            _ => None,
        })
        .expect("the arrow is painted");
    assert_eq!(text, "\u{2039}", "the previous arrow");
    assert_eq!(ink, paint(style.arrow_fg), "in the surface's ink");
}

#[test]
fn a_disabled_arrow_is_inked_at_alpha_38() {
    let style = kit().resolved_style();
    let control = control_at(kit().total(3).current(1), 0);
    let label = label_of(control.as_ref());
    let ops = ops_of(&*label);
    let ink = ops.iter().find_map(|op| match op {
        PaintOp::Text { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(
        ink,
        Some(paint(style.disabled_arrow_fg)),
        "v1's disabled arrow: the ink at alpha 38"
    );
}

#[test]
fn the_buttons_are_the_tokens_square() {
    let style = kit().resolved_style();
    let control = control_at(kit().total(3).current(1), 1);
    let request = box_of(control.as_ref())
        .layout_request()
        .expect("the button asks for its own space");
    assert_eq!(request.width, Some(Length::Px(style.button_size)));
    assert_eq!(request.height, Some(Length::Px(style.button_size)));
}

#[test]
fn the_controls_are_a_column() {
    let outer = kit().total(3).current(1).view();
    assert_eq!(outer.axis(), Axis::Horizontal, "v1's container was a row");
    let column = column_of(kit().total(3).current(1));
    let column = column
        .as_any()
        .downcast_ref::<floom_widgets::Stack>()
        .expect("the column is a stack");
    assert_eq!(column.axis(), Axis::Vertical, "and v1's list was a column");
    assert_eq!(
        column.children().len(),
        5,
        "the previous arrow, three pages and the next arrow"
    );
}

#[test]
fn the_buttons_have_no_gap_between_them() {
    let column = column_of(kit().total(3).current(1));
    let request = column
        .layout_request()
        .expect("the column asks for its own space");
    assert_eq!(
        request.gap,
        Some(0.0),
        "v1's gap was on the container that held the column, and that had one child"
    );
}

#[test]
fn the_container_keeps_the_tokens_gap() {
    let style = kit().resolved_style();
    let outer = kit().total(3).current(1).view();
    let request = outer
        .layout_request()
        .expect("the container asks for its own space");
    assert_eq!(
        request.gap,
        Some(style.gap),
        "v1 wrote the token's gap here"
    );
}

#[test]
fn a_page_button_answers_with_its_page() {
    let seen: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));
    let kit = kit().total(3).current(1).on_change({
        let seen = seen.clone();
        move |page| seen.lock().unwrap().push(page)
    });
    let mut control = control_at(kit, 2);
    tap(&mut *control);
    assert_eq!(*seen.lock().unwrap(), vec![2], "the page that was tapped");
}

#[test]
fn a_disabled_arrow_does_not_answer() {
    let seen: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));
    let kit = kit().total(3).current(1).on_change({
        let seen = seen.clone();
        move |page| seen.lock().unwrap().push(page)
    });
    let mut control = control_at(kit, 0);
    tap(&mut *control);
    assert!(
        seen.lock().unwrap().is_empty(),
        "the previous arrow is disabled on the first page"
    );
}

#[test]
fn the_props_report_the_totals() {
    let totals = kit().total(7).current(3);
    assert_eq!(totals.props().total, 7);
    assert_eq!(totals.props().current, 3);
    assert!(
        totals.props().on_change.is_none(),
        "and there is no callback, because none was given"
    );

    let with_callback = kit().total(7).current(3).on_change(|_| {});
    assert!(
        with_callback.props().on_change.is_some(),
        "v1's props held the callback too"
    );
}
