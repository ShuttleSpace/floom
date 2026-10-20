//! Cupertino responsive box: the ratio, and the fill.
//!
//! ## The claim
//!
//! A responsive box keeps a width-to-height ratio, and 2.0's `LayoutRequest`
//! grew `aspect_ratio` so that it can say so. The assertions are about the
//! request the box makes and the fill it paints.

use floom_core::Widget;
use floom_types::{Color, Length, Rect};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::global_theme::{DesignSystem, set_design_system};
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_cupertino_responsive::CupertinoResponsive;

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 320.0,
    y1: 180.0,
};

/// The Cupertino theme, with Cupertino selected.
fn theme() -> MultiTheme {
    set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

/// The ratio reaches the layout request, which is the whole point of the
/// component: a fixed height would be wrong at every width but one.
#[test]
fn the_ratio_reaches_the_layout_request() {
    let surface = CupertinoResponsive::new(16.0 / 9.0).with_theme(theme()).view();
    let request = surface.layout_request().expect("the box asks for its own space");
    assert_eq!(
        request.aspect_ratio,
        Some(16.0 / 9.0),
        "the ratio, as stated"
    );
    assert_eq!(
        request.width,
        Some(Length::Pct(100.0)),
        "and the box is as wide as its parent"
    );
}

/// The ratio is on the props, so a caller can read it back.
#[test]
fn the_ratio_is_on_the_props() {
    let kit = CupertinoResponsive::new(4.0 / 3.0);
    assert_eq!(kit.props().aspect_ratio, 4.0 / 3.0);
}

/// The box is filled with the theme's surface variant.
#[test]
fn the_box_is_filled_with_the_surface_variant() {
    let kit = CupertinoResponsive::new(1.0).with_theme(theme());
    let mut surface = kit.view();
    let mut create = AllocatingCreateContext::new();
    let id = surface.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    surface.paint(&mut recorder, id);
    recorder.finish();

    let fill = recorder.ops().iter().find_map(|op| match op {
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(
        fill,
        Some(to_engine(theme().colors().surface_variant)),
        "v1's `background(surface_variant)`"
    );
}

/// The content goes inside the box.
#[test]
fn the_content_goes_inside_the_box() {
    let surface = CupertinoResponsive::new(1.0)
        .with_theme(theme())
        .child(|| Box::new(floom_widgets::Text::new("media")))
        .view();
    assert_eq!(surface.child_count(), 1, "the caller's content is inside");
    assert!(
        surface.children()[0].build().as_any().is::<floom_widgets::Text>(),
        "and it is the caller's own content"
    );
}

/// Design-system colour -> engine colour.
fn to_engine(c: floom_core_theme::tokens::color::Color) -> Color {
    Color { r: c.r, g: c.g, b: c.b, a: c.a }
}
