//! The contract every Fluent text field offers, rewritten for the 2.0 API.

use floom_core_theme::global_theme::{set_design_system, DesignSystem};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_fluent_text_field::{FluentTextField, DESIGN_SYSTEM};

#[test]
fn text_field_exposes_neutral_contract_api() {
    let field = FluentTextField::new()
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

    assert_eq!(DESIGN_SYSTEM, "fluent");

    let _ = FluentTextField::new().theme();
}

#[test]
fn text_field_style_resolves_from_the_fluent_theme() {
    set_design_system(DesignSystem::Fluent);
    let theme = MultiTheme::new_for(DesignSystem::Fluent);
    let style = FluentTextField::new()
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

    assert_eq!(
        style.height,
        Some(32.0),
        "v1 read `TextFieldTokens::height`, which is Fluent's own 32.0"
    );
    assert_eq!(style.border_radius, 4.0, "v1 read the same token");

    assert_ne!(
        style.height,
        Some(44.0),
        "control: 44.0 is Cupertino's token, and a Fluent field must not report it"
    );
    assert_ne!(
        style.border_radius, 10.0,
        "control: 10.0 is Cupertino's radius"
    );
}
