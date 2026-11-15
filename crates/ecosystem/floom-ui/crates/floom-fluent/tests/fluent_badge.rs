#![cfg(feature = "badge")]
//! fluent's badge as a kit crate: the token translation, plus the

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

use floom_fluent::widgets::badge::{FluentBadge, FluentBadgeStyle};

fn theme() -> MultiTheme {
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn style_of(badge: &FluentBadge) -> FluentBadgeStyle {
    badge.resolved_style()
}

#[test]
fn the_bubble_colour_is_primary() {
    let badge = FluentBadge::new().with_theme(theme());
    let style = style_of(&badge);
    assert_eq!(style.bubble_color, theme().colors().primary);
}

#[test]
fn the_text_colour_is_on_primary() {
    let badge = FluentBadge::new().with_theme(theme());
    let style = style_of(&badge);
    assert_eq!(style.text_color, theme().colors().on_primary);
}

#[test]
fn the_font_size_comes_from_badge_tokens() {
    let badge = FluentBadge::new().with_theme(theme());
    let style = style_of(&badge);
    let tokens = match theme().component(ComponentId::Badge) {
        ComponentTokens::Badge(t) => t,
        _ => unreachable!(),
    };
    assert_eq!(style.font_size, tokens.font_size);
}

#[test]
fn the_min_size_comes_from_badge_tokens() {
    let badge = FluentBadge::new().with_theme(theme());
    let style = style_of(&badge);
    let tokens = match theme().component(ComponentId::Badge) {
        ComponentTokens::Badge(t) => t,
        _ => unreachable!(),
    };
    assert_eq!(style.min_size, tokens.min_size);
}

#[test]
fn the_padding_comes_from_badge_tokens() {
    let badge = FluentBadge::new().with_theme(theme());
    let style = style_of(&badge);
    let tokens = match theme().component(ComponentId::Badge) {
        ComponentTokens::Badge(t) => t,
        _ => unreachable!(),
    };
    assert_eq!(style.padding, tokens.padding);
}

#[test]
fn the_dot_size_comes_from_badge_tokens() {
    let badge = FluentBadge::new().with_theme(theme());
    let style = style_of(&badge);
    let tokens = match theme().component(ComponentId::Badge) {
        ComponentTokens::Badge(t) => t,
        _ => unreachable!(),
    };
    assert_eq!(style.dot_size, tokens.dot_size);
}

#[test]
fn the_design_system_tag_is_fluent() {
    let badge = FluentBadge::new().with_theme(theme());
    let style = style_of(&badge);
    assert_eq!(style.design_system, "fluent");
}

#[test]
fn the_view_builds_a_badge() {
    let badge = FluentBadge::new()
        .with_theme(theme())
        .content("3")
        .dot(false);
    let _b = badge.view();
}

#[test]
fn a_dot_builds_without_content() {
    let badge = FluentBadge::new().with_theme(theme()).dot(true);
    let _b = badge.view();
}
