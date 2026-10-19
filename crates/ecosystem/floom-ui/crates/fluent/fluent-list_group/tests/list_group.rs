//! Fluent list group: the header, the chevron, and the rows it reveals.
//!
//! ## The claim
//!
//! A list group is a titled, collapsible group of rows. v1 built it as a
//! clickable header over a stack of rows that appeared and disappeared with a
//! signal; 2.0's `Accordion` owns that state, and one list group is one
//! accordion with a single panel. The assertions are about the two things the
//! kit decides -- the header's style and the rows -- and about the chevron,
//! which is this design system's own glyph.

use std::sync::{Arc, Mutex};

use floom_core::Widget;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_list_group::{
    FluentListGroup, FluentListGroupStyle,
};

/// The Fluent theme, with Fluent selected.
///
/// `MultiTheme::new_for(ds)` does *not* select `ds` -- it builds all three and
/// `activate` is a documented no-op -- and a `MultiTheme` reads the *active*
/// design system at read time. So a test that wants Fluent's own values has to
/// select Fluent, and `set_design_system` writes a thread-local, so the
/// selection is this test's alone.
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

/// The style resolves the theme's own tokens and ink.
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
    assert_eq!(style.item_font_size, theme.typography().body.medium.font_size);
}

/// The group is one panel, with the group's title on it.
#[test]
fn the_group_is_one_panel_with_the_title() {
    let accordion = group().view();
    assert_eq!(accordion.panel_count(), 1, "a list group is one accordion panel");
    assert_eq!(accordion.titles(), vec!["Section"]);
}

/// The group's initial state is the props' `expanded`.
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

/// The chevron is this design system's own glyph: the collapsed and the
/// expanded one are different drawings, and both are drawn at the token's size
/// in the muted ink.
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

/// The toggle callback has to be `Send + Sync` to reach a click that lands
/// inside a `Child` builder closure. Holding it as an `Arc` and cloning it into
/// the accordion is what makes this compile, so the test that exists is the one
/// that runs that construction.
#[test]
fn the_toggle_callback_survives_being_shared_into_the_tree() {
    let seen: Arc<Mutex<Vec<bool>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let mut accordion = group()
        .on_toggle(move |open| sink.lock().unwrap().push(open))
        .view();

    // The imperative door does not report a person's click -- the callback is
    // for the user, and a caller that just set the state knows already.
    accordion.set_open(0, true);
    assert!(seen.lock().unwrap().is_empty());
    assert!(
        accordion.is_open(0),
        "but the state did move, so the callback has something to report when a \
         click does happen"
    );
}

/// The rows are carried by the group, and the props keep reporting what v1
/// reported.
#[test]
fn the_props_report_the_group_state() {
    let group = group().expanded(true).inset(true);
    assert!(group.props().expanded);
    assert!(group.props().inset);
    assert_eq!(group.props().title.as_deref(), Some("Section"));
}
