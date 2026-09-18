use floem_core_theme::props::button::{ButtonSize, ButtonVariant};
use floem_core_theme::props::divider::DividerOrientation;
use floem_core_theme::theme::{MaterialTheme, Theme};
use floem_elements::prelude::*;

#[test]
fn composed_form_row_text_field_and_checkbox() {
    let theme = MaterialTheme::new();
    let label = Text::new("Name").with_theme(theme.clone());
    let field = TextField::new()
        .placeholder("Enter your name")
        .with_theme(theme.clone());
    let terms = Checkbox::new("Accept terms").with_theme(theme.clone());

    assert_eq!(label.props().content, "Name");
    assert_eq!(field.props().placeholder, "Enter your name");
    assert_eq!(terms.props().label, "Accept terms");
    assert!(!terms.props().checked);

    let text_style = label.resolved_style();
    let field_style = field.resolved_style();
    let checkbox_style = terms.resolved_style();

    assert_eq!(text_style.color, theme.colors().on_surface);
    assert_eq!(field_style.border_color, theme.colors().outline);
    assert_eq!(checkbox_style.outline_color, theme.colors().outline);
}

#[test]
fn dialog_and_button_composition_contract() {
    let theme = MaterialTheme::new();

    let dialog = Dialog::new()
        .title("Delete item")
        .content("This action cannot be undone.")
        .open(true)
        .with_theme(theme.clone());

    let confirm_button = Button::new("Delete")
        .variant(ButtonVariant::Danger)
        .with_theme(theme.clone());
    let cancel_button = Button::new("Cancel")
        .variant(ButtonVariant::Secondary)
        .with_theme(theme.clone());

    assert!(dialog.props().open);
    assert_eq!(dialog.props().title, "Delete item");

    assert_eq!(confirm_button.props().label, "Delete");
    assert_eq!(confirm_button.props().variant, ButtonVariant::Danger);
    assert_eq!(cancel_button.props().label, "Cancel");
    assert_eq!(cancel_button.props().variant, ButtonVariant::Secondary);

    let dialog_style = dialog.resolved_style();
    assert_eq!(dialog_style.container_color, theme.colors().surface);
    assert_eq!(dialog_style.scrim_color, theme.colors().scrim);

    let danger_style = confirm_button.resolved_style();
    assert_eq!(danger_style.background, theme.colors().error);
    assert_eq!(danger_style.foreground, theme.colors().on_error);
}

#[test]
fn menu_with_icons_and_dividers() {
    let theme = MaterialTheme::new();

    let menu = Menu::new()
        .item("Copy", || {})
        .item("Paste", || {})
        .open(false)
        .with_theme(theme.clone());

    let icon = Icon::new("content_copy").size(18.0).label("Copy");
    let divider = Divider::new()
        .orientation(DividerOrientation::Horizontal)
        .inset(8.0)
        .with_theme(theme.clone());

    assert_eq!(menu.props().items.len(), 2);
    assert_eq!(menu.props().items[0].label, "Copy");
    assert_eq!(menu.props().items[1].label, "Paste");
    assert!(!menu.props().open);

    let menu_style = menu.resolved_style();
    assert_eq!(menu_style.container_color, theme.colors().surface);
    assert_eq!(menu_style.text_color, theme.colors().on_surface);
    assert!(menu_style.min_width > 0.0);

    let icon_style = icon.with_theme(theme.clone()).resolved_style();
    assert_eq!(icon_style.size, 18.0);
    assert_eq!(icon_style.color, theme.colors().on_surface_variant);

    let divider_style = divider.resolved_style();
    assert_eq!(divider_style.orientation, DividerOrientation::Horizontal);
    assert_eq!(divider_style.inset, 8.0);
    assert_eq!(divider_style.color, theme.colors().outline_variant);
}

#[test]
fn switch_in_a_card_with_labels() {
    let theme = MaterialTheme::new();

    let card = Card::new().elevation(2).with_theme(theme.clone());
    let switch = Switch::new().checked(true).with_theme(theme.clone());
    let status_text = Text::new("Enabled").with_theme(theme.clone());

    assert_eq!(card.props().elevation, 2);
    let card_style = card.resolved_style();
    assert_eq!(card_style.container_color, theme.colors().surface);
    assert_eq!(card_style.content_color, theme.colors().on_surface);
    assert!(card_style.border_radius > 0.0);

    assert!(switch.props().checked);
    let switch_style = switch.resolved_style();
    assert_eq!(switch_style.track_color, theme.colors().primary);
    assert_eq!(switch_style.thumb_color, theme.colors().on_primary);

    assert_eq!(status_text.resolved_style().color, theme.colors().on_surface);
}

#[test]
fn button_all_variants_and_sizes() {
    let theme = MaterialTheme::new();

    let variants = [
        (ButtonVariant::Primary, theme.colors().primary, theme.colors().on_primary),
        (ButtonVariant::Secondary, theme.colors().secondary_container, theme.colors().on_secondary_container),
        (ButtonVariant::Danger, theme.colors().error, theme.colors().on_error),
    ];

    for &(variant, expected_bg, expected_fg) in &variants {
        let btn = Button::new("test")
            .variant(variant)
            .with_theme(theme.clone());
        let style = btn.resolved_style();
        assert_eq!(style.background, expected_bg, "background for {variant:?}");
        assert_eq!(style.foreground, expected_fg, "foreground for {variant:?}");
    }

    let small = Button::new("s")
        .size(ButtonSize::Small)
        .with_theme(theme.clone())
        .resolved_style();
    let medium = Button::new("m")
        .size(ButtonSize::Medium)
        .with_theme(theme.clone())
        .resolved_style();
    let large = Button::new("l")
        .size(ButtonSize::Large)
        .with_theme(theme.clone())
        .resolved_style();

    assert!(small.height <= 40.0);
    assert_eq!(medium.height, 40.0);
    assert!(large.height >= 48.0);
}

#[test]
fn disabled_widgets_still_resolve_styles() {
    let theme = MaterialTheme::new();

    let btn = Button::new("Save")
        .disabled(true)
        .loading(true)
        .with_theme(theme.clone());
    assert!(btn.props().disabled);
    assert!(btn.props().loading);
    let style = btn.resolved_style();
    assert!(style.height > 0.0);

    let field = TextField::new()
        .disabled(true)
        .with_theme(theme.clone());
    assert!(field.props().disabled);

    let cb = Checkbox::new("T")
        .disabled(true)
        .checked(true)
        .with_theme(theme.clone());
    assert!(cb.props().disabled);
    assert!(cb.props().checked);

    let sw = Switch::new()
        .disabled(true)
        .checked(true)
        .with_theme(theme.clone());
    assert!(sw.props().disabled);
    assert!(sw.props().checked);
}
