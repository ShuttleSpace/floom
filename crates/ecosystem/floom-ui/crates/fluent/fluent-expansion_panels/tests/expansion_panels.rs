//!

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{Axis, Widget};
use floom_types::Insets;
use floom_widgets::VectorIcon;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, ExpansionPanelsTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::expansion_panels::ExpansionPanelsProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_widget_fluent_expansion_panels::{FluentExpansionPanels, FluentExpansionPanelsStyle};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn kit() -> FluentExpansionPanels {
    let _ = theme();
    FluentExpansionPanels::new()
}

fn tokens() -> ExpansionPanelsTokens {
    let theme = theme();
    let ComponentTokens::ExpansionPanels(tokens) = theme.component(ComponentId::ExpansionPanels)
    else {
        unreachable!("ComponentId::ExpansionPanels must resolve to ExpansionPanelsTokens")
    };
    tokens
}

fn panels(count: usize) -> FluentExpansionPanels {
    let mut built = kit();
    for index in 0..count {
        built = built.panel(format!("Panel {index}"), || {
            Box::new(floom_widgets::Text::new("body"))
        });
    }
    built
}


#[test]
fn the_resolved_style_is_the_theme_tokens() {
    let tokens = tokens();
    let style: FluentExpansionPanelsStyle = kit().resolved_style();
    assert_eq!(style.header_height, tokens.header_height);
    assert_eq!(style.header_padding, tokens.content_padding);
    assert_eq!(style.content_padding, tokens.content_padding);
    assert_eq!(style.content_indent, tokens.indent);
    assert_eq!(style.gap, tokens.gap);
    assert_eq!(style.radius, tokens.radius);
    assert_eq!(style.border_width, tokens.border);
}

#[test]
fn the_outline_is_the_theme_outline_at_30_alpha() {
    let style = kit().resolved_style();
    assert_eq!(style.border_color.a, 30, "v1's `with_alpha(outline, 30)`");

    let theme = MultiTheme::new_for(DesignSystem::Fluent);
    assert_eq!(
        [
            style.border_color.r,
            style.border_color.g,
            style.border_color.b
        ],
        [
            theme.colors().outline.r,
            theme.colors().outline.g,
            theme.colors().outline.b
        ],
        "and the outline's own colour, not a constant"
    );
}

#[test]
fn the_engine_widget_declares_the_token_axis_and_gap() {
    let req = panels(2)
        .resolve()
        .layout_request()
        .expect("the panels are one vertical stack");
    assert_eq!(req.axis, Some(Axis::Vertical));
    assert_eq!(req.gap, Some(tokens().gap), "the token gap, not a default");
}


#[test]
fn every_declared_panel_becomes_one_slot() {
    let accordion = panels(3).resolve();
    assert_eq!(accordion.panel_count(), 3);
    assert_eq!(accordion.titles(), vec!["Panel 0", "Panel 1", "Panel 2"]);
    assert_eq!(accordion.children().len(), 3);
    assert_eq!(kit().panel_count(), 0);
}

#[test]
fn the_single_panel_rule_survives_the_trip() {
    let mut accordion = panels(2).resolve();
    accordion.set_open(0, true);
    accordion.set_open(1, true);
    assert_eq!(
        accordion.open_state(),
        vec![false, true],
        "Material's expansion panels are an accordion by default"
    );

    let mut accordion = panels(2).multiple(true).resolve();
    accordion.set_open(0, true);
    accordion.set_open(1, true);
    assert_eq!(
        accordion.open_state(),
        vec![true, true],
        "unless the caller opts out"
    );
}


#[test]
fn a_body_is_a_send_sync_builder() {
    let builds = Arc::new(AtomicUsize::new(0));
    let sink = builds.clone();
    let component = kit().panel("Only", move || {
        sink.fetch_add(1, Ordering::SeqCst);
        Box::new(floom_widgets::Text::new("body"))
    });

    assert_eq!(component.panel_count(), 1);
    assert_eq!(
        builds.load(Ordering::SeqCst),
        0,
        "declaring a panel must not build its body — a body is built when the \
         panel is opened, and again whenever the tree rebuilds that slot"
    );

    let accordion = component.resolve();
    let _ = accordion.children();
    assert_eq!(
        builds.load(Ordering::SeqCst),
        0,
        "even declaring the panel's slot builds nothing"
    );
}

