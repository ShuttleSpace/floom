#![cfg(feature = "list_group")]
//! Fluent list group: the header, the chevron, and the rows it reveals.

use std::sync::{Arc, Mutex};

use floom_core::{Length, Widget};
use floom_types::Insets;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_fluent::widgets::list_group::{FluentListGroup, FluentListGroupStyle};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn tokens() -> floom_core_theme::component_tokens::ListGroupTokens {
    let theme = theme();
    match theme.component(ComponentId::ListGroup) {
        ComponentTokens::ListGroup(t) => t,
        _ => unreachable!(),
    }
}

fn group() -> FluentListGroup {
    FluentListGroup::new()
        .with_theme(theme())
        .title("Section")
        .item("One")
        .item("Two")
}

#[test]
fn the_style_resolves_the_themes_tokens_and_ink() {
    let theme = theme();
    let style: FluentListGroupStyle = group().resolved_style();
    assert_eq!(style.header_height, tokens().header_height);
    assert_eq!(style.content_padding, tokens().content_padding);
    assert_eq!(style.indent, tokens().indent);
    assert_eq!(style.gap, tokens().gap);
    assert_eq!(style.icon_size, tokens().icon_size);
    assert_eq!(style.title_color, theme.colors().on_surface);
    assert_eq!(style.item_color, theme.colors().on_surface);
    assert_eq!(style.indicator_color, theme.colors().on_surface_variant);
    assert_eq!(
        style.item_font_size,
        theme.typography().body.medium.font_size
    );
}

#[test]
fn the_group_is_one_panel_with_the_title() {
    let accordion = group().view();
    assert_eq!(
        accordion.panel_count(),
        1,
        "a list group is one accordion panel"
    );
    assert_eq!(accordion.titles(), vec!["Section"]);
}

#[test]
fn the_group_starts_in_the_props_state() {
    let closed = FluentListGroup::new()
        .with_theme(theme())
        .title("Section")
        .expanded(false)
        .view();
    assert!(!closed.is_open(0), "v1's default is collapsed");

    let open = FluentListGroup::new()
        .with_theme(theme())
        .title("Section")
        .expanded(true)
        .view();
    assert!(open.is_open(0), "and v1 seeded the flag from the props");
}

#[test]
fn the_indicator_is_this_design_systems_glyph() {
    let style = group().resolved_style();
    let (collapsed, expanded) = group().indicator();

    assert!(
        !collapsed.path().segments().is_empty(),
        "the collapsed glyph has a path"
    );
    assert!(
        !expanded.path().segments().is_empty(),
        "and so does the expanded one"
    );
    assert_ne!(
        collapsed.path(),
        expanded.path(),
        "they are two different drawings"
    );
    assert_eq!(collapsed.edge(), style.icon_size, "at the token's size");
    assert_eq!(expanded.edge(), style.icon_size);
    assert_eq!(collapsed.fill().r, style.indicator_color.r);
    assert_eq!(collapsed.fill().g, style.indicator_color.g);
    assert_eq!(collapsed.fill().b, style.indicator_color.b);
}

#[test]
fn the_toggle_callback_survives_being_shared_into_the_tree() {
    let seen: Arc<Mutex<Vec<bool>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let mut accordion = group()
        .on_toggle(move |open| sink.lock().unwrap().push(open))
        .view();

    accordion.set_open(0, true);
    assert!(seen.lock().unwrap().is_empty());
    assert!(
        accordion.is_open(0),
        "but the state did move, so the callback has something to report when a \
         click does happen"
    );
}

#[test]
fn the_props_report_the_group_state() {
    let group = group().expanded(true).inset(true);
    assert!(group.props().expanded);
    assert!(group.props().inset);
    assert_eq!(group.props().title.as_deref(), Some("Section"));
}

fn rows_of(accordion: &floom_ui_composites::Accordion) -> Box<dyn floom_core::WidgetAny> {
    let mut leaf = accordion.children()[0].build();
    let mut create = floom_widgets::testkit::AllocatingCreateContext::new();
    let id = leaf.create(&mut create);
    let mut cx = floom_widgets::testkit::RecordingUpdateContext::new();
    leaf.update(&mut cx, id);
    assert_eq!(leaf.children().len(), 2, "the header and the rows");
    leaf.children().remove(1).build()
}

#[test]
fn the_rows_carry_v1_s_vertical_padding() {
    let style = group().resolved_style();
    let accordion = group().expanded(true).view();

    let rows = rows_of(&accordion);
    let request = rows
        .layout_request()
        .expect("the rows' stack asks for its own space");
    assert_eq!(
        request.padding,
        Some(Insets {
            left: 0.0,
            right: 0.0,
            top: f64::from(style.content_padding),
            bottom: f64::from(style.content_padding),
        }),
        "v1: `padding_vert(content_padding)` on the stack that holds the rows"
    );
}

#[test]
fn each_row_is_v1_s_height_and_padding() {
    let style = group().resolved_style();
    let accordion = group().expanded(true).view();
    let rows = rows_of(&accordion);
    assert_eq!(rows.children().len(), 2, "the two rows the group has");

    let row = rows.children().remove(0).build();
    let request = row.layout_request().expect("a row asks for its own space");
    assert_eq!(
        request.height,
        Some(Length::Px(style.header_height)),
        "v1: the row is as tall as the header"
    );
    assert_eq!(
        request.padding,
        Some(Insets {
            left: f64::from(style.content_padding),
            right: f64::from(style.content_padding),
            top: 0.0,
            bottom: 0.0,
        }),
        "v1: `padding_horiz(pad)` on the row"
    );
}

#[test]
fn an_inset_group_indents_its_rows() {
    let style = group().inset(true).resolved_style();
    let accordion = group().inset(true).expanded(true).view();
    let rows = rows_of(&accordion);
    let row = rows.children().remove(0).build();
    let request = row.layout_request().expect("a row asks for its own space");
    assert_eq!(
        request.padding,
        Some(Insets {
            left: f64::from(style.indent),
            right: f64::from(style.indent),
            top: 0.0,
            bottom: 0.0,
        }),
        "v1: an inset row takes the indent, not the content padding"
    );
    assert_ne!(
        style.indent, style.content_padding,
        "and the two numbers are not the same, so the test can tell them apart"
    );
}

#[test]
fn the_header_is_padded_at_the_sides() {
    let style = group().resolved_style();
    let accordion = group().view();
    let leaf = accordion.children()[0].build();
    let pressable = leaf.children().remove(0).build();
    let row = pressable.children().remove(0).build();
    let request = row
        .layout_request()
        .expect("the header's row asks for its own space");
    assert_eq!(
        request.padding,
        Some(Insets {
            left: f64::from(style.content_padding),
            right: f64::from(style.content_padding),
            top: 0.0,
            bottom: 0.0,
        }),
        "v1: `padding_horiz(content_padding)` on the header"
    );
}

#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    assert_ne!(
        FluentListGroup::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Fluent list group took a snapshot, not the app-wide signal"
    );
}
