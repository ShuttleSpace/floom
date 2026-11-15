#![cfg(feature = "tabs")]
use std::sync::{Arc, Mutex};

use floom_core::{ElementId, LayoutRequest, Point, Rect, Widget, WidgetAny};
use floom_types::{Color, PointerEvent};
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder, TestEventContext,
};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_material::widgets::tabs::{MaterialTabs, MaterialTabsStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 400.0,
};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn kit() -> MaterialTabs {
    MaterialTabs::new().with_theme(theme())
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

fn ops_of_tree(widget: &mut dyn WidgetAny) -> Vec<PaintOp> {
    let mut out = Vec::new();
    collect_ops(widget, &mut out);
    out
}

fn collect_ops(widget: &mut dyn WidgetAny, out: &mut Vec<PaintOp>) {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut update = RecordingUpdateContext::new();
    widget.update(&mut update, id);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    out.extend(recorder.ops().iter().cloned());
    for child in widget.children() {
        let mut built = child.build();
        collect_ops(&mut *built, out);
    }
}

/// The tabs of a bar, one widget each.
fn tab_widgets(kit: &MaterialTabs) -> Vec<Box<dyn WidgetAny>> {
    kit.view()
        .children()
        .into_iter()
        .map(|child| child.build())
        .collect()
}

fn slots_of(widget: &mut dyn WidgetAny) -> Vec<(LayoutRequest, Box<dyn WidgetAny>)> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut update = RecordingUpdateContext::new();
    widget.update(&mut update, id);
    widget
        .children()
        .into_iter()
        .map(|child| (child.layout, child.build()))
        .collect()
}

/// The text run in a set of paint ops: its text, its size and its ink.
fn text_of(ops: &[PaintOp]) -> (String, f32, Color) {
    ops.iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("the label is painted")
}

/// The fill in a set of paint ops, if there is one.
fn fill_of(ops: &[PaintOp]) -> Option<Color> {
    ops.iter().find_map(|op| match op {
        PaintOp::Rect { color, .. } => Some(*color),
        PaintOp::RoundedRect { color, .. } => Some(*color),
        _ => None,
    })
}

/// Clicks a tab: a press and a release over the same spot.
fn click(tab: &mut dyn WidgetAny) {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = tab.create(&mut create);
    let point = Point::new(10.0, 10.0);
    let mut cx = TestEventContext::pointer(id, BOX, PointerEvent::primary_down(point));
    tab.event(&mut cx, id);
    let mut cx = TestEventContext::pointer(id, BOX, PointerEvent::primary_up(point));
    tab.event(&mut cx, id);
}

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: MaterialTabsStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::Tabs) {
        floom_core_theme::component_tokens::ComponentTokens::Tabs(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.height, tokens.height, "the token's bar height");
    assert_eq!(
        style.indicator_height, tokens.indicator_height,
        "the token's indicator thickness"
    );
    assert_eq!(style.padding, tokens.padding, "the token's padding");
    assert_eq!(
        style.active_color,
        theme().colors().primary,
        "the active label's ink"
    );
    assert_eq!(
        style.inactive_color,
        theme().colors().on_surface_variant,
        "the inactive label's ink"
    );
    assert_eq!(
        style.indicator_color,
        theme().colors().primary,
        "the indicator's colour"
    );
    assert_eq!(style.label_font_size, 14.0, "v1's label size");
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
}

#[test]
fn the_labels_are_painted_in_v1_s_inks() {
    let resolved = kit().resolved_style();
    let kit = kit().tab("One").tab("Two");
    let mut tabs = tab_widgets(&kit);

    let active = ops_of_tree(&mut *tabs.remove(0));
    let (text, size, ink) = text_of(&active);
    assert_eq!(text, "One");
    assert_eq!(size, resolved.label_font_size, "v1's label size");
    assert_eq!(ink, paint(resolved.active_color), "the active label's ink");

    let inactive = ops_of_tree(&mut *tabs.remove(0));
    let (text, size, ink) = text_of(&inactive);
    assert_eq!(text, "Two");
    assert_eq!(size, resolved.label_font_size, "v1's label size");
    assert_eq!(
        ink,
        paint(resolved.inactive_color),
        "the inactive label's ink"
    );
}

