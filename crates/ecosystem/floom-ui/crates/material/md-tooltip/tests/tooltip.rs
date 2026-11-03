
use floom_core::{Child, ChildKey, ElementId, LayoutRequest, Point, PointerEvent, Rect, Widget};
use floom_types::{Color, PointerPhase};
use floom_ui_composites::Tooltip;
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder, TestEventContext,
};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_md_tooltip::{MaterialTooltip, MaterialTooltipStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 80.0,
};

const AT: Point = Point { x: 10.0, y: 10.0 };

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn kit() -> MaterialTooltip {
    MaterialTooltip::new().with_theme(theme())
}

/// The control the tooltip hangs off.
fn trigger() -> Child {
    Child::new(ChildKey::index(0), LayoutRequest::new(), || {
        Box::new(floom_widgets::Text::new("trigger"))
    })
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

/// The pointer arriving on the control, and the update pass that carries the
/// hover to the bubble.
fn arrive(tooltip: &mut Tooltip) {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = tooltip.create(&mut create);
    let mut enter = PointerEvent::primary_down(AT);
    enter.phase = PointerPhase::Enter;
    let mut cx = TestEventContext::new(id, BOX, enter.into());
    tooltip.event(&mut cx, id);

    let mut create = AllocatingCreateContext::new();
    let id: ElementId = tooltip.create(&mut create);
    let mut cx = RecordingUpdateContext::new();
    tooltip.update(&mut cx, id);
}

/// The fill and the ink one bubble painted.
fn bubble_painted(tooltip: &Tooltip) -> (Option<Color>, Option<Color>) {
    let bubble = tooltip.children().remove(1).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = bubble.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    bubble.paint(&mut recorder, id);
    recorder.finish();
    let fill = recorder.ops().iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });

    let label = bubble.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = label.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    label.paint(&mut recorder, id);
    recorder.finish();
    let ink = recorder.ops().iter().find_map(|op| match op {
        PaintOp::Text { color, .. } => Some(*color),
        _ => None,
    });

    (fill, ink)
}


/// The tokens and the colours reach the engine's style.
#[test]
fn the_tokens_and_colours_reach_the_engine_style() {
    let style: MaterialTooltipStyle = kit().resolved_style();
    let tokens = match floom_core_theme::theme::MultiTheme::new_for(DesignSystem::Material)
        .component(floom_core_theme::component_tokens::ComponentId::Tooltip)
    {
        floom_core_theme::component_tokens::ComponentTokens::Tooltip(tokens) => tokens,
        _ => unreachable!(),
    };

    assert_eq!(style.padding, tokens.padding, "the token's padding");
    assert_eq!(
        style.border_radius, tokens.border_radius,
        "the token's radius"
    );
    assert_eq!(style.font_size, tokens.font_size, "the token's text size");
    assert_eq!(style.text_color, theme().colors().surface, "the text's ink");
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
}

#[test]
fn the_gap_is_v1_s() {
    assert_eq!(
        kit().resolved_style().gap,
        4.0,
        "v1's `translate_y(4.0)`, a literal and not a token"
    );
}


#[test]
fn the_bubble_paints_v1_s_appearance() {
    let resolved = kit().resolved_style();
    let mut tooltip = kit().child(trigger()).text("Hello").view();
    arrive(&mut tooltip);

    let (fill, ink) = bubble_painted(&tooltip);
    assert_eq!(
        fill,
        Some(paint(resolved.bubble_color)),
        "the bubble's fill"
    );
    assert_eq!(ink, Some(paint(resolved.text_color)), "and the text's ink");
}

/// The bubble is up only while the pointer rests on the control.
#[test]
fn the_bubble_is_up_only_while_the_pointer_rests_on_the_control() {
    let mut tooltip = kit().child(trigger()).text("Hello").view();
    assert!(
        !tooltip.is_present(),
        "no bubble before the pointer arrives"
    );
    arrive(&mut tooltip);
    assert!(tooltip.is_hovered(), "the pointer arrived");
    assert!(tooltip.is_present(), "so the bubble is up");
}

/// The text is the tooltip's own.
#[test]
fn the_bubble_paints_the_tooltips_own_text() {
    let mut tooltip = kit().child(trigger()).text("Hello").view();
    arrive(&mut tooltip);
    let bubble = tooltip.children().remove(1).build();
    let label = bubble.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = label.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    label.paint(&mut recorder, id);
    recorder.finish();
    let (text, size) = recorder
        .ops()
        .iter()
        .find_map(|op| match op {
            PaintOp::Text { text, size, .. } => Some((text.clone(), *size)),
            _ => None,
        })
        .expect("the bubble paints its text");
    assert_eq!(text, "Hello");
    assert_eq!(
        size,
        kit().resolved_style().font_size,
        "at the token's size"
    );
}

/// The props report the text.
#[test]
fn the_props_report_the_text() {
    let kit = kit().text("Hello");
    assert_eq!(kit.props().text, "Hello");
}

#[test]
fn the_theme_is_the_app_wide_one() {
    assert_eq!(
        MaterialTooltip::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material tooltip read the app-wide signal, not a snapshot"
    );
}
