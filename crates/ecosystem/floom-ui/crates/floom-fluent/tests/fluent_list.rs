#![cfg(feature = "list")]
//! Fluent's list as a kit crate: the token translation, plus the numbers that

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color_ext::with_alpha;

use floom_fluent::widgets::list::{FluentList, FluentListStyle};

fn theme() -> MultiTheme {
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn style_of(list: &FluentList) -> FluentListStyle {
    list.resolved_style()
}

#[test]
fn the_container_radius_is_radius_md_not_hardcoded_eight() {
    let list = FluentList::new().with_theme(theme());
    let style = style_of(&list);
    assert_eq!(style.border_radius, theme().radius().md);
    assert_ne!(style.border_radius, 8.0);
}

#[test]
fn the_surface_is_the_theme_surface() {
    let list = FluentList::new().with_theme(theme());
    let style = style_of(&list);
    assert_eq!(style.surface_color, theme().colors().surface);
}

#[test]
fn the_text_color_is_on_surface() {
    let list = FluentList::new().with_theme(theme());
    let style = style_of(&list);
    assert_eq!(style.text_color, theme().colors().on_surface);
}

#[test]
fn the_hover_is_primary_at_eight_and_is_not_painted() {
    let list = FluentList::new().with_theme(theme());
    let style = style_of(&list);
    let expected = with_alpha(theme().colors().primary, 8);
    assert_eq!(style.hover_color, expected);
}

#[test]
fn the_item_height_comes_from_list_tokens() {
    let list = FluentList::new().with_theme(theme());
    let style = style_of(&list);
    let tokens = match theme().component(ComponentId::List) {
        ComponentTokens::List(t) => t,
        _ => unreachable!(),
    };
    assert_eq!(style.item_height, tokens.item_height);
}

#[test]
fn the_min_width_comes_from_list_tokens() {
    let list = FluentList::new().with_theme(theme());
    let style = style_of(&list);
    let tokens = match theme().component(ComponentId::List) {
        ComponentTokens::List(t) => t,
        _ => unreachable!(),
    };
    assert_eq!(style.min_width, tokens.min_width);
}

#[test]
fn the_design_system_tag_is_fluent() {
    let list = FluentList::new().with_theme(theme());
    let style = style_of(&list);
    assert_eq!(style.design_system, "fluent");
}

#[test]
fn the_view_builds_a_tree() {
    let list = FluentList::new()
        .with_theme(theme())
        .item("First", || {})
        .item("Second", || {});
    let _stack = list.view();
}
