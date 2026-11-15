#![cfg(feature = "bottom_navigation")]
//! The ported Fluent bottom navigation -- a bar of destinations, one of them

use floom_core::{
    Child, ChildKey, ElementId, LayoutRequest, Point, PointerEvent, Rect, Widget, WidgetAny,
};
use floom_types::Color;
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder, TestEventContext};
use floom_widgets::Pressable;

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_fluent::widgets::bottom_navigation::{
    BottomNavigationBar, FluentBottomNavigation, FluentBottomNavigationStyle, IconBuilder,
};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 56.0,
};

const ROW: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 56.0,
};

const AT: Point = Point { x: 20.0, y: 20.0 };

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn kit() -> FluentBottomNavigation {
    FluentBottomNavigation::new().with_theme(theme())
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

fn icon() -> IconBuilder {
    std::sync::Arc::new(|| Box::new(floom_widgets::Text::new("I")))
}

fn label_of(bar: &BottomNavigationBar, index: usize) -> Box<dyn WidgetAny> {
    let pressable = bar.children()[index].build();
    let destination = pressable.children().remove(0).build();
    let mut children = destination.children();
    children.remove(children.len() - 1).build()
}

fn label_painted(bar: &BottomNavigationBar, index: usize) -> (String, f32, Color) {
    let label = label_of(bar, index);
    ops_of(&*label)
        .iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("a destination paints its label")
}

fn destination_of(bar: &BottomNavigationBar, index: usize) -> Box<dyn WidgetAny> {
    let pressable = bar.children()[index].build();
    pressable.children().remove(0).build()
}

fn tap(bar: &BottomNavigationBar, index: usize) {
    let mut mounted = bar.children()[index].build();
    let pressable = mounted
        .as_any_mut()
        .downcast_mut::<Pressable>()
        .expect("a destination is a pressable");
    let mut pressable = std::mem::replace(
        pressable,
        Pressable::new(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
            Box::new(floom_widgets::Text::new(""))
        })),
    );
    let mut create = AllocatingCreateContext::new();
    let id = pressable.create(&mut create);
    for phase in [PointerEvent::primary_down(AT), PointerEvent::primary_up(AT)] {
        let mut cx = TestEventContext::new(id, ROW, phase.into());
        pressable.event(&mut cx, id);
    }
}

#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: FluentBottomNavigationStyle = kit().resolved_style();
    let tokens = match theme()
        .component(floom_core_theme::component_tokens::ComponentId::BottomNavigation)
    {
        floom_core_theme::component_tokens::ComponentTokens::BottomNavigation(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.height, tokens.height, "the token's bar height");
    assert_eq!(style.icon_size, tokens.icon_size, "the token's icon size");
    assert_eq!(
        style.label_font_size, tokens.label_font_size,
        "the token's label size"
    );
    assert_eq!(style.gap, tokens.gap, "the token's gap");
    assert_eq!(style.background, theme().colors().surface, "the bar's fill");
    assert_eq!(
        style.selected_color,
        theme().colors().primary,
        "the chosen ink"
    );
    assert_eq!(
        style.unselected_color,
        theme().colors().on_surface_variant,
        "the other destinations' ink"
    );
}

#[test]
fn the_rule_along_the_bar_is_the_one_v1_drew() {
    let style = kit().resolved_style();
    assert_eq!(
        style.divider_color.a, 12,
        "v1's `with_alpha(on_surface, 12)`"
    );
    assert_eq!(style.divider_width, 1.0, "v1's `.border_top(1.0)`");
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "fluent");
}

#[test]
fn the_props_report_the_destinations_and_not_the_callback() {
    let kit = kit().labels(vec!["Home".into(), "Search".into()]);
    assert_eq!(
        kit.props().labels,
        vec!["Home".to_string(), "Search".to_string()]
    );
    assert_eq!(kit.props().selected, 0);
    assert!(kit.props().on_change.is_none());
}

#[test]
fn the_bar_paints_v1_s_fill_and_rule() {
    let resolved = kit().resolved_style();
    let bar = kit().labels(vec!["Home".into()]).view();

    let ops = ops_of(&bar);
    let fill = ops.iter().find_map(|op| match op {
        PaintOp::Rect { bounds, color } if bounds.height() > 1.0 => Some((*bounds, *color)),
        _ => None,
    });
    assert_eq!(
        fill,
        Some((BOX, paint(resolved.background))),
        "the bar's fill, over the whole bar"
    );

    let rule = ops.iter().find_map(|op| match op {
        PaintOp::Rect { bounds, color } if bounds.height() <= 1.0 => Some((*bounds, *color)),
        _ => None,
    });
    assert_eq!(
        rule,
        Some((
            Rect {
                x0: BOX.x0,
                y0: BOX.y0,
                x1: BOX.x1,
                y1: BOX.y0 + 1.0,
            },
            paint(resolved.divider_color),
        )),
        "and the rule along the top edge"
    );
}

