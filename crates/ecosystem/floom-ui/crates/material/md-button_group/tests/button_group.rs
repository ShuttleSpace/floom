use floom_core::Widget;
use floom_types::{Color, Insets};
use floom_widgets::{Axis, Stack, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_md_button_group::MaterialButtonGroup;

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn tokens() -> floom_core_theme::component_tokens::ButtonGroupTokens {
    let theme = theme();
    match theme.component(ComponentId::ButtonGroup) {
        ComponentTokens::ButtonGroup(t) => t,
        _ => unreachable!(),
    }
}

fn group(joined: bool) -> MaterialButtonGroup {
    MaterialButtonGroup::new()
        .with_theme(theme())
        .joined(joined)
        .child(|| Box::new(floom_widgets::Text::new("one")))
        .child(|| Box::new(floom_widgets::Text::new("two")))
}

/// The joined shape: no gap, the token's radius, no padding.
#[test]
fn the_joined_shape_has_no_gap_a_radius_and_no_padding() {
    let surface = group(true).view();
    let decoration = surface.decoration();
    assert_eq!(
        decoration.corner_radius(),
        tokens().border_radius,
        "v1: the radius is the token's when joined, and none otherwise"
    );
    assert_eq!(
        surface.padding_insets(),
        Insets::ZERO,
        "v1: no padding when joined, and 4dp otherwise"
    );

    let inner = surface.children()[0].build();
    let inner = inner.as_any().downcast_ref::<Stack>().unwrap().clone();
    let request = inner
        .layout_request()
        .expect("the buttons ask for their own space");
    assert_eq!(
        request.gap,
        Some(0.0),
        "v1: no gap when joined, the token's gap otherwise"
    );
    assert_eq!(inner.child_count(), 2, "both buttons are in it");
}

/// The unjoined shape: the token's gap, no radius, 4dp of padding.
#[test]
fn the_unjoined_shape_has_the_token_gap_no_radius_and_four_points_of_padding() {
    let surface = group(false).view();
    let decoration = surface.decoration();
    assert_eq!(
        decoration.corner_radius(),
        0.0,
        "v1: no radius when not joined"
    );
    assert_eq!(
        surface.padding_insets(),
        Insets::uniform(4.0),
        "v1: no padding when joined, and 4dp otherwise"
    );

    let inner = surface.children()[0].build();
    let inner = inner.as_any().downcast_ref::<Stack>().unwrap().clone();
    let request = inner
        .layout_request()
        .expect("the buttons ask for their own space");
    assert_eq!(request.gap, Some(tokens().gap), "v1: the token's gap");
}

/// The group is filled with the theme's surface variant.
#[test]
fn the_group_is_filled_with_the_surface_variant() {
    let theme = theme();
    let style = MaterialButtonGroup::new()
        .with_theme(theme.clone())
        .resolved_style();
    let surface = group(false).view();
    assert_eq!(
        surface.decoration().background_color(),
        Some(to_paint(style.surface_variant)),
        "v1: `background(t.colors().surface_variant)`"
    );
    assert_eq!(style.surface_variant, theme.colors().surface_variant);
}

/// A vertical group stacks its buttons down.
#[test]
fn a_vertical_group_stacks_its_buttons_down() {
    let surface = MaterialButtonGroup::new()
        .with_theme(theme())
        .vertical(true)
        .child(|| Box::new(floom_widgets::Text::new("one")))
        .view();
    let inner = surface.children()[0].build();
    let inner = inner.as_any().downcast_ref::<Stack>().unwrap().clone();
    let request = inner
        .layout_request()
        .expect("the buttons ask for their own space");
    assert_eq!(
        request.axis,
        Some(Axis::Vertical),
        "v1: `list(kids)` for vertical"
    );
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
