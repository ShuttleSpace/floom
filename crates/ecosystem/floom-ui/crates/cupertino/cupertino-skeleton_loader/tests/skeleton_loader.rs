//! The ported Cupertino skeleton loader -- the grey placeholder shown while
//! content loads.

use floom_core::{ElementId, Length, Rect, Widget, WidgetAny};
use floom_types::Color;
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::skeleton_loader::SkeletonType;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_cupertino_skeleton_loader::{CupertinoSkeletonLoader, CupertinoSkeletonStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 400.0,
};

fn theme() -> MultiTheme {
    // `MultiTheme::new_for` builds all three palettes and reads the *current*
    // one, and `set_design_system` writes a thread-local, so the selection is
    floom_core_theme::global_theme::set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

fn kit() -> CupertinoSkeletonLoader {
    CupertinoSkeletonLoader::new().with_theme(theme())
}

fn tokens() -> floom_core_theme::component_tokens::SkeletonTokens {
    match theme().component(ComponentId::SkeletonLoader) {
        ComponentTokens::SkeletonLoader(t) => t,
        _ => unreachable!(),
    }
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

/// The placeholder inside the outer box.
fn content_of(kit: CupertinoSkeletonLoader) -> Box<dyn WidgetAny> {
    let outer = kit.view();
    outer.children().remove(0).build()
}

// --- the translation ------------------------------------------------------

/// The tokens reach the style.
#[test]
fn the_tokens_reach_the_style() {
    let style: CupertinoSkeletonStyle = kit().resolved_style();
    let tokens = tokens();
    assert_eq!(style.color, tokens.color, "the placeholder's fill");
    assert_eq!(
        style.corner_radius, tokens.corner_radius,
        "the corner radius"
    );
    assert_eq!(style.text_height, tokens.text_height, "the line height");
    assert_eq!(
        style.avatar_size, tokens.avatar_size,
        "the avatar's diameter"
    );
    assert_eq!(style.gap, tokens.gap, "the gap");
    assert_eq!(style.card_radius, tokens.card_radius, "the card's radius");
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "cupertino");
}

/// The outer width is the type's own: a card is 200 wide, a rectangle 160,
/// and anything else 240.
#[test]
fn the_outer_width_is_the_types_own() {
    for (skeleton_type, width) in [
        (SkeletonType::Card, 200.0),
        (SkeletonType::Rect, 160.0),
        (SkeletonType::Text, 240.0),
        (SkeletonType::Circle, 240.0),
        (SkeletonType::Avatar, 240.0),
    ] {
        let style = kit().skeleton_type(skeleton_type).resolved_style();
        assert_eq!(
            style.outer_width, width,
            "v1's default for {skeleton_type:?}"
        );
    }
}

/// An explicit width wins over the type's default.
#[test]
fn an_explicit_width_wins() {
    let style = kit()
        .skeleton_type(SkeletonType::Card)
        .width(300.0)
        .resolved_style();
    assert_eq!(style.outer_width, 300.0);
}

// --- the pixels -----------------------------------------------------------

/// The circle is a circle of the avatar's size.
#[test]
fn the_circle_is_a_circle() {
    let style = kit().skeleton_type(SkeletonType::Circle).resolved_style();
    let circle = content_of(kit().skeleton_type(SkeletonType::Circle));

    let request = circle
        .layout_request()
        .expect("the circle asks for its own space");
    assert_eq!(
        request.width,
        Some(Length::Px(style.avatar_size)),
        "v1's circle was the avatar's size"
    );
    assert_eq!(request.height, Some(Length::Px(style.avatar_size)));

    let ops = ops_of(&*circle);
    let (fill, radius) = ops
        .iter()
        .find_map(|op| match op {
            PaintOp::RoundedRect { color, radius, .. } => Some((*color, *radius)),
            _ => None,
        })
        .expect("the circle is painted");
    assert_eq!(fill, paint(style.color), "v1's placeholder colour");
    assert_eq!(radius, 9999.0, "9999 is a circle whatever the size");
}

/// The rectangle is two text lines tall.
#[test]
fn the_rectangle_is_two_lines_tall() {
    let style = kit().skeleton_type(SkeletonType::Rect).resolved_style();
    let rect = content_of(kit().skeleton_type(SkeletonType::Rect));

    let request = rect
        .layout_request()
        .expect("the rectangle asks for its own space");
    assert_eq!(request.width, Some(Length::Px(160.0)), "v1's width");
    assert_eq!(
        request.height,
        Some(Length::Px(style.text_height * 2.0)),
        "v1's height: two text lines"
    );

    let ops = ops_of(&*rect);
    let radius = ops
        .iter()
        .find_map(|op| match op {
            PaintOp::RoundedRect { radius, .. } => Some(*radius),
            _ => None,
        })
        .expect("the rectangle is painted");
    assert_eq!(radius, style.corner_radius, "the token's corner radius");
}

/// The card is the card's size, with the card's radius.
#[test]
fn the_card_is_the_card_size() {
    let style = kit().skeleton_type(SkeletonType::Card).resolved_style();
    let card = content_of(kit().skeleton_type(SkeletonType::Card));

    let request = card
        .layout_request()
        .expect("the card asks for its own space");
    assert_eq!(request.width, Some(Length::Px(200.0)), "v1's width");
    assert_eq!(request.height, Some(Length::Px(120.0)), "v1's height");

    let ops = ops_of(&*card);
    let radius = ops
        .iter()
        .find_map(|op| match op {
            PaintOp::RoundedRect { radius, .. } => Some(*radius),
            _ => None,
        })
        .expect("the card is painted");
    assert_eq!(radius, style.card_radius, "the card's own radius");
}

// --- the structure --------------------------------------------------------

/// The text placeholder is a column of lines.
#[test]
fn the_text_is_a_column_of_lines() {
    let style = kit()
        .skeleton_type(SkeletonType::Text)
        .lines(3)
        .resolved_style();
    let column = content_of(kit().skeleton_type(SkeletonType::Text).lines(3));
    assert_eq!(
        column.children().len(),
        3,
        "v1 drew one line per line asked for"
    );

    let line = column.children().remove(0).build();
    let request = line
        .layout_request()
        .expect("a line asks for its own space");
    assert_eq!(
        request.height,
        Some(Length::Px(style.text_height)),
        "and it is one text line tall"
    );
    assert_eq!(
        request.width,
        Some(Length::Pct(100.0)),
        "and as wide as the column"
    );
}

/// The avatar is a circle and at most three lines.
#[test]
fn the_avatar_is_a_circle_and_at_most_three_lines() {
    let row = content_of(kit().skeleton_type(SkeletonType::Avatar).lines(9));
    assert_eq!(row.children().len(), 2, "the circle and the lines");

    let column = row.children().remove(1).build();
    assert_eq!(
        column.children().len(),
        3,
        "v1 drew at most three lines beside the avatar"
    );
}

/// `animated` changes nothing: nothing shimmers.
#[test]
fn the_animated_prop_changes_nothing() {
    let still = ops_of(&*content_of(
        kit().skeleton_type(SkeletonType::Text).animated(false),
    ));
    let moving = ops_of(&*content_of(
        kit().skeleton_type(SkeletonType::Text).animated(true),
    ));
    assert_eq!(
        still, moving,
        "v1's build had no animation frame hook, so nothing shimmered"
    );
}

/// The props report what the caller set.
#[test]
fn the_props_report_what_the_caller_set() {
    let kit = kit()
        .skeleton_type(SkeletonType::Avatar)
        .width(64.0)
        .height(48.0)
        .lines(2)
        .animated(false);
    let props = kit.props();
    assert_eq!(props.skeleton_type, SkeletonType::Avatar);
    assert_eq!(props.width, Some(64.0));
    assert_eq!(props.height, Some(48.0));
    assert_eq!(props.lines, 2);
    assert!(!props.animated);
}
