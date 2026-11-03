
use floom_core::{ElementId, Point, PointerEvent, Rect, Widget, WidgetAny};
use floom_types::{Color, FontWeight};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder, TestEventContext};
use floom_widgets::Pressable;

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_md_breadcrumbs::{MaterialBreadcrumbs, MaterialBreadcrumbsStyle};

const BOX: Rect = Rect {
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

fn kit() -> MaterialBreadcrumbs {
    MaterialBreadcrumbs::new().with_theme(theme())
}

fn trail_of(labels: &[&str]) -> MaterialBreadcrumbs {
    kit().items(labels.iter().map(|l| l.to_string()).collect())
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

/// The row of items and dividers inside the box.
fn trail_widget(kit: MaterialBreadcrumbs) -> Box<dyn WidgetAny> {
    let surface = kit.view();
    surface.children().remove(0).build()
}

/// One of the trail's slots: an item or a divider.
fn slot(kit: MaterialBreadcrumbs, index: usize) -> Box<dyn WidgetAny> {
    let trail = trail_widget(kit);
    trail.children().remove(index).build()
}

/// The text one of the trail's slots painted.
fn slot_text(kit: MaterialBreadcrumbs, index: usize) -> (String, f32, FontWeight, Color) {
    let widget = slot(kit, index);
    let text = if widget.as_any().is::<Pressable>() {
        widget.children().remove(0).build()
    } else {
        widget
    };
    let ops = ops_of(&*text);
    ops.iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text,
                size,
                weight,
                color,
                ..
            } => Some((text.clone(), *size, *weight, *color)),
            _ => None,
        })
        .expect("a breadcrumb paints its texts")
}

/// Taps one of the trail's items, the way the host would.
fn tap(kit: MaterialBreadcrumbs, index: usize) {
    let mut widget = slot(kit, index);
    let mut create = AllocatingCreateContext::new();
    let id = widget.create(&mut create);
    for phase in [PointerEvent::primary_down(AT), PointerEvent::primary_up(AT)] {
        let mut cx = TestEventContext::new(id, BOX, phase.into());
        widget.event(&mut cx, id);
    }
}


/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: MaterialBreadcrumbsStyle = kit().resolved_style();
    let tokens =
        match theme().component(floom_core_theme::component_tokens::ComponentId::Breadcrumbs) {
            floom_core_theme::component_tokens::ComponentTokens::Breadcrumbs(tokens) => tokens,
            _ => unreachable!(),
        };
    assert_eq!(style.gap, tokens.gap, "the token's gap");
    assert_eq!(style.font_size, tokens.font_size, "the token's font size");
    assert_eq!(style.divider, tokens.divider, "the token's divider");
    assert_eq!(
        style.on_surface,
        theme().colors().on_surface,
        "the current item's ink"
    );
    assert_eq!(
        style.on_surface_variant,
        theme().colors().on_surface_variant,
        "the other items' ink"
    );
    assert_eq!(
        style.outline_variant,
        theme().colors().outline_variant,
        "the divider's ink"
    );
}

#[test]
fn the_weights_the_paint_used_are_reported() {
    let style = kit().resolved_style();
    assert_eq!(style.current_font_weight, FontWeight::Medium, "v1's medium");
    assert_eq!(
        style.item_font_weight,
        FontWeight::Normal,
        "v1's regular, which the engine calls normal"
    );
    assert_eq!(
        style.divider_font_weight,
        FontWeight::Normal,
        "v1 wrote no weight for the divider at all"
    );
}