#[test]
fn the_body_inset_carries_both_tokens() {
    let tokens = tokens();
    let margin = kit().content_layout().margin.expect("the body's inset");
    assert_eq!(
        margin.left,
        f64::from(tokens.content_padding + tokens.indent),
        "horizontal: padding plus indent, as v1 wrote it"
    );
    assert_eq!(
        margin.right,
        f64::from(tokens.content_padding + tokens.indent)
    );
}


#[test]
fn the_indicator_is_a_pair_of_distinct_glyphs() {
    let indicator = kit().indicator();
    assert_ne!(
        indicator.collapsed.path(),
        indicator.expanded.path(),
        "a collapsed panel's chevron and an expanded one's are different shapes"
    );
}

#[test]
fn the_indicator_carries_the_token_size_and_the_muted_ink() {
    let indicator = kit().indicator();
    for icon in [&indicator.collapsed, &indicator.expanded] {
        assert_eq!(icon.edge(), tokens().icon_size);
        let req = icon
            .layout_request()
            .expect("an icon asks for its own size");
        assert_eq!(
            req.width.and_then(|l| l.px()),
            Some(tokens().icon_size),
            "the icon is sized in pixels, not as a percentage"
        );
        assert_eq!(
            req.height.and_then(|l| l.px()),
            Some(tokens().icon_size),
            "the icon is sized in pixels, not as a percentage"
        );
    }

    let theme = MultiTheme::new_for(DesignSystem::Fluent);
    assert_eq!(
        indicator.expanded.fill().r,
        theme.colors().on_surface_variant.r,
        "v1 coloured the chevron `on_surface_variant`"
    );
    assert_eq!(
        indicator.expanded.fill(),
        indicator.collapsed.fill(),
        "and both states share the ink"
    );
}

#[test]
fn the_kit_hands_the_engine_a_builder_that_is_told_the_state() {
    let indicator = kit().indicator();
    let (collapsed, expanded) = (indicator.collapsed, indicator.expanded);
    let choose = move |open: bool| -> Box<dyn floom_core::WidgetAny> {
        Box::new(if open {
            expanded.clone()
        } else {
            collapsed.clone()
        })
    };
    assert!(choose(false).as_any().is::<VectorIcon>());
    assert!(choose(true).as_any().is::<VectorIcon>());
    assert_ne!(
        choose(true)
            .as_any()
            .downcast_ref::<VectorIcon>()
            .expect("an icon")
            .path(),
        choose(false)
            .as_any()
            .downcast_ref::<VectorIcon>()
            .expect("an icon")
            .path(),
        "and the two states resolve to different glyphs"
    );
}


#[test]
fn the_shared_props_report_the_multiple_rule_only() {
    let props: ExpansionPanelsProps = panels(1).multiple(true).props();
    assert!(props.multiple);
    assert!(
        props.on_change.is_none(),
        "the callback is held as an `Arc` of the kit's own, not in the v1 carrier"
    );
}

#[test]
fn the_change_callback_survives_being_shared_into_the_tree() {
    let seen: Arc<std::sync::Mutex<Vec<(usize, bool)>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = seen.clone();
    let mut accordion = panels(2)
        .on_change(move |index, open| sink.lock().unwrap().push((index, open)))
        .resolve();

    accordion.set_open(1, true);
    assert!(seen.lock().unwrap().is_empty());
    assert!(
        accordion.is_open(1),
        "but the state did move, so the callback has something to report when a \
         click does happen"
    );
}

#[test]
#[test]
fn the_colours_are_this_design_system_s_own() {
    let theme = theme();
    let style = panels(1).resolved_style();
    assert_eq!(
        style.title_color.r,
        theme.colors().on_surface.r,
        "the title is drawn in this design system's own ink"
    );
    assert_eq!(
        style.border_color.r,
        theme.colors().outline.r,
        "and the separator in its own outline"
    );
    assert_eq!(
        theme.colors().primary,
        Color::rgb(0, 120, 212),
        "Fluent's own primary"
    );
}

#[test]
fn the_header_is_padded_at_the_sides() {
    let style = FluentExpansionPanels::new().resolved_style();
    let accordion = panels(1).resolve();
    let leaf = accordion.children()[0].build();
    let pressable = leaf.children().remove(0).build();
    let row = pressable.children().remove(0).build();
    let request = row
        .layout_request()
        .expect("the header's row asks for its own space");
    assert_eq!(
        request.padding,
        Some(Insets {
            left: f64::from(style.header_padding),
            right: f64::from(style.header_padding),
            top: 0.0,
            bottom: 0.0,
        }),
        "v1: `padding_horiz(content_padding)` on the header"
    );
}
