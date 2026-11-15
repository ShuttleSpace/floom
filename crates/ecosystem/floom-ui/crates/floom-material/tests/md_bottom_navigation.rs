#![cfg(feature = "bottom_navigation")]
use floom_core::{
    Child, ChildKey, ElementId, LayoutRequest, Point, PointerEvent, Rect, Widget, WidgetAny,
};
use floom_types::Color;
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder, TestEventContext};
use floom_widgets::Pressable;

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_material::widgets::bottom_navigation::{
    BottomNavigationBar, IconBuilder, MaterialBottomNavigation, MaterialBottomNavigationStyle,
};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 80.0,
};

const ROW: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 80.0,
};

const AT: Point = Point { x: 20.0, y: 20.0 };

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn kit() -> MaterialBottomNavigation {
    MaterialBottomNavigation::new().with_theme(theme())
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
fn ops_of(widget: &dyn Widget) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

/// An icon builder that paints a glyph, so a test can look for it.
fn icon() -> IconBuilder {
    std::sync::Arc::new(|| Box::new(floom_widgets::Text::new("I")))
}

/// The destination's own box: the slot it is mounted in.
fn destination_slot(bar: &BottomNavigationBar, index: usize) -> LayoutRequest {
    bar.children()[index].layout
}

/// The label one destination shows.
fn label_of(bar: &BottomNavigationBar, index: usize) -> Box<dyn WidgetAny> {
    let pressable = bar.children()[index].build();
    let destination = pressable.children().remove(0).build();
    let mut children = destination.children();
    children.remove(children.len() - 1).build()
}

/// The text, the size and the ink one destination's label painted.
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

/// Taps a destination, the way the host would.
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

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: MaterialBottomNavigationStyle = kit().resolved_style();
    let tokens = match theme()
        .component(floom_core_theme::component_tokens::ComponentId::BottomNavigation)
    {
        floom_core_theme::component_tokens::ComponentTokens::BottomNavigation(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.height, tokens.height, "the token's bar height");
    assert_eq!(
        style.item_min_width, tokens.item_min_width,
        "the token's minimum width"
    );
    assert_eq!(style.icon_size, tokens.icon_size, "the token's icon size");
    assert_eq!(
        style.label_font_size, tokens.label_font_size,
        "the token's label size"
    );
    assert_eq!(style.gap, tokens.gap, "the token's gap");
    assert_eq!(style.bar_color, theme().colors().surface, "the bar's fill");
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
    assert_eq!(
        style.divider_color,
        theme().colors().outline_variant,
        "the rule along the top"
    );
}

/// v1 wrote `.border_top(1.0)`: a literal, and the port says so.
#[test]
fn the_rule_along_the_top_is_v1_s_one_point() {
    assert_eq!(kit().resolved_style().divider_width, 1.0);
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
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
        Some((BOX, paint(resolved.bar_color))),
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

/// The chosen destination's label is painted in the chosen ink.
#[test]
fn the_chosen_destination_s_label_is_painted_in_v1_s_ink() {
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
fn the_icon_is_not_sized_as_v1_did_not_size_it() {
    let bar = kit().labels(vec!["Home".into()]).icons(vec![icon()]).view();
    let pressable = bar.children()[0].build();
    let destination = pressable.children().remove(0).build();
    let mut children = destination.children();
    assert_eq!(children.len(), 2, "the icon and the label");
    let icon_slot = children.remove(0).layout;
    assert_eq!(
        icon_slot.width, None,
        "v1's Material destinations did not size the icon"
    );
}

/// The bar is a row, as tall as the token says and as wide as its parent.
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

/// One destination per label, each at least as wide as the token says and
/// growing to share the bar.
#[test]
fn there_is_one_destination_per_label() {
    let resolved = kit().resolved_style();
    let bar = kit()
        .labels(vec!["Home".into(), "Search".into(), "Profile".into()])
        .view();
    assert_eq!(bar.children().len(), 3, "three destinations");

    let slot = destination_slot(&bar, 0);
    assert_eq!(
        slot.min_width,
        Some(floom_core::Length::Px(resolved.item_min_width)),
        "v1's `min_width(item_min_width)`"
    );
    assert_eq!(slot.flex_grow, Some(1.0), "the destinations share the bar");
}

/// The chosen destination is the one the props said, before mount.
#[test]
fn the_destination_chosen_before_mount_is_the_chosen_one() {
    let bar = kit()
        .labels(vec!["Home".into(), "Search".into()])
        .selected(1)
        .view();
    assert_eq!(bar.selected(), 1);
}

/// Tapping a destination chooses it.
#[test]
fn tapping_a_destination_chooses_it() {
    let bar = kit().labels(vec!["Home".into(), "Search".into()]).view();
    assert_eq!(
        bar.selected(),
        0,
        "v1's first destination is chosen to begin with"
    );
    tap(&bar, 1);
    assert_eq!(bar.selected(), 1);
}

/// And the chosen destination's label is painted in the chosen ink afterwards.
#[test]
fn the_label_s_ink_follows_the_choice() {
    let resolved = kit().resolved_style();
    let bar = kit().labels(vec!["Home".into(), "Search".into()]).view();
    tap(&bar, 1);

    let (_, _, ink) = label_painted(&bar, 1);
    assert_eq!(
        ink,
        paint(resolved.selected_color),
        "the destination that was tapped"
    );
    let (_, _, ink) = label_painted(&bar, 0);
    assert_eq!(
        ink,
        paint(resolved.unselected_color),
        "and the one that was not"
    );
}

/// The callback reports the chosen destination.
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
fn the_theme_is_the_app_wide_one() {
    assert_eq!(
        MaterialBottomNavigation::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material bottom navigation read the app-wide signal, not a snapshot"
    );
}