#[test]
fn the_padding_v1_wrote_as_a_literal_is_a_literal_here() {
    assert_eq!(
        kit().resolved_style().padding,
        8.0,
        "v1's `padding_horiz(8.0)`"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
}


/// The current item is painted in `on_surface`; the others are not.
#[test]
fn the_current_item_is_painted_in_on_surface_and_the_others_are_not() {
    let style = kit().resolved_style();
    let (text, size, weight, ink) = slot_text(trail_of(&["Home", "Docs", "Guide"]), 0);
    assert_eq!(text, "Home");
    assert_eq!(size, style.font_size, "the token's font size");
    assert_eq!(weight, style.item_font_weight, "v1's regular");
    assert_eq!(
        ink,
        paint(style.on_surface_variant),
        "an item that is not the current one"
    );

    let (text, _, weight, ink) = slot_text(trail_of(&["Home", "Docs", "Guide"]), 2);
    assert_eq!(text, "Docs");
    assert_eq!(weight, style.item_font_weight, "v1's regular");
    assert_eq!(
        ink,
        paint(style.on_surface_variant),
        "still not the current one"
    );

    let (text, _, weight, ink) = slot_text(trail_of(&["Home", "Docs", "Guide"]), 4);
    assert_eq!(text, "Guide");
    assert_eq!(weight, style.current_font_weight, "v1's medium");
    assert_eq!(
        ink,
        paint(style.on_surface),
        "the current item, in `on_surface`"
    );
}

/// The divider is painted in the outline variant, with the token's text.
#[test]
fn the_divider_is_painted_in_the_outline_variant() {
    let style = kit().resolved_style();
    let (text, size, _, ink) = slot_text(trail_of(&["Home", "Guide"]), 1);
    assert_eq!(text, style.divider, "the token's divider");
    assert_eq!(size, style.font_size, "the token's font size");
    assert_eq!(ink, paint(style.outline_variant), "v1's `outline_variant`");
}

#[test]
fn the_box_keeps_v1_s_padding() {
    let surface = kit().view();
    let insets = surface.padding_insets();
    assert_eq!(insets.left, 8.0);
    assert_eq!(insets.right, 8.0);
    assert_eq!(insets.top, 8.0);
    assert_eq!(insets.bottom, 8.0);
}


/// The trail is the items and the dividers between them: three items, two
/// dividers.
#[test]
fn the_trail_is_the_items_and_the_dividers_between_them() {
    let trail = trail_widget(trail_of(&["Home", "Docs", "Guide"]));
    assert_eq!(trail.children().len(), 5, "three items and two dividers");
}

#[test]
fn the_current_item_is_not_a_link() {
    assert!(
        slot(trail_of(&["Home", "Guide"]), 0)
            .as_any()
            .is::<Pressable>(),
        "an item that is not the current one is"
    );
    assert!(
        !slot(trail_of(&["Home", "Guide"]), 2)
            .as_any()
            .is::<Pressable>(),
        "v1 gave the current item no listener, so it is not a link"
    );
}

/// An item that is not the current one is.
#[test]
fn an_earlier_item_is_a_link() {
    assert!(
        slot(trail_of(&["Home", "Guide"]), 0)
            .as_any()
            .is::<Pressable>(),
        "v1 put a `Click` listener on every item but the last"
    );
}


/// Clicking an item reports its index.
#[test]
fn clicking_an_item_reports_its_index() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let make = || {
        let s = seen.clone();
        trail_of(&["Home", "Docs", "Guide"]).on_click(move |index| s.lock().unwrap().push(index))
    };
    tap(make(), 0);
    assert_eq!(seen.lock().unwrap().as_slice(), &[0], "the first item");
    tap(make(), 2);
    assert_eq!(seen.lock().unwrap().as_slice(), &[0, 1], "the second item");
}

/// The current item reports nothing: it is not a link.
#[test]
fn clicking_the_current_item_reports_nothing() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let s = seen.clone();
    let kit = trail_of(&["Home", "Guide"]).on_click(move |index| s.lock().unwrap().push(index));

    tap(kit, 2);
    assert!(
        seen.lock().unwrap().is_empty(),
        "v1 gave the current item no listener"
    );
}


#[test]
fn the_theme_is_the_app_wide_one() {
    assert_eq!(
        MaterialBreadcrumbs::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material breadcrumbs read the app-wide signal, not a snapshot"
    );
}
