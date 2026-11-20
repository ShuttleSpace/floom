#![cfg(feature = "progress_circular")]
//! Fluent circular progress: the ring, and the two numbers this kit owns.

use floom_core::Widget;
use floom_types::{Color, Length, Rect};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::progress_circular::ProgressCircularProps;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_fluent::widgets::progress_circular::{
    FluentProgressCircular, FluentProgressCircularStyle,
};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 48.0,
    y1: 48.0,
};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn tokens() -> floom_core_theme::component_tokens::ProgressCircularTokens {
    let theme = theme();
    match theme.component(ComponentId::ProgressCircular) {
        ComponentTokens::ProgressCircular(t) => t,
        _ => unreachable!(),
    }
}

fn ring(p: FluentProgressCircular) -> Vec<PaintOp> {
    let mut p = p.view();
    let mut create = AllocatingCreateContext::new();
    let id = p.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    p.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

#[test]
fn the_style_resolves_this_kits_tokens_and_colours() {
    let theme = theme();
    let style: FluentProgressCircularStyle = FluentProgressCircular::new()
        .with_theme(theme.clone())
        .resolved_style();
    assert_eq!(
        style.size,
        ProgressCircularProps::default().size,
        "the *props'* size, which is what the rule below actually reads"
    );
    assert_eq!(style.stroke_width, tokens().stroke_width);
    assert_eq!(style.track_color, theme.colors().surface_variant);
    assert_eq!(style.fill_color, theme.colors().primary);
    assert_eq!(style.design_system, "fluent");
}

#[test]
fn the_props_size_wins_over_the_tokens() {
    let style = FluentProgressCircular::new()
        .with_theme(theme())
        .size(64.0)
        .resolved_style();
    assert_eq!(style.size, 64.0, "v1: the props' size when it is set");

    let style = FluentProgressCircular::new()
        .with_theme(theme())
        .resolved_style();
    assert_eq!(
        style.size, 36.0,
        "the props' own default, which is why the token's size is never read"
    );
    assert_eq!(
        style.size,
        ProgressCircularProps::default().size,
        "CONTROL: the props' own default, and not the token"
    );
}

#[test]
fn the_indeterminate_arc_is_the_shape_this_kit_asks_for() {
    let style = FluentProgressCircular::new()
        .with_theme(theme())
        .resolved_style();
    assert_eq!(
        style.indeterminate_fraction, 0.25,
        "the fraction of the ring the indeterminate arc covers"
    );

    let ring = FluentProgressCircular::new()
        .with_theme(theme())
        .indeterminate(true)
        .view();
    assert!(
        ring.value().is_none(),
        "and the ring itself draws the indeterminate arc"
    );
    assert!(
        (ring.sweep_rad() - std::f64::consts::TAU * 0.25).abs() < 1e-9,
        "sweeping that fraction: {}",
        ring.sweep_rad()
    );
}

#[test]
fn the_determinate_arc_starts_at_twelve_oclock() {
    let ring = FluentProgressCircular::new()
        .with_theme(theme())
        .value(50.0)
        .view();
    assert_eq!(
        ring.value(),
        Some(0.5),
        "v1's percentage is the engine's fraction"
    );
    assert!(
        (ring.start_rad() - -std::f64::consts::FRAC_PI_2).abs() < 1e-9,
        "and it starts at twelve o'clock: {}",
        ring.start_rad()
    );
}

#[test]
fn the_ring_is_a_track_and_an_arc() {
    let ops = ring(
        FluentProgressCircular::new()
            .with_theme(theme())
            .value(50.0),
    );
    let strokes: Vec<(f64, Color, usize)> = ops
        .iter()
        .filter_map(|op| match op {
            PaintOp::Path {
                stroke: Some((width, color)),
                segments,
                ..
            } => Some((*width, *color, *segments)),
            _ => None,
        })
        .collect();
    assert_eq!(strokes.len(), 2, "the track and the arc");
    let theme = theme();
    assert_eq!(
        strokes[0].1,
        to_paint(theme.colors().surface_variant),
        "the track"
    );
    assert_eq!(strokes[1].1, to_paint(theme.colors().primary), "the arc");
}

#[test]
fn the_ring_is_the_props_size() {
    let ring = FluentProgressCircular::new().with_theme(theme()).view();
    let request = ring
        .layout_request()
        .expect("a ring asks for its own space");
    assert_eq!(request.width, Some(Length::Px(36.0)));
    assert_eq!(request.height, Some(Length::Px(36.0)));
}

fn to_paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
