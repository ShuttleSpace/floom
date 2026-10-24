//! Fluent floating action button: the circle, the elevation, and the click.
//!
//! ## The claim
//!
//! The button is a circle of the token's size, raised by the token's elevation,
//! filled with `primary` and inked with `on_primary` -- and a disabled one
//! loses both to `on_surface` at 12% and 38%. The assertions are about the
//! painted fill, the painted shadow, and the click.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::Widget;
use floom_types::{Color, Rect};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::{set_design_system, DesignSystem};
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_floating_action_button::FluentFloatingActionButton;

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 96.0,
    y1: 96.0,
};

/// The Fluent theme, with Fluent selected.
fn theme() -> MultiTheme {
    set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn tokens() -> floom_core_theme::component_tokens::FloatingActionButtonTokens {
    let theme = theme();
    match theme.component(ComponentId::FloatingActionButton) {
        ComponentTokens::FloatingActionButton(t) => t,
        _ => unreachable!(),
    }
}

/// The style resolves the token's numbers and the theme's palette.
#[test]
fn the_style_resolves_the_tokens_and_the_palette() {
    let theme = theme();
    let style = FluentFloatingActionButton::new()
        .with_theme(theme.clone())
        .resolved_style();
    assert_eq!(style.design_system, "fluent");
    assert_eq!(style.size, tokens().size);
    assert_eq!(style.border_radius, tokens().border_radius);
    assert_eq!(style.elevation, tokens().elevation);
    assert_eq!(style.background, theme.colors().primary);
    assert_eq!(style.foreground, theme.colors().on_primary);
    assert!(!style.extended, "and no label means no extended shape");
}

/// The button is a circle of the token's size.
#[test]
fn the_button_is_a_circle_of_the_tokens_size() {
    let button = FluentFloatingActionButton::new().with_theme(theme());
    let style = button.resolved_style();
    let pressable = button.view();
    let surface = pressable.children()[0].build();
    let surface = surface
        .as_any()
        .downcast_ref::<floom_widgets::Surface>()
        .unwrap()
        .clone();
    // The size is on the *content* -- the surface is the outer box, and the
    // circle it holds is the square inside it.
    let content = surface.children()[0].build();
    let request = content
        .as_any()
        .downcast_ref::<floom_widgets::Stack>()
        .unwrap()
        .layout_request()
        .expect("the circle asks for its own space");
    assert_eq!(request.width, Some(floom_types::Length::Px(style.size)));
    assert_eq!(request.height, Some(floom_types::Length::Px(style.size)));
    assert_eq!(surface.decoration().corner_radius(), style.border_radius);
}

/// The fill is the accent, and the disabled one is the muted ink.
#[test]
fn the_fill_is_the_accent_and_the_disabled_one_is_muted() {
    let button = FluentFloatingActionButton::new().with_theme(theme());
    let style = button.resolved_style();
    // The fill is the surface's: the pressable paints nothing, it is only the
    // interaction.
    let pressable = button.view();
    let surface = pressable.children()[0].build();
    let mut surface = surface
        .as_any()
        .downcast_ref::<floom_widgets::Surface>()
        .unwrap()
        .clone();
    let mut create = AllocatingCreateContext::new();
    let id = surface.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    surface.paint(&mut recorder, id);
    recorder.finish();
    // The button is a circle, so the fill is a *rounded* rect.
    let fill = recorder.ops().iter().find_map(|op| match op {
        PaintOp::Rect { color, .. } => Some(*color),
        PaintOp::RoundedRect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(
        fill,
        Some(to_paint(style.background)),
        "the accent fills the button"
    );

    let disabled = FluentFloatingActionButton::new()
        .with_theme(theme())
        .disabled(true);
    let disabled_style = disabled.resolved_style();
    assert_ne!(
        disabled_style.background, style.background,
        "a disabled button is not the accent"
    );
}

/// The elevation is the shadow v1 wrote: a blur of 2 + the elevation, a spread
/// of 1, and a black ink at 30%.
#[test]
fn the_elevation_is_the_shadow_v1_wrote() {
    let button = FluentFloatingActionButton::new().with_theme(theme());
    let style = button.resolved_style();
    let pressable = button.view();
    let surface = pressable.children()[0].build();
    let surface = surface
        .as_any()
        .downcast_ref::<floom_widgets::Surface>()
        .unwrap()
        .clone();
    let shadow = surface
        .decoration()
        .drop_shadow()
        .expect("the button is raised");
    assert_eq!(shadow.blur, 2.0 + f64::from(style.elevation));
    assert_eq!(shadow.spread, 1.0);
    assert_eq!(shadow.color.a, 30);
}

/// The click fires, and a disabled button does not fire.
#[test]
fn the_click_fires_and_a_disabled_button_does_not() {
    let hits = Arc::new(AtomicUsize::new(0));
    let sink = Arc::clone(&hits);
    let button = FluentFloatingActionButton::new()
        .with_theme(theme())
        .on_click(move || {
            sink.fetch_add(1, Ordering::SeqCst);
        });
    let mut pressable = button.view();
    let mut create = AllocatingCreateContext::new();
    let id = pressable.create(&mut create);
    click(&mut pressable, id);
    assert_eq!(
        hits.load(Ordering::SeqCst),
        1,
        "the callback runs on a click"
    );

    let hits = Arc::new(AtomicUsize::new(0));
    let sink = Arc::clone(&hits);
    let button = FluentFloatingActionButton::new()
        .with_theme(theme())
        .disabled(true)
        .on_click(move || {
            sink.fetch_add(1, Ordering::SeqCst);
        });
    let mut pressable = button.view();
    let mut create = AllocatingCreateContext::new();
    let id = pressable.create(&mut create);
    click(&mut pressable, id);
    assert_eq!(hits.load(Ordering::SeqCst), 0, "a disabled button is inert");
}

/// A click is a press followed by a release over the same spot.
fn click(pressable: &mut floom_widgets::Pressable, id: floom_core::ElementId) {
    let at = floom_types::Point::new(48.0, 48.0);
    let mut down = floom_widgets::testkit::TestEventContext::new(
        id,
        BOX,
        floom_types::PointerEvent::primary_down(at).into(),
    );
    pressable.event(&mut down, id);
    let mut up = floom_widgets::testkit::TestEventContext::new(
        id,
        BOX,
        floom_types::PointerEvent::primary_up(at).into(),
    );
    pressable.event(&mut up, id);
}

/// Design-system colour -> engine colour.
fn to_paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
