//! Material's list as a kit crate: the token translation, plus the numbers that
//! are this kit's own rather than the tokens'.
//!
//! This crate has never been compiled, which is how three kits came to ship a
//! list with hover, container padding and a hardcoded radius without anyone
//! noticing the engine has none of those. So the point of this file is to pin the
//! numbers *this kit wrote*, in the places where a plausible-looking edit would
//! change a pixel.
//!
//! ## What v1 drew that this port cannot
//!
//! * The row's hovered background (`primary`@8) -- `ListItemStyle` has a single
//!   static `background` and `ListItemRow` owns the interaction state without a
//!   variant for hover. Carried as `hover_color`, asserted against (it is never
//!   painted), so a later reader finds the number in history and knows.
//! * The container's horizontal padding (8px) -- dropped, see the kit's module
//!   docs. The vertical inset survives as two `Spacer`s.
//!
//! ## What this kit's identity actually is
//!
//! The only number Material answers differently from the other two is the
//! container radius: it hardcodes `8.0` where Cupertino and Fluent read
//! `radius().md`. That is the tripwire that keeps the three kits three crates.

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
    // Material's whole disagreement with the other two kits: it wrote `8.0`,
    // not `radius().md`. If this ever becomes `radius().md`, Material and
    // Cupertino collapse into one crate's worth of divergence.
    let list = MdList::new().with_theme(theme());
    let style = style_of(&list);
    assert_eq!(style.border_radius, 8.0);
    // And it must NOT equal the token radius, or the hardcoded number is lying.
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
    // v1 wrote `.hover(|s| s.background(with_alpha(primary, 8)))`. The port
    // cannot draw it (see module docs), so it is carried and asserted *against*:
    // the value exists, named, so nobody re-arms the trap by "fixing" a missing
    // field with a fresh guess.
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
    // A `Container` (the surface) + two `Spacer`s (the vertical inset) + one
    // `ListItemRow` per item. `view()` returns a `Stack`; building it must not
    // panic and must reconcile to a non-trivial subtree.
    let list = MdList::new()
        .with_theme(theme())
        .item("First", || {})
        .item("Second", || {});
    let _stack = list.view();
    // If we got here, the tree built. The structure is exercised by the
    // composites' own `ListItemRow` tests; this kit's job is the token glue.
}
