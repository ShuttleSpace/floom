#![cfg(feature = "container")]
//! Fluent container: the two boxes, and what each of them carries.

use floom_core::{ElementId, Widget};
use floom_types::{Color, Length, Rect};
use floom_widgets::{Align, Stack, Surface};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_fluent::widgets::container::FluentContainer;

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 300.0,
};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn inner(container: FluentContainer) -> Box<dyn floom_core::WidgetAny> {
    let stack = container.view();
    stack.children()[0].build()
}

#[test]
fn the_outer_box_is_full_width_and_centres_its_content() {
    let stack = FluentContainer::new().with_theme(theme()).view();
    let request = stack
        .layout_request()
        .expect("a container asks for its own space");
    assert_eq!(
        request.width,
        Some(Length::Pct(100.0)),
        "v1's `width_pct(100.0)`"
    );
    assert_eq!(
        request.main_align,
        Some(Align::Center),
        "v1's `justify_center()`"
    );
}

#[test]
fn the_inner_box_carries_the_padding_the_fill_and_the_cap() {
    let style = FluentContainer::new()
        .with_theme(theme())
        .max_width(640.0)
        .padding(24.0)
        .resolved_style();
    let inner = inner(
        FluentContainer::new()
            .with_theme(theme())
            .max_width(640.0)
            .padding(24.0),
    );
    let surface = inner
        .as_any()
        .downcast_ref::<Surface>()
        .expect("the inner box is a surface");
    let request = surface
        .layout_request()
        .expect("a box asks for its own space");
    assert_eq!(
        request.padding,
        Some(floom_types::Insets::uniform(24.0)),
        "v1's `.padding(padding)`"
    );
    assert_eq!(request.max_width, Some(Length::Px(640.0)), "and the cap");
    assert_eq!(
        surface.decoration().background_color(),
        Some(to_paint(style.surface)),
        "and the fill"
    );
}

#[test]
fn a_fluid_container_drops_the_cap() {
    let inner = inner(
        FluentContainer::new()
            .with_theme(theme())
            .max_width(640.0)
            .padding(24.0)
            .fluid(true),
    );
    let surface = inner
        .as_any()
        .downcast_ref::<Surface>()
        .expect("the inner box is a surface");
    let request = surface
        .layout_request()
        .expect("a box asks for its own space");
    assert_eq!(
        request.max_width, None,
        "v1: the cap is applied only when the container is not fluid"
    );
    assert_eq!(
        request.padding,
        Some(floom_types::Insets::uniform(24.0)),
        "CONTROL: the padding is still there"
    );
}

#[test]
fn the_content_is_inside_the_inner_box() {
    let inner = inner(
        FluentContainer::new()
            .with_theme(theme())
            .child(|| Box::new(floom_widgets::Text::new("body"))),
    );
    let surface = inner
        .as_any()
        .downcast_ref::<Surface>()
        .expect("the inner box is a surface");
    assert_eq!(surface.child_count(), 1, "the caller's content is inside");
}

#[test]
fn the_style_resolves_the_themes_surface_and_ink() {
    let theme = theme();
    let style = FluentContainer::new()
        .with_theme(theme.clone())
        .resolved_style();
    assert_eq!(style.surface, theme.colors().surface);
    assert_eq!(style.content, theme.colors().on_surface);
}

fn to_paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
