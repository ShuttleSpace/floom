//! The ported Fluent carousel -- one slide at a time, with arrows and dots.
//!
//! The shape is the engine's vocabulary, so what is tested here is the
//! translation and the pixels: the tokens reach the style, the area is
//! painted in v1's colours, the dots are painted in the colours v1 gave
//! them, and only the current slide is mounted.
//!
//! ## The tripwires
//!
//! `the_dots_are_painted_in_v1_s_colours` and
//! `only_the_current_slide_is_mounted` would fail if this kit stopped being
//! this kit.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{
    Child, ChildKey, ElementId, Event, LayoutRequest, Point, PointerEvent, Rect, Widget,
};
use floom_types::Color;
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder, TestEventContext,
};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_carousel::{FluentCarousel, FluentCarouselStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 400.0,
};

fn theme() -> MultiTheme {
    // `MultiTheme::new_for` builds all three palettes and reads the *current*
    // one, and `set_design_system` writes a thread-local, so the selection is
    // this test's alone.
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn kit() -> FluentCarousel {
    FluentCarousel::new().with_theme(theme())
}

/// A slide: a slot holding a piece of text.
fn slide(text: &str) -> Child {
    let text = text.to_string();
    Child::new(ChildKey::index(0), LayoutRequest::new(), move || {
        Box::new(floom_widgets::Text::new(text.clone()))
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

/// The area the carousel holds: the slides, the arrows and the dots.
fn area(kit: FluentCarousel) -> Box<dyn floom_core::WidgetAny> {
    let view = kit.view();
    view.children().remove(0).build()
}

/// One slide's presence, mounted and reconciled so that it holds its slide
/// when it is the current one.
fn slide_presence(widget: Box<dyn floom_core::WidgetAny>) -> Box<dyn floom_core::WidgetAny> {
    let mut presence = widget;
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = presence.create(&mut create);
    let mut cx = RecordingUpdateContext::new();
    presence.update(&mut cx, id);
    presence
}

/// The paint ops of one widget.
fn ops_of(widget: &dyn Widget) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

// --- the translation ------------------------------------------------------

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: FluentCarouselStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::Carousel)
    {
        floom_core_theme::component_tokens::ComponentTokens::Carousel(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(
        style.border_radius, tokens.border_radius,
        "the token's radius"
    );
    assert_eq!(style.height, tokens.height, "the token's height");
    assert_eq!(style.dot_size, tokens.dot_size, "the token's dot size");
    assert_eq!(style.dot_gap, tokens.dot_gap, "the token's dot gap");
    assert_eq!(
        style.dot_inactive_opacity, tokens.dot_inactive_opacity,
        "the token's inactive dot opacity"
    );
    assert_eq!(
        style.arrow_size, tokens.arrow_size,
        "the token's arrow size"
    );
    assert_eq!(style.slide_gap, tokens.slide_gap, "the token's slide gap");
    assert_eq!(style.surface, theme().colors().surface, "the area's fill");
    assert_eq!(
        style.on_surface,
        theme().colors().on_surface,
        "the arrows' ink"
    );
    assert_eq!(
        style.primary,
        theme().colors().primary,
        "the current dot's ink"
    );
    assert_eq!(
        style.outline,
        theme().colors().outline,
        "the outline's colour"
    );
    assert_eq!(
        style.arrow_font_size,
        tokens.arrow_size * 0.6,
        "v1's arrow size"
    );
}

/// The height is the prop's when it is positive, and the token's otherwise.
#[test]
fn the_height_is_the_prop_s_when_it_is_positive() {
    // The token's height, read from the token: the three kits' tokens do not
    // agree on it.
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::Carousel)
    {
        floom_core_theme::component_tokens::ComponentTokens::Carousel(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(
        kit().resolved_style().height,
        tokens.height,
        "the token's height"
    );
    assert_eq!(
        kit().height(300.0).resolved_style().height,
        300.0,
        "the prop's height wins"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "fluent");
}

// --- the pixels -----------------------------------------------------------

/// The area is painted the colours v1 painted.
#[test]
fn the_area_is_painted_v1_s_colours() {
    let resolved = kit().resolved_style();
    let view = kit().view();
    let ops = ops_of(&view);
    let fill = ops.iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(fill, Some(paint(resolved.surface)), "the area's fill");
    let outline = ops.iter().find_map(|op| match op {
        PaintOp::Path {
            stroke: Some((width, colour)),
            ..
        } => Some((*width, *colour)),
        _ => None,
    });
    let expected = paint(resolved.outline);
    let expected = Color {
        a: resolved.border_alpha,
        ..expected
    };
    assert_eq!(
        outline,
        Some((f64::from(resolved.border_width), expected)),
        "the outline"
    );
}

/// The dots are painted in v1's colours: the current one in the primary, the
/// others in the ink at the token's inactive opacity.
#[test]
fn the_dots_are_painted_in_v1_s_colours() {
    let resolved = kit().resolved_style();
    let area = area(kit().slides(vec![slide("One"), slide("Two")]));
    let mut children = area.children();
    let overlay = children.pop().expect("the dots' overlay").build();
    let dots_row = overlay.children().remove(0).build();
    let mut colours = Vec::new();
    for child in dots_row.children() {
        let pressable = child.build();
        let dot = pressable.children().remove(0).build();
        let colour = ops_of(&*dot).iter().find_map(|op| match op {
            PaintOp::RoundedRect { color, .. } => Some(*color),
            PaintOp::Rect { color, .. } => Some(*color),
            _ => None,
        });
        colours.push(colour);
    }
    assert_eq!(colours.len(), 2, "one dot per slide");
    let inactive = paint(resolved.on_surface);
    let inactive = Color {
        a: (resolved.dot_inactive_opacity * 255.0) as u8,
        ..inactive
    };
    assert_eq!(colours[0], Some(paint(resolved.primary)), "the current dot");
    assert_eq!(colours[1], Some(inactive), "the other dot");
}

/// The arrows are painted with v1's glyphs at v1's size.
#[test]
fn the_arrows_are_painted_with_v1_s_glyphs() {
    let resolved = kit().resolved_style();
    let area = area(kit().slides(vec![slide("One"), slide("Two")]));
    let children = area.children();
    // The two slides, then the two arrows, then the dots.
    let previous = children[2].build();
    let next = children[3].build();
    let previous_label = previous.children().remove(0).build();
    let next_label = next.children().remove(0).build();
    let previous_ops = ops_of(&*previous_label);
    let next_ops = ops_of(&*next_label);
    let (text, size, ink) = previous_ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("the previous arrow is painted");
    assert_eq!(text, "‹", "v1's previous glyph");
    assert_eq!(size, resolved.arrow_font_size, "v1's arrow size");
    assert_eq!(ink, paint(resolved.on_surface), "v1's arrow ink");
    let (text, ..) = next_ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Text { text, .. } => Some((text.clone(),)),
            _ => None,
        })
        .expect("the next arrow is painted");
    assert_eq!(text, "›", "v1's next glyph");
}

// --- the structure --------------------------------------------------------

/// Only the current slide is mounted.
#[test]
fn only_the_current_slide_is_mounted() {
    let area = area(kit().slides(vec![slide("One"), slide("Two"), slide("Three")]));
    let children = area.children();
    // The first slide is the current one.
    let first = slide_presence(children[0].build());
    assert_eq!(first.children().len(), 1, "the current slide is mounted");
    let second = slide_presence(children[1].build());
    assert!(
        second.children().is_empty(),
        "v1 hid the slides that were not showing"
    );
}

/// The arrows are only drawn when there is somewhere to go.
#[test]
fn the_arrows_are_only_drawn_when_there_is_somewhere_to_go() {
    let area = area(kit().slides(vec![slide("One")]));
    let children = area.children();
    // One slide, no arrows, and the dots' overlay.
    assert_eq!(children.len(), 2, "the slide and the dots");
}

/// The dots are only drawn when the caller asks for them.
#[test]
fn the_dots_are_only_drawn_when_the_caller_asks() {
    let area = area(kit().slides(vec![slide("One")]).show_dots(false));
    let children = area.children();
    assert_eq!(children.len(), 1, "the slide, and nothing else");
}

/// The next arrow moves the carousel on and says so.
#[test]
fn the_next_arrow_moves_the_carousel_on_and_says_so() {
    let changed = Arc::new(AtomicUsize::new(0));
    let counter = changed.clone();
    let kit = kit()
        .slides(vec![slide("One"), slide("Two")])
        .on_change(move |index| {
            assert_eq!(index, 1, "v1's callback carried the new index");
            counter.fetch_add(1, Ordering::SeqCst);
        });
    assert_eq!(kit.index(), 0, "the first slide is showing");
    let area = area(kit);
    let children = area.children();
    // The two slides, then the two arrows, then the dots.
    let mut next = children[3].build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = next.create(&mut create);
    let mut down = TestEventContext::new(
        id,
        BOX,
        Event::Pointer(PointerEvent::primary_down(Point::new(10.0, 10.0))),
    );
    next.event(&mut down, id);
    let mut up = TestEventContext::new(
        id,
        BOX,
        Event::Pointer(PointerEvent::primary_up(Point::new(10.0, 10.0))),
    );
    next.event(&mut up, id);
    assert_eq!(
        changed.load(Ordering::SeqCst),
        1,
        "v1's callback fired once"
    );
}

/// The props report the switches.
#[test]
fn the_props_report_the_switches() {
    let kit = kit().show_arrows(false).show_dots(false).auto_play(true);
    assert!(!kit.props().show_arrows);
    assert!(!kit.props().show_dots);
    assert!(kit.props().auto_play);
}

/// The theme: v1's Fluent carousel took a *snapshot* of its own design
/// system, so a change of the app-wide theme does not reach it.
#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    // Not `kit()`: the helper hands the widget a theme of its own, which is
    // exactly the thing this test is about.
    assert_ne!(
        FluentCarousel::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Fluent carousel took a snapshot, not the app-wide signal"
    );
}
