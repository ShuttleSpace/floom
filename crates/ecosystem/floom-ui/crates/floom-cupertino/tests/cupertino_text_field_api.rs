#![cfg(feature = "text_field")]
//! The contract every Cupertino text field offers, rewritten for the 2.0 API.
//!

use floom_core_theme::global_theme::{set_design_system, DesignSystem};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_cupertino::widgets::text_field::{CupertinoTextField, DESIGN_SYSTEM};

/// The props cross into the style-independent half of the contract: the value and
/// the placeholder, the cap, the flag, and the builder-only prop that renders
/// nothing.
#[test]
fn text_field_exposes_neutral_contract_api() {
    let field = CupertinoTextField::new()
        .placeholder("Name")
        .value("Ada")
        .password(true)
        .max_length(8)
        .disabled(true);

    assert_eq!(field.props().placeholder, "Name");
    assert_eq!(field.props().value, "Ada");
    assert!(
        field.props().is_password,
        "the prop is carried; whether anything renders a mask is a different \
         question, and this crate's own test says it is not answered"
    );
    assert_eq!(field.props().max_length, Some(8));
    assert!(field.props().disabled);

    // v1 spelled this on a style struct field. The engine's `TextFieldStyle`
    // names no design system, so the string is a constant on the crate now --
    // the same value, somewhere a caller can still read it.
    assert_eq!(DESIGN_SYSTEM, "cupertino");

    // `theme()` is what a host would reach for to re-theme one field. Asserted
    // only for being callable: `MultiTheme` is not `PartialEq`, and the numbers
    // that matter are asserted through `resolved_style` below.
    let _ = CupertinoTextField::new().theme();
}

/// The four style values v1's `resolved_style()` claimed and every `view()`
/// painted. The literals are Cupertino's and are kept exactly.
#[test]
fn text_field_style_resolves_from_the_cupertino_theme() {
    set_design_system(DesignSystem::Cupertino);
    let theme = MultiTheme::new_for(DesignSystem::Cupertino);
    let style = CupertinoTextField::new()
        .with_theme(theme.clone())
        .resolved_style();

    let colors = theme.colors();
    assert_eq!(
        style.border_color.r, colors.outline.r,
        "the resting rim is `outline`"
    );
    assert_eq!(style.border_color.g, colors.outline.g);
    assert_eq!(style.border_color.b, colors.outline.b);
    assert_eq!(
        style.text_color.r, colors.on_surface.r,
        "the ink is `on_surface`"
    );
    assert_eq!(style.text_color.g, colors.on_surface.g);
    assert_eq!(style.text_color.b, colors.on_surface.b);

    // v1's struct spelled these `f32`; the engine's spells the height
    // `Option<f32>` because a kit that pins none derives it from the font
    // metrics. All three kits pin one, so the `Some` is the port's, not v1's.
    assert_eq!(
        style.height,
        Some(44.0),
        "v1 read `TextFieldTokens::height`, which is Cupertino's own 44.0"
    );
    assert_eq!(style.border_radius, 10.0, "v1 read the same token");

    // And the height is Cupertino's, not another kit's. Without the pin above a Cupertino
    // field would answer 48.0 and this file would prove nothing.
    assert_ne!(
        style.height,
        Some(32.0),
        "control: 32.0 is Fluent's token, and a Cupertino field must not report it"
    );
    assert_ne!(style.border_radius, 4.0, "control: 4.0 is Fluent's radius");
}
