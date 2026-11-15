#![cfg(feature = "data_iterator")]
//! Cupertino data iterator: the container, the parts in it, and the rule between
//! them.

use floom_core::Widget;
use floom_types::{Color, Rect};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};
use floom_widgets::{Divider, Stack};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::{set_design_system, DesignSystem};
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_cupertino::widgets::data_iterator::{CupertinoDataIterator, CupertinoDataIteratorStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 200.0,
};

/// The Cupertino theme, with Cupertino selected.
fn theme() -> MultiTheme {
    set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

fn tokens() -> floom_core_theme::component_tokens::DataIteratorTokens {
    let theme = theme();
    match theme.component(ComponentId::DataIterator) {
        ComponentTokens::DataIterator(t) => t,
        _ => unreachable!(),
    }
}

fn three_items(kit: CupertinoDataIterator) -> CupertinoDataIterator {
    kit.item(|| Box::new(floom_widgets::Text::new("one")))
        .item(|| Box::new(floom_widgets::Text::new("two")))
        .item(|| Box::new(floom_widgets::Text::new("three")))
}

/// The style resolves the theme's colours and the token's numbers.
#[test]
fn the_style_resolves_the_theme_and_the_tokens() {
    let theme = theme();
    let style: CupertinoDataIteratorStyle = CupertinoDataIterator::new()
        .with_theme(theme.clone())
        .resolved_style();
    assert_eq!(style.design_system, "cupertino");
    assert_eq!(style.surface, theme.colors().surface);
    assert_eq!(style.outline, theme.colors().outline);
    assert_eq!(style.padding, tokens().padding);
    assert_eq!(style.item_gap, tokens().item_gap);
    assert!(style.divided, "v1's default is divided");
}

/// The container is padded, rounded and filled with the theme's surface.
#[test]
fn the_container_is_padded_rounded_and_filled() {
    let kit = three_items(
        CupertinoDataIterator::new()
            .with_theme(theme())
            .divided(true),
    );
    let style = kit.resolved_style();
    let surface = kit.view();
    let request = surface
        .layout_request()
        .expect("the container asks for its own space");
    assert_eq!(
        request.padding,
        Some(floom_types::Insets::uniform(f64::from(style.padding))),
        "the token's padding"
    );
    assert_eq!(
        surface.decoration().corner_radius(),
        8.0,
        "v1's literal 8.0"
    );
    assert_eq!(
        surface.decoration().background_color(),
        Some(to_engine(style.surface)),
        "and the theme's surface"
    );
}

/// The items are in the container, in order, and the rule is between them.
#[test]
fn the_items_are_in_order_with_a_rule_between_them() {
    let kit = three_items(
        CupertinoDataIterator::new()
            .with_theme(theme())
            .divided(true),
    );
    let surface = kit.view();
    let column = surface.children()[0].build();
    let column = column.as_any().downcast_ref::<Stack>().unwrap().clone();

    // Three items and two rules: the rule never comes before the first item.
    assert_eq!(column.child_count(), 5, "three items and two rules");
    let rules = (0..column.child_count())
        .filter(|i| column.children()[*i].build().as_any().is::<Divider>())
        .count();
    assert_eq!(rules, 2, "a rule between each pair of items");
    assert!(
        !column.children()[0].build().as_any().is::<Divider>(),
        "and never before the first item"
    );
}

/// Without `divided` there is no rule at all.
#[test]
fn an_undivided_iterator_has_no_rules() {
    let kit = three_items(
        CupertinoDataIterator::new()
            .with_theme(theme())
            .divided(false),
    );
    let surface = kit.view();
    let column = surface.children()[0].build();
    let column = column.as_any().downcast_ref::<Stack>().unwrap().clone();
    assert_eq!(column.child_count(), 3, "just the three items");
}

/// The header comes first and the footer last.
#[test]
fn the_header_comes_first_and_the_footer_last() {
    let kit = three_items(CupertinoDataIterator::new().with_theme(theme()))
        .header(|| Box::new(floom_widgets::Text::new("header")))
        .footer(|| Box::new(floom_widgets::Text::new("footer")));
    let surface = kit.view();
    let column = surface.children()[0].build();
    let column = column.as_any().downcast_ref::<Stack>().unwrap().clone();
    assert_eq!(
        column.child_count(),
        7,
        "the header, three items, the two rules between them, the footer"
    );
    let first = column.children()[0].build();
    let last = column.children()[6].build();
    assert!(
        first.as_any().is::<floom_widgets::Text>(),
        "the header is first"
    );
    assert!(
        last.as_any().is::<floom_widgets::Text>(),
        "and the footer is last"
    );
}

/// Design-system colour -> engine colour.
fn to_engine(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
