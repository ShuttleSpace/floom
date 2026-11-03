//! The ported Fluent slide group -- a strip of items with one of them

use std::sync::{Arc, Mutex};

use floom_core::{ElementId, Length, Point, Rect, Widget, WidgetAny};
use floom_types::Color;
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder, TestEventContext,
};
use floom_widgets::{Conditional, Stack};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_slide_group::{FluentSlideGroup, FluentSlideGroupStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 200.0,
};

const AT: Point = Point { x: 20.0, y: 20.0 };

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn kit() -> FluentSlideGroup {
    FluentSlideGroup::new()
        .with_theme(theme())
        .items(vec![item("one"), item("two"), item("three")])
}

fn item(label: &'static str) -> floom_core::Child {
    floom_core::Child::new(
        floom_core::ChildKey::index(0),
        floom_core::LayoutRequest::new(),
        move || Box::new(floom_widgets::Text::new(label)),
    )
}

fn paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn ops_of(widget: &dyn Widget) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

fn row_of(area: &Stack) -> Box<dyn WidgetAny> {
    area.children().remove(0).build()
}

fn item_at(area: &Stack, index: usize) -> Box<dyn WidgetAny> {
    let row = row_of(area);
    row.children().remove(index).build()
}

fn item_box_of(item: &dyn WidgetAny) -> Box<dyn WidgetAny> {
    item.children().remove(0).build()
}

fn band_of(item_box: &dyn WidgetAny) -> Box<dyn WidgetAny> {
    item_box.children().remove(1).build()
}

fn conditional_of(band: &dyn WidgetAny) -> Box<dyn WidgetAny> {
    band.children().remove(0).build()
}

fn indicator_of(conditional: &dyn WidgetAny) -> Option<Box<dyn WidgetAny>> {
    let mut children = conditional.children();
    if children.is_empty() {
        None
    } else {
        Some(children.remove(0).build())
    }
}

fn tap(item: &mut dyn WidgetAny) {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = item.create(&mut create);
    for phase in [
        floom_types::PointerEvent::primary_down(AT),
        floom_types::PointerEvent::primary_up(AT),
    ] {
        let mut cx = TestEventContext::new(id, BOX, phase.into());
        item.event(&mut cx, id);
    }
}

fn settle(conditional: &mut dyn WidgetAny) -> usize {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = conditional.create(&mut create);
    let mut ctx = RecordingUpdateContext::default();
    let conditional = conditional
        .as_any_mut()
        .downcast_mut::<Conditional>()
        .expect("the indicator is a conditional");
    conditional.update(&mut ctx, id);
    conditional.children().len()
}


#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: FluentSlideGroupStyle = kit().resolved_style();
    let tokens = match theme().component(ComponentId::SlideGroup) {
        ComponentTokens::SlideGroup(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.height, tokens.height, "the token's height");
    assert_eq!(style.gap, tokens.gap, "the token's gap");
    assert_eq!(
        style.active_indicator_height, tokens.active_indicator_height,
        "the token's indicator"
    );
    assert_eq!(
        style.arrow_size, tokens.arrow_size,
        "the token's arrow size"
    );
    assert_eq!(
        style.on_surface_variant,
        theme().colors().on_surface_variant,
        "the arrows' ink"
    );
    assert_eq!(
        style.primary,
        theme().colors().primary,
        "the indicator's fill"
    );
    assert_eq!(
        style.outline,
        theme().colors().outline,
        "the unchosen indicator"
    );
    assert_eq!(style.item_width, 120.0, "v1's literal");
    assert_eq!(style.arrow_width, 32.0, "v1's literal");
    assert_eq!(
        style.arrow_font_size,
        tokens.arrow_size * 0.6,
        "v1's literal: the arrow size times 0.6"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "fluent");
}