#[test]
fn only_the_active_tab_has_the_indicator() {
    let resolved = kit().resolved_style();
    let kit = kit().tab("One").tab("Two");
    let mut tabs = tab_widgets(&kit);

    let active = ops_of_tree(&mut *tabs.remove(0));
    assert_eq!(
        fill_of(&active),
        Some(paint(resolved.indicator_color)),
        "the indicator under the active tab"
    );

    let inactive = ops_of_tree(&mut *tabs.remove(0));
    assert_eq!(
        fill_of(&inactive),
        None,
        "v1 hid the indicator on an inactive tab"
    );
}

/// The label's box is the tab's height with the token's padding at each side.
#[test]
fn the_label_box_is_the_tabs_height_with_the_tokens_padding() {
    let resolved = kit().resolved_style();
    let kit = kit().tab("One");
    let mut tabs = tab_widgets(&kit);

    let mut tab = slots_of(&mut *tabs.remove(0)).remove(0).1;
    let (request, _) = slots_of(&mut *tab).remove(0);

    assert_eq!(
        request.height,
        Some(floom_core::Length::Px(resolved.height)),
        "v1's label was as tall as the bar"
    );
    let padding = request.padding.expect("v1's label had a padding");
    assert_eq!(
        padding.left,
        f64::from(resolved.padding),
        "v1's left padding"
    );
    assert_eq!(
        padding.right,
        f64::from(resolved.padding),
        "v1's right padding"
    );
    assert_eq!(padding.top, 0.0, "v1 padded the label sideways only");
    assert_eq!(padding.bottom, 0.0, "v1 padded the label sideways only");
}

/// Clicking a tab moves the highlight and calls the callback.
#[test]
fn clicking_a_tab_moves_the_highlight() {
    let seen = Arc::new(Mutex::new(Vec::<usize>::new()));
    let recorder = seen.clone();
    let kit = kit()
        .tab("One")
        .tab("Two")
        .on_change(move |index| recorder.lock().unwrap().push(index));

    let mut tabs = tab_widgets(&kit);
    let mut second = tabs.remove(1);
    click(&mut *second);

    assert_eq!(
        *seen.lock().unwrap(),
        vec![1],
        "the callback was called with the tab's index"
    );
    assert_eq!(kit.active(), Some(1), "the shared selection moved");

    let mut tabs = tab_widgets(&kit);
    let active = ops_of_tree(&mut *tabs.remove(1));
    let (_, _, ink) = text_of(&active);
    assert_eq!(
        ink,
        paint(kit.resolved_style().active_color),
        "the second tab is the active one"
    );
}

/// The tab that was selected before the bar was built is the active one.
#[test]
fn the_selected_tab_is_the_active_one() {
    let kit = kit().tab("One").tab("Two").selected(1);
    assert_eq!(kit.active(), Some(1));
    let mut tabs = tab_widgets(&kit);
    let active = ops_of_tree(&mut *tabs.remove(1));
    let (_, _, ink) = text_of(&active);
    assert_eq!(ink, paint(kit.resolved_style().active_color));
}

#[test]
fn the_props_report_the_tabs() {
    let kit = kit().tab("One").tab("Two");
    assert_eq!(kit.props().tabs, vec!["One".to_string(), "Two".to_string()]);
    assert_eq!(kit.props().selected, 0);
    assert!(
        kit.props().on_change.is_none(),
        "the props' callback slot stays empty"
    );
}

#[test]
fn the_theme_is_the_app_wide_one() {
    assert_eq!(
        MaterialTabs::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material tabs read the app-wide signal, not a snapshot"
    );
}