#[test]
fn the_label_is_painted_in_v1_s_ink_and_size() {
    let resolved = kit().resolved_style();
    let bar = kit()
        .labels(vec!["Home".into(), "Search".into()])
        .selected(1)
        .view();

    let (text, size, ink) = label_painted(&bar, 0);
    assert_eq!(text, "Home");
    assert_eq!(size, resolved.label_font_size, "v1's label size");
    assert_eq!(
        ink,
        paint(resolved.unselected_color),
        "the destination that is not chosen"
    );
}

#[test]
fn the_chosen_destination_is_painted_in_v1_s_ink() {
    let resolved = kit().resolved_style();
    let bar = kit()
        .labels(vec!["Home".into(), "Search".into()])
        .selected(1)
        .view();

    let (text, _, ink) = label_painted(&bar, 1);
    assert_eq!(text, "Search");
    assert_eq!(
        ink,
        paint(resolved.selected_color),
        "the chosen destination's label"
    );
}

#[test]
fn the_icon_is_sized_as_v1_sized_it() {
    let resolved = kit().resolved_style();
    let bar = kit().labels(vec!["Home".into()]).icons(vec![icon()]).view();
    let destination = destination_of(&bar, 0);
    let mut children = destination.children();
    assert_eq!(children.len(), 2, "the icon and the label");
    let icon_slot = children.remove(0).layout;
    assert_eq!(
        icon_slot.width,
        Some(floom_core::Length::Px(resolved.icon_size)),
        "v1's `width(icon_size).height(icon_size)`"
    );
}

#[test]
fn the_chosen_destination_has_a_rule_along_its_top_edge() {
    let resolved = kit().resolved_style();
    let bar = kit()
        .labels(vec!["Home".into(), "Search".into()])
        .selected(1)
        .view();

    let chosen = destination_of(&bar, 1);
    let rule = ops_of(&*chosen).into_iter().find_map(|op| match op {
        PaintOp::Rect { bounds, color } => Some((bounds, color)),
        _ => None,
    });
    assert_eq!(
        rule,
        Some((
            Rect {
                x0: BOX.x0,
                y0: BOX.y0,
                x1: BOX.x1,
                y1: BOX.y0 + 2.0,
            },
            paint(resolved.selected_color),
        )),
        "v1's `border_top(2.0).border_color(primary)`"
    );

    let other = destination_of(&bar, 0);
    assert!(
        ops_of(&*other).is_empty(),
        "and no rule on a destination that is not chosen"
    );
}

#[test]
fn the_bar_is_a_row_of_the_tokens_height() {
    let resolved = kit().resolved_style();
    let bar = kit().labels(vec!["Home".into(), "Search".into()]).view();
    let request = bar.layout_request().expect("the bar declares its own box");
    assert_eq!(request.axis, Some(floom_core::Axis::Horizontal));
    assert_eq!(
        request.height,
        Some(floom_core::Length::Px(resolved.height))
    );
    assert_eq!(request.width, Some(floom_core::Length::Pct(100.0)));
}

#[test]
fn there_is_one_destination_per_label() {
    let bar = kit()
        .labels(vec!["Home".into(), "Search".into(), "Profile".into()])
        .view();
    assert_eq!(bar.children().len(), 3, "three destinations");

    let slot = bar.children()[0].layout;
    assert_eq!(slot.flex_grow, Some(1.0), "the destinations share the bar");
    assert_eq!(
        slot.min_width, None,
        "v1's Fluent destinations asked for no minimum width"
    );
}

#[test]
fn the_destination_chosen_before_mount_is_the_chosen_one() {
    let bar = kit()
        .labels(vec!["Home".into(), "Search".into()])
        .selected(1)
        .view();
    assert_eq!(bar.selected(), 1);
}

#[test]
fn tapping_a_destination_chooses_it() {
    let resolved = kit().resolved_style();
    let bar = kit().labels(vec!["Home".into(), "Search".into()]).view();
    assert_eq!(
        bar.selected(),
        0,
        "v1's first destination is chosen to begin with"
    );
    tap(&bar, 1);
    assert_eq!(bar.selected(), 1);

    let (_, _, ink) = label_painted(&bar, 1);
    assert_eq!(
        ink,
        paint(resolved.selected_color),
        "the destination that was tapped"
    );
}

#[test]
fn the_change_callback_reports_the_chosen_destination() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let s = seen.clone();
    let bar = kit()
        .labels(vec!["Home".into(), "Search".into()])
        .on_change(move |index| s.lock().unwrap().push(index))
        .view();

    tap(&bar, 1);
    assert_eq!(seen.lock().unwrap().as_slice(), &[1]);
}

#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    assert_ne!(
        FluentBottomNavigation::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Fluent bottom navigation took a snapshot, not the app-wide signal"
    );
}
