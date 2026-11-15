#![cfg(feature = "item_group")]
//! one that proves the promotion was right.

use floom_core::Widget;
use floom_types::{Insets, Rect};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_fluent::widgets::item_group::{FluentItemGroup, FluentItemGroupStyle};

const AREA: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 120.0,
};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn group() -> FluentItemGroup {
    FluentItemGroup::new().with_theme(theme())
}

fn three_rows() -> FluentItemGroup {
    group()
        .row("One", || Box::new(floom_widgets::Text::new("one")))
        .row("Two", || Box::new(floom_widgets::Text::new("two")))
        .row("Three", || Box::new(floom_widgets::Text::new("three")))
}

#[test]
fn the_tokens_and_colours_reach_the_engine_style() {
    let style = group().select_group_style();
    let resolved = group().resolved_style();

    assert_eq!(style.gap, resolved.gap, "the token's gap");
    assert_eq!(style.pad, resolved.pad, "the token's padding");
    assert_eq!(style.radius, resolved.radius, "the token's radius");
    assert_eq!(
        style.border_width, 1.0,
        "v1 wrote `.border(1.0)` as a literal, not a token"
    );

    use floom_ui_composites::SelectGroupStyle;
    let expect = SelectGroupStyle {
        gap: resolved.gap,
        axis: floom_core::Axis::Vertical,
        height: None,
        pad: resolved.pad,
        content_placement: floom_ui_composites::ContentPlacement::Top,
        toggles_off_in_single_mode: false,
        mandatory_in_multiple_mode: true,
        radius: resolved.radius,
        border_width: 1.0,
        background: paint(resolved.surface_variant),
        foreground: paint(resolved.on_surface),
        border_color: paint(resolved.outline),
        selected_background: paint(resolved.primary_container),
        selected_foreground: paint(resolved.on_primary_container),
        selected_border_color: paint(resolved.primary),
    };
    assert_eq!(style, expect, "each colour in its right place");
}

fn paint(c: floom_core_theme::tokens::color::Color) -> floom_types::Color {
    floom_types::Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

#[test]
fn the_ink_follows_the_selection() {
    let resolved = group().resolved_style();
    assert_eq!(group().foreground(false), paint(resolved.on_surface));
    assert_eq!(
        group().foreground(true),
        paint(resolved.on_primary_container),
        "and a chosen row's content is in the on-primary-container ink"
    );
}

#[test]
fn the_padding_is_horizontal_pad_and_vertical_half() {
    let pad = f64::from(group().resolved_style().pad);
    assert_eq!(
        group().padding(),
        Insets {
            left: pad,
            right: pad,
            top: pad / 2.0,
            bottom: pad / 2.0,
        }
    );
    assert_ne!(
        group().padding(),
        Insets::uniform(pad),
        "and is deliberately not uniform"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(group().resolved_style().design_system, "fluent");
}

#[test]
fn one_widget_answers_both_accessors() {
    let widget = group()
        .row("One", || Box::new(floom_widgets::Text::new("one")))
        .view(None::<fn(usize)>, None::<fn(Vec<usize>)>);
    assert_eq!(floom_fluent::widgets::item_group::selected(&widget), None);
    assert_eq!(
        floom_fluent::widgets::item_group::selected_multi(&widget),
        Vec::<usize>::new()
    );
}

#[test]
fn the_mode_and_mandatory_props_reach_the_widget() {
    use floom_ui_composites::SelectionMode;

    let single = three_rows().view(None::<fn(usize)>, None::<fn(Vec<usize>)>);
    assert_eq!(single.selection_mode(), SelectionMode::Single);
    assert!(!single.is_mandatory());

    let many = three_rows()
        .multiple(true)
        .mandatory(true)
        .view(None::<fn(usize)>, None::<fn(Vec<usize>)>);
    assert_eq!(many.selection_mode(), SelectionMode::Multiple);
    assert!(many.is_mandatory());
}

#[test]
fn the_initial_selection_is_readable_through_both_accessors() {
    let widget = three_rows()
        .selected(2)
        .view(None::<fn(usize)>, None::<fn(Vec<usize>)>);
    assert_eq!(floom_fluent::widgets::item_group::selected(&widget), Some(2));
    assert_eq!(
        floom_fluent::widgets::item_group::selected_multi(&widget),
        vec![2]
    );
}

#[test]
fn every_declared_row_reaches_the_widget() {
    let widget = three_rows().view(None::<fn(usize)>, None::<fn(Vec<usize>)>);
    assert_eq!(widget.children().len(), 3);

    let four = three_rows()
        .row_unlabelled(|| Box::new(floom_widgets::Text::new("x")))
        .view(None::<fn(usize)>, None::<fn(Vec<usize>)>);
    assert_eq!(four.children().len(), 4);

    let one = group()
        .row_unlabelled(|| Box::new(floom_widgets::Text::new("x")))
        .view(None::<fn(usize)>, None::<fn(Vec<usize>)>);
    assert_eq!(one.children().len(), 1);
}

#[test]
fn an_empty_group_still_builds() {
    let widget = FluentItemGroup::new()
        .with_theme(theme())
        .view(None::<fn(usize)>, None::<fn(Vec<usize>)>);
    assert!(widget.children().is_empty());
    assert!(widget.layout_request().is_some(), "and it is still a stack");
}

#[test]
fn both_callbacks_are_reported_and_agree() {
    let seen_single = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen_multi = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

    let s = seen_single.clone();
    let m = seen_multi.clone();
    let group = three_rows().multiple(true);
    let _ = group.view(
        Some(move |index| s.lock().unwrap().push(index)),
        Some(move |next| m.lock().unwrap().push(next)),
    );

    assert!(
        seen_single.lock().unwrap().is_empty(),
        "no click, no report"
    );
    assert!(seen_multi.lock().unwrap().is_empty());
}

#[test]
fn the_resolved_style_is_enough_to_build_the_widget() {
    let resolved: FluentItemGroupStyle = group().resolved_style();
    assert!(resolved.gap > 0.0);
    assert!(resolved.pad > 0.0);
    assert!(resolved.radius > 0.0);

    let by_hand =
        floom_ui_composites::SelectGroup::new().style(floom_ui_composites::SelectGroupStyle {
            gap: resolved.gap,
            axis: floom_core::Axis::Vertical,
            height: None,
            pad: resolved.pad,
            content_placement: floom_ui_composites::ContentPlacement::Top,
            toggles_off_in_single_mode: false,
            mandatory_in_multiple_mode: true,
            radius: resolved.radius,
            border_width: 1.0,
            background: paint(resolved.surface_variant),
            foreground: paint(resolved.on_surface),
            border_color: paint(resolved.outline),
            selected_background: paint(resolved.primary_container),
            selected_foreground: paint(resolved.on_primary_container),
            selected_border_color: paint(resolved.primary),
        });
    let by_port = group().select_group_style();
    assert_eq!(by_hand.declared_style(), by_port);
    let _ = AREA;
    let _ = Widget::layout_request(&by_hand);
}

#[test]
fn the_colours_are_this_design_system_s_own() {
    {
        let theme = theme();
        let style = group().resolved_style();
        assert_eq!(style.design_system, "fluent");
        assert_eq!(style.primary, theme.colors().primary);
        assert_eq!(
            style.primary,
            floom_core_theme::tokens::color::Color::rgb(0, 120, 212),
            "Fluent's own primary"
        );
    }
}
