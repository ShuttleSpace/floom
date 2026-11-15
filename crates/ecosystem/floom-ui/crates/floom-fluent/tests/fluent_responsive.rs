#![cfg(feature = "responsive")]
//! Fluent responsive box: the ratio, and the fill.

use floom_core::Widget;
use floom_types::{Color, Length, Rect};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::global_theme::{set_design_system, DesignSystem};
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_fluent::widgets::responsive::FluentResponsive;

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 320.0,
    y1: 180.0,
};

fn theme() -> MultiTheme {
    set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

#[test]
fn the_ratio_reaches_the_layout_request() {
    let surface = FluentResponsive::new(16.0 / 9.0).with_theme(theme()).view();
    let request = surface
        .layout_request()
        .expect("the box asks for its own space");
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

#[test]
fn the_ratio_is_on_the_props() {
    let kit = FluentResponsive::new(4.0 / 3.0);
    assert_eq!(kit.props().aspect_ratio, 4.0 / 3.0);
}

#[test]
fn the_box_is_filled_with_the_surface_variant() {
    let kit = FluentResponsive::new(1.0).with_theme(theme());
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

#[test]
fn the_content_goes_inside_the_box() {
    let surface = FluentResponsive::new(1.0)
        .with_theme(theme())
        .child(|| Box::new(floom_widgets::Text::new("media")))
        .view();
    assert_eq!(surface.child_count(), 1, "the caller's content is inside");
    assert!(
        surface.children()[0]
            .build()
            .as_any()
            .is::<floom_widgets::Text>(),
        "and it is the caller's own content"
    );
}

fn to_engine(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
