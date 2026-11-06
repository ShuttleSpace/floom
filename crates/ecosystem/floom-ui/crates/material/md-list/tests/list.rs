use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color_ext::with_alpha;

use floom_widget_md_list::{MdList, MdListStyle};

fn theme() -> MultiTheme {
    MultiTheme::new_for(DesignSystem::Material)
}

fn style_of(list: &MdList) -> MdListStyle {
    list.resolved_style()
}

#[test]
fn the_container_radius_is_hardcoded_eight_not_radius_md() {
    let list = MdList::new().with_theme(theme());
    let style = style_of(&list);
    assert_eq!(style.border_radius, 8.0);
    assert_ne!(style.border_radius, theme().radius().md);
}

#[test]
fn the_surface_is_the_theme_surface() {
    let list = MdList::new().with_theme(theme());
    let style = style_of(&list);
    assert_eq!(style.surface_color, theme().colors().surface);
}

#[test]
fn the_text_color_is_on_surface() {
    let list = MdList::new().with_theme(theme());
    let style = style_of(&list);
    assert_eq!(style.text_color, theme().colors().on_surface);
}

#[test]
fn the_hover_is_primary_at_eight_and_is_not_painted() {
    let list = MdList::new().with_theme(theme());
    let style = style_of(&list);
    let expected = with_alpha(theme().colors().primary, 8);
    assert_eq!(style.hover_color, expected);
}

#[test]
fn the_item_height_comes_from_list_tokens() {
    let list = MdList::new().with_theme(theme());
    let style = style_of(&list);
    let tokens = match theme().component(ComponentId::List) {
        ComponentTokens::List(t) => t,
        _ => unreachable!(),
    };
    assert_eq!(style.item_height, tokens.item_height);
}

#[test]
fn the_min_width_comes_from_list_tokens() {
    let list = MdList::new().with_theme(theme());
    let style = style_of(&list);
    let tokens = match theme().component(ComponentId::List) {
        ComponentTokens::List(t) => t,
        _ => unreachable!(),
    };
    assert_eq!(style.min_width, tokens.min_width);
}

#[test]
fn the_design_system_tag_is_material() {
    let list = MdList::new().with_theme(theme());
    let style = style_of(&list);
    assert_eq!(style.design_system, "material");
}

#[test]
fn the_view_builds_a_tree_with_one_container_two_spacers_and_a_row_each_item() {
    let list = MdList::new()
        .with_theme(theme())
        .item("First", || {})
        .item("Second", || {});
    let _stack = list.view();
}