#[test]
fn the_chosen_item_has_a_primary_indicator() {
    let style = kit().selected(1).resolved_style();
    let area = kit().selected(1).view();
    let item = item_at(&area, 1);
    let band = band_of(&*item_box_of(&*item));
    let conditional = conditional_of(&*band);
    let indicator = indicator_of(&*conditional).expect("the chosen item has an indicator");

    let ops = ops_of(&*indicator);
    let fill = ops.iter().find_map(|op| match op {
        PaintOp::Rect { color, .. } => Some(*color),
        PaintOp::RoundedRect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(
        fill,
        Some(paint(style.primary)),
        "the chosen item's indicator"
    );
}

#[test]
fn an_unchosen_item_has_no_indicator() {
    let style = kit().resolved_style();
    let area = kit().view();
    let item = item_at(&area, 2);
    let band = band_of(&*item_box_of(&*item));

    let ops = ops_of(&*band);
    let fill = ops.iter().find_map(|op| match op {
        PaintOp::Rect { color, .. } => Some(*color),
        PaintOp::RoundedRect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(
        fill,
        Some(paint(floom_core_theme::tokens::color_ext::with_alpha(
            style.outline,
            0
        ))),
        "v1's unchosen indicator: the outline at alpha 0"
    );

    let conditional = conditional_of(&*band);
    assert!(
        indicator_of(&*conditional).is_none(),
        "an unchosen item draws no indicator at all"
    );
}

#[test]
fn the_indicator_is_the_tokens_thick() {
    let style = kit().resolved_style();
    let area = kit().view();
    let item = item_at(&area, 0);
    let band = band_of(&*item_box_of(&*item));
    let request = band
        .layout_request()
        .expect("the band asks for its own space");
    assert_eq!(
        request.height,
        Some(Length::Px(style.active_indicator_height)),
        "v1's border was the token's thick"
    );
}

#[test]
fn an_item_is_the_tokens_tall_and_the_literals_wide() {
    let style = kit().resolved_style();
    let area = kit().view();
    let item = item_at(&area, 0);
    let box_ = item_box_of(&*item);
    let request = box_
        .layout_request()
        .expect("the item asks for its own space");
    assert_eq!(request.height, Some(Length::Px(style.height)));
    assert_eq!(
        request.min_width,
        Some(Length::Px(style.item_width)),
        "v1's `min_width(120)`"
    );
}

#[test]
fn the_arrows_are_filled_and_inked() {
    let style = kit().resolved_style();
    let area = kit().view();
    let arrow = area.children().remove(1).build();
    let surface = arrow.children().remove(0).build();

    let ops = ops_of(&*surface);
    let fill = ops.iter().find_map(|op| match op {
        PaintOp::Rect { color, .. } => Some(*color),
        PaintOp::RoundedRect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(
        fill,
        Some(paint(style.arrow_background)),
        "v1's arrow background: the surface at alpha 220"
    );

    let label = surface.children().remove(0).build();
    let ops = ops_of(&*label);
    let (text, size, ink) = ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("the arrow is painted");
    assert_eq!(text, "\u{2039}", "the previous arrow");
    assert_eq!(size, style.arrow_font_size, "v1's arrow font size");
    assert_eq!(ink, paint(style.on_surface_variant), "and the arrows' ink");
}


#[test]
fn there_are_no_arrows_for_one_item() {
    let area = FluentSlideGroup::new()
        .with_theme(theme())
        .items(vec![item("one")])
        .view();
    assert_eq!(area.children().len(), 1, "the strip and nothing else");
}

#[test]
fn the_arrows_are_shown_for_more_than_one_item() {
    let area = kit().view();
    assert_eq!(area.children().len(), 3, "the strip and the two arrows");
}

#[test]
fn the_arrows_can_be_turned_off() {
    let area = kit().show_arrows(false).view();
    assert_eq!(area.children().len(), 1, "the strip and nothing else");
}

#[test]
fn the_items_touch() {
    let area = kit().view();
    let row = row_of(&area);
    let request = row
        .layout_request()
        .expect("the strip asks for its own space");
    assert_eq!(request.gap, Some(0.0), "v1's `gap(0.0)`");
    assert_eq!(
        request.height,
        Some(Length::Px(kit().resolved_style().height)),
        "and the token's height"
    );
}


#[test]
fn tapping_an_item_chooses_it_and_says_so() {
    let seen: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));
    let area = kit()
        .on_change({
            let seen = seen.clone();
            move |index| seen.lock().unwrap().push(index)
        })
        .view();
    let mut item = item_at(&area, 2);
    tap(&mut *item);
    assert_eq!(*seen.lock().unwrap(), vec![2], "the item that was tapped");
}

#[test]
fn tapping_an_item_moves_the_indicator() {
    let area = kit().selected(0).view();
    let chosen = item_at(&area, 0);
    let tapped = item_at(&area, 1);
    let mut chosen_indicator = conditional_of(&*band_of(&*item_box_of(&*chosen)));
    let mut tapped_indicator = conditional_of(&*band_of(&*item_box_of(&*tapped)));

    assert_eq!(
        settle(&mut *chosen_indicator),
        1,
        "the chosen item's indicator"
    );
    assert_eq!(settle(&mut *tapped_indicator), 0, "and the other has none");

    let mut item = item_at(&area, 1);
    tap(&mut *item);

    assert_eq!(
        settle(&mut *chosen_indicator),
        0,
        "the item that was chosen loses its indicator"
    );
    assert_eq!(
        settle(&mut *tapped_indicator),
        1,
        "and the item that was tapped gains one"
    );
}

#[test]
fn a_mandatory_group_ignores_a_tap_on_the_chosen_item() {
    let seen: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(Vec::new()));
    let area = kit()
        .selected(1)
        .mandatory(true)
        .on_change({
            let seen = seen.clone();
            move |index| seen.lock().unwrap().push(index)
        })
        .view();
    let mut item = item_at(&area, 1);
    tap(&mut *item);
    assert!(
        seen.lock().unwrap().is_empty(),
        "v1 ignored a tap on the chosen item when the group was mandatory"
    );
}

#[test]
fn the_props_report_what_the_caller_set() {
    let kit = kit().show_arrows(false).mandatory(true);
    assert!(!kit.props().show_arrows);
    assert!(kit.props().mandatory);
    assert!(
        !kit.props().center_active,
        "v1 never read this prop, and neither does the port"
    );
}
