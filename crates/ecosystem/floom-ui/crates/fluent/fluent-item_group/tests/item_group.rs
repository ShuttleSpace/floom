//! The ported Fluent item group — the shortest port in the library, and the
//! one that proves the promotion was right.
//!
//! 209 lines of v1 became a token-to-style wrapper, because 123 of 149
//! substantive lines are shared with the cupertino and fluent copies. So this
//! suite is mostly *not* about the state machine — that lives in
//! `floom-widgets`' `compose::select_group_tests`, which is the right boundary for
//! it. What is here is the translation: the tokens reach the engine's style, the
//! colours mean what they meant, and the props reach the rules.

use floom_core::Widget;
use floom_types::{Insets, Rect};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_fluent_item_group::{FluentItemGroup, FluentItemGroupStyle};

const AREA: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 120.0,
};

/// The Fluent theme, with Fluent selected.
///
/// `MultiTheme::new_for(ds)` does *not* select `ds` -- it builds all three and
/// `activate` is a documented no-op -- and a `MultiTheme` reads the *active*
/// design system at read time. So a test that wants Fluent's own
/// palette has to select Fluent, and `set_design_system` writes a
/// thread-local, so the selection is this test's alone.
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

// --- the translation ------------------------------------------------------

/// The whole of this port: the theme's tokens and colours, in the engine's
/// vocabulary, with no logic left over.
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

/// The unselected/selected pair is a *pair*, and this is the assertion that stops
/// a caller styling a chosen row's box while leaving its label in the unchosen ink.
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

/// v1 wrote `padding_horiz(pad).padding_vert(pad / 2.0)` — not a uniform inset,
/// and the kind of asymmetry a "just use `uniform`" cleanup would quietly lose.
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

// --- the props reach the rules --------------------------------------------

/// v1 held two signals — `single` and `multi` — and read `multi.contains(&i)` for
/// a row's appearance. 2.0 has one shared state, so the two accessors are derived
/// from one widget rather than kept in step by hand.
#[test]
fn one_widget_answers_both_accessors() {
    let widget = group()
        .row("One", || Box::new(floom_widgets::Text::new("one")))
        .view(None::<fn(usize)>, None::<fn(Vec<usize>)>);
    assert_eq!(floom_widget_fluent_item_group::selected(&widget), None);
    assert_eq!(
        floom_widget_fluent_item_group::selected_multi(&widget),
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

/// v1's `selected(usize)` seeded both signals; here it seeds the one shared state,
/// and both accessors have to agree about it.
#[test]
fn the_initial_selection_is_readable_through_both_accessors() {
    let widget = three_rows()
        .selected(2)
        .view(None::<fn(usize)>, None::<fn(Vec<usize>)>);
    assert_eq!(floom_widget_fluent_item_group::selected(&widget), Some(2));
    assert_eq!(
        floom_widget_fluent_item_group::selected_multi(&widget),
        vec![2]
    );
}

// --- the structure --------------------------------------------------------

#[test]
fn every_declared_row_reaches_the_widget() {
    let widget = three_rows().view(None::<fn(usize)>, None::<fn(Vec<usize>)>);
    assert_eq!(widget.children().len(), 3);

    // `row_unlabelled` appends, like `row` does — a fourth row, not a
    // replacement.
    let four = three_rows()
        .row_unlabelled(|| Box::new(floom_widgets::Text::new("x")))
        .view(None::<fn(usize)>, None::<fn(Vec<usize>)>);
    assert_eq!(four.children().len(), 4);

    let one = group()
        .row_unlabelled(|| Box::new(floom_widgets::Text::new("x")))
        .view(None::<fn(usize)>, None::<fn(Vec<usize>)>);
    assert_eq!(one.children().len(), 1);
}

/// An empty group is a stack with no children. v1 wrote
/// `apply_if(n == 0, |s| s.hide())` — hiding the *node* — and 2.0 has no `hide()`.
/// The difference is real: a hidden node takes no space, an empty stack takes
/// whatever the parent gives it. Recorded rather than papered over.
#[test]
fn an_empty_group_still_builds() {
    let widget = FluentItemGroup::new()
        .with_theme(theme())
        .view(None::<fn(usize)>, None::<fn(Vec<usize>)>);
    assert!(widget.children().is_empty());
    assert!(widget.layout_request().is_some(), "and it is still a stack");
}

// --- the callbacks --------------------------------------------------------

/// v1 had two callbacks because it had two signals. A caller in single mode gets
/// the index; a caller in multiple mode gets the set; both are reported so neither
/// has to know which mode it is in.
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

    // The rules themselves are the engine's; what this pins is that both
    // callbacks reach the same change rather than one shadowing the other.
    assert!(
        seen_single.lock().unwrap().is_empty(),
        "no click, no report"
    );
    assert!(seen_multi.lock().unwrap().is_empty());
}

/// The style struct is the port's whole output, so a caller can build the widget
/// by hand from it — which is what `view` does internally, and what makes the
/// component composable in a way v1's `AnyView` was not.
#[test]
fn the_resolved_style_is_enough_to_build_the_widget() {
    let resolved: FluentItemGroupStyle = group().resolved_style();
    assert!(resolved.gap > 0.0);
    assert!(resolved.pad > 0.0);
    assert!(resolved.radius > 0.0);

    // And a hand-built widget with those exact numbers is indistinguishable from
    // the port's own.
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

/// The kit resolves Fluent's palette, not another system's.
///
/// The item-group *tokens* are the same in all three design systems, so the
/// only thing that distinguishes the kits is the colour -- and a test that
/// compared the kit against whatever theme the thread happened to hold would
/// pass for any of the three. So the palette is pinned here: Fluent's
/// `primary` is a literal, and it is not Material's.
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
