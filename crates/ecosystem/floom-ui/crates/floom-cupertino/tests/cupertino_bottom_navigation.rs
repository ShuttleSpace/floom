#![cfg(feature = "bottom_navigation")]
//! The ported Cupertino bottom navigation -- a bar of destinations, one of them
//! chosen, and a fill under the one the pointer is over.

use floom_core::{
    Child, ChildKey, ElementId, LayoutRequest, Point, PointerEvent, Rect, Widget, WidgetAny,
};
use floom_types::{Color, PointerPhase};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder, TestEventContext};
use floom_widgets::Pressable;

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_cupertino::widgets::bottom_navigation::{
    BottomNavigationBar, CupertinoBottomNavigation, CupertinoBottomNavigationStyle, IconBuilder,
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
    // `MultiTheme::new_for` builds all three palettes and reads the *current*
    // one, and `set_design_system` writes a thread-local, so the selection is
    floom_core_theme::global_theme::set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

fn kit() -> CupertinoBottomNavigation {
    CupertinoBottomNavigation::new().with_theme(theme())
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

/// The label one destination shows.
///
/// The destination is a box, a pressable and then the content, so the label is
/// three levels down.
fn label_of(bar: &BottomNavigationBar, index: usize) -> Box<dyn WidgetAny> {
    let host = bar.children()[index].build();
    let pressable = host.children().remove(0).build();
    let destination = pressable.children().remove(0).build();
    let mut children = destination.children();
    // The label is the destination's last child; the icon, when there is one,
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
    let host = bar.children()[index].build();
    let mut mounted = host.children().remove(0).build();
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

/// The pointer arriving on a destination, and the pointer leaving it again.
fn pointer_over(destination: &mut Box<dyn WidgetAny>, arriving: bool) {
    let mut create = AllocatingCreateContext::new();
    let id = destination.create(&mut create);
    let mut pointer = PointerEvent::primary_down(AT);
    pointer.phase = if arriving {
        PointerPhase::Enter
    } else {
        PointerPhase::Leave
    };
    let mut cx = TestEventContext::new(id, BOX, pointer.into());
    destination.event(&mut cx, id);
}

// --- the translation ------------------------------------------------------

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: CupertinoBottomNavigationStyle = kit().resolved_style();
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
    assert_eq!(
        style.active_color,
        theme().colors().primary,
        "the chosen ink"
    );
    assert_eq!(
        style.inactive_color,
        theme().colors().on_surface_variant,
        "the other destinations' ink"
    );
}

/// v1's bar was `with_alpha(surface, 235)` and its rule
/// `with_alpha(on_surface, 30)`: the style reports the colours v1 painted.
#[test]
fn the_bar_s_colours_are_the_ones_v1_painted() {
    let style = kit().resolved_style();
    assert_eq!(style.background.a, 235, "v1's `with_alpha(surface, 235)`");
    assert_eq!(
        style.divider_color.a, 30,
        "v1's `with_alpha(on_surface, 30)`"
    );
    assert_eq!(style.hover_color.a, 12, "v1's `with_alpha(on_surface, 12)`");
}

/// v1 wrote `.border_top(0.5)`: a literal, and the port says so.
#[test]
fn the_rule_along_the_top_is_v1_s_half_a_point() {
    assert_eq!(kit().resolved_style().divider_width, 0.5);
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "cupertino");
}

/// The props report the destinations, and not the callback: the kit holds that
/// as an `Arc` of its own, because 2.0's slots are `Send + Sync` and the
/// props' `Box<dyn Fn(..)>` is not.
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

// --- the pixels -----------------------------------------------------------

/// The bar paints v1's fill, and the rule along its top edge.
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
                y1: BOX.y0 + 0.5,
            },
            paint(resolved.divider_color),
        )),
        "and the rule along the top edge"
    );
}

/// The label is painted in the unchosen ink, at v1's size.
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
        paint(resolved.inactive_color),
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
        paint(resolved.active_color),
        "the chosen destination's label"
    );
}

/// v1's Cupertino destination sized its icon, and the port does too.
#[test]
fn the_icon_is_sized_as_v1_sized_it() {
    let resolved = kit().resolved_style();
    let bar = kit().labels(vec!["Home".into()]).icons(vec![icon()]).view();
    let host = bar.children()[0].build();
    let pressable = host.children().remove(0).build();
    let destination = pressable.children().remove(0).build();
    let mut children = destination.children();
    assert_eq!(children.len(), 2, "the icon and the label");
    let icon_slot = children.remove(0).layout;
    assert_eq!(
        icon_slot.width,
        Some(floom_core::Length::Px(resolved.icon_size)),
        "v1's `.size(icon_size, icon_size)`"
    );
}

/// v1's label was pushed down by four points.
#[test]
fn the_label_is_pushed_down_as_v1_pushed_it_down() {
    let bar = kit().labels(vec!["Home".into()]).view();
    let host = bar.children()[0].build();
    let pressable = host.children().remove(0).build();
    let destination = pressable.children().remove(0).build();
    let mut children = destination.children();
    let label_slot = children.remove(children.len() - 1).layout;
    let margin = label_slot.margin.expect("v1's `margin_top(4.0)`");
    assert_eq!(margin.top, 4.0);
    assert_eq!(margin.bottom, 0.0, "and only the top");
}

/// The pointer's fill appears while the pointer is over the destination, and
/// goes away when it leaves.
#[test]
fn the_pointer_s_fill_appears_only_while_the_pointer_is_over_the_destination() {
    let resolved = kit().resolved_style();
    let bar = kit().labels(vec!["Home".into(), "Search".into()]).view();

    let mut host = bar.children()[1].build();
    assert!(
        ops_of(&*host).is_empty(),
        "no fill before the pointer arrives"
    );

    pointer_over(&mut host, true);
    let fill = ops_of(&*host).iter().find_map(|op| match op {
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(
        fill,
        Some(paint(resolved.hover_color)),
        "v1's `with_alpha(on_surface, 12)` under the pointer"
    );

    pointer_over(&mut host, false);
    assert!(
        ops_of(&*host).is_empty(),
        "and no fill once the pointer has gone"
    );
}

/// The chosen destination keeps its own appearance: v1's hover fill did not
/// appear on the chosen destination.
#[test]
fn the_chosen_destination_is_not_filled_under_the_pointer() {
    let bar = kit()
        .labels(vec!["Home".into(), "Search".into()])
        .selected(1)
        .view();

    let mut host = bar.children()[1].build();
    pointer_over(&mut host, true);
    assert!(
        ops_of(&*host).is_empty(),
        "v1 filled the destination only when it was not the chosen one"
    );
}

// --- the structure --------------------------------------------------------

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

    let slot = bar.children()[0].layout;
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

/// Tapping a destination chooses it, and the label's ink follows.
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
        paint(resolved.active_color),
        "the destination that was tapped"
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

/// The theme: v1's Cupertino bottom navigation took a *snapshot* of its own
/// design system's theme, so a change of the app-wide theme does not reach it.
#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    // Not `kit()`: the helper hands the widget a theme of its own, which is
    assert_ne!(
        CupertinoBottomNavigation::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Cupertino bottom navigation took a snapshot, not the app-wide signal"
    );
}
