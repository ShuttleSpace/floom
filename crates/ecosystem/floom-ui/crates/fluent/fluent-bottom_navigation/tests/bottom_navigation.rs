//! The ported Fluent bottom navigation -- a bar of destinations, one of them
//! chosen, with a rule along the top of the chosen one.
//!
//! The bar is the kit's own, because the engine has no bottom navigation (see
//! the kit's module documentation), so what is tested here is the translation
//! and the pixels: the tokens reach the style, the bar paints v1's fill and
//! v1's rule, and the chosen destination is painted in v1's ink with v1's rule
//! along its top edge.
//!
//! ## The tripwires
//!
//! `the_bar_paints_v1_s_fill_and_rule`,
//! `the_chosen_destination_is_painted_in_v1_s_ink` and
//! `the_chosen_destination_has_a_rule_along_its_top_edge` would fail if this
//! kit stopped being this kit.

use floom_core::{Child, ChildKey, ElementId, LayoutRequest, Point, PointerEvent, Rect, Widget, WidgetAny};
use floom_types::Color;
use floom_widgets::Pressable;
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, SceneRecorder, TestEventContext,
};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_bottom_navigation::{
    FluentBottomNavigation, FluentBottomNavigationBar, FluentBottomNavigationStyle, IconBuilder,
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
    // this test's alone.
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn kit() -> FluentBottomNavigation {
    FluentBottomNavigation::new().with_theme(theme())
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
fn label_of(bar: &FluentBottomNavigationBar, index: usize) -> Box<dyn WidgetAny> {
    let pressable = bar.children()[index].build();
    let destination = pressable.children().remove(0).build();
    let mut children = destination.children();
    // The label is the destination's last child; the icon, when there is one,
    // is the first.
    children.remove(children.len() - 1).build()
}

/// The text, the size and the ink one destination's label painted.
fn label_painted(bar: &FluentBottomNavigationBar, index: usize) -> (String, f32, Color) {
    let label = label_of(bar, index);
    ops_of(&*label)
        .iter()
        .find_map(|op| match op {
            PaintOp::Text { text, size, color, .. } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("a destination paints its label")
}

/// The destination's own box: the pressable and the content inside it.
fn destination_of(bar: &FluentBottomNavigationBar, index: usize) -> Box<dyn WidgetAny> {
    let pressable = bar.children()[index].build();
    pressable.children().remove(0).build()
}

/// Taps a destination, the way the host would.
fn tap(bar: &FluentBottomNavigationBar, index: usize) {
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

// --- the translation ------------------------------------------------------

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: FluentBottomNavigationStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::BottomNavigation) {
        floom_core_theme::component_tokens::ComponentTokens::BottomNavigation(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.height, tokens.height, "the token's bar height");
    assert_eq!(style.icon_size, tokens.icon_size, "the token's icon size");
    assert_eq!(style.label_font_size, tokens.label_font_size, "the token's label size");
    assert_eq!(style.gap, tokens.gap, "the token's gap");
    assert_eq!(style.background, theme().colors().surface, "the bar's fill");
    assert_eq!(style.selected_color, theme().colors().primary, "the chosen ink");
    assert_eq!(
        style.unselected_color,
        theme().colors().on_surface_variant,
        "the other destinations' ink"
    );
}

/// v1's bar drew its rule in `with_alpha(on_surface, 12)`: a colour v1's style
/// did not report, and the port does.
#[test]
fn the_rule_along_the_bar_is_the_one_v1_drew() {
    let style = kit().resolved_style();
    assert_eq!(style.divider_color.a, 12, "v1's `with_alpha(on_surface, 12)`");
    assert_eq!(style.divider_width, 1.0, "v1's `.border_top(1.0)`");
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "fluent");
}

/// The props report the destinations, and not the callback: the kit holds that
/// as an `Arc` of its own, because 2.0's slots are `Send + Sync` and the
/// props' `Box<dyn Fn(..)>` is not.
#[test]
fn the_props_report_the_destinations_and_not_the_callback() {
    let kit = kit().labels(vec!["Home".into(), "Search".into()]);
    assert_eq!(kit.props().labels, vec!["Home".to_string(), "Search".to_string()]);
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
                y1: BOX.y0 + 1.0,
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
        paint(resolved.unselected_color),
        "the destination that is not chosen"
    );
}

/// The chosen destination is painted in the chosen ink.
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

/// v1's Fluent destinations sized their icon, and the port does too.
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

/// The chosen destination has v1's rule along its top edge, and the others do
/// not.
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

// --- the structure --------------------------------------------------------

/// The bar is a row, as tall as the token says and as wide as its parent.
#[test]
fn the_bar_is_a_row_of_the_tokens_height() {
    let resolved = kit().resolved_style();
    let bar = kit().labels(vec!["Home".into(), "Search".into()]).view();
    let request = bar.layout_request().expect("the bar declares its own box");
    assert_eq!(request.axis, Some(floom_core::Axis::Horizontal));
    assert_eq!(request.height, Some(floom_core::Length::Px(resolved.height)));
    assert_eq!(request.width, Some(floom_core::Length::Pct(100.0)));
}

/// One destination per label, each growing to share the bar.
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
    let bar = kit()
        .labels(vec!["Home".into(), "Search".into()])
        .view();
    assert_eq!(bar.selected(), 0, "v1's first destination is chosen to begin with");
    tap(&bar, 1);
    assert_eq!(bar.selected(), 1);

    let (_, _, ink) = label_painted(&bar, 1);
    assert_eq!(
        ink,
        paint(resolved.selected_color),
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

/// The theme: v1's Fluent bottom navigation took a *snapshot* of its own design
/// system's theme, so a change of the app-wide theme does not reach it.
#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    // Not `kit()`: the helper hands the widget a theme of its own, which is
    // exactly the thing this test is about.
    assert_ne!(
        FluentBottomNavigation::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Fluent bottom navigation took a snapshot, not the app-wide signal"
    );
}
