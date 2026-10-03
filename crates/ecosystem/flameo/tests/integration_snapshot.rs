use floem_core_theme::props::button::ButtonVariant;
use floem_core_theme::theme::{MaterialTheme, Theme};
use floem_elements::prelude::*;
use floem_testing::component_snapshot;

#[test]
fn snapshot_button_primary_default() {
    let theme = MaterialTheme::new();
    let button = Button::new("Submit").with_theme(theme.clone());
    let props = button.props();
    let style = button.resolved_style();

    let snap = component_snapshot(
        "Button/Primary/Default",
        format!(
            "Button {{ label: {:?}, variant: {:?}, size: {:?}, disabled: {}, loading: {} }}",
            props.label, props.variant, props.size, props.disabled, props.loading
        ),
    );
    snap.assert_matches(&snap.content);
    assert_eq!(style.background, theme.colors().primary);
}

#[test]
fn snapshot_text_field_with_values() {
    let theme = MaterialTheme::new();
    let field = TextField::new()
        .placeholder("Email")
        .initial_value("user@example.com")
        .with_theme(theme.clone());
    let props = field.props();

    let snap = component_snapshot(
        "TextField/WithValue",
        format!(
            "TextField {{ placeholder: {:?}, value: {:?}, password: {}, max_length: {:?}, disabled: {} }}",
            props.placeholder, props.value, props.is_password, props.max_length, props.disabled
        ),
    );
    snap.assert_matches(&snap.content);
}

#[test]
fn snapshot_dialog_open_state() {
    let theme = MaterialTheme::new();
    let dialog = Dialog::new()
        .title("Delete")
        .content("Are you sure?")
        .open(true)
        .with_theme(theme.clone());
    let props = dialog.props();

    let snap = component_snapshot(
        "Dialog/Open",
        format!(
            "Dialog {{ title: {:?}, content: {:?}, open: {} }}",
            props.title, props.content, props.open
        ),
    );
    snap.assert_matches(&snap.content);
}

#[test]
fn snapshot_checkbox_checked_and_disabled() {
    let theme = MaterialTheme::new();
    let cb = Checkbox::new("Agree")
        .checked(true)
        .disabled(true)
        .with_theme(theme.clone());
    let props = cb.props();

    let snap = component_snapshot(
        "Checkbox/CheckedDisabled",
        format!(
            "Checkbox {{ label: {:?}, checked: {}, disabled: {} }}",
            props.label, props.checked, props.disabled
        ),
    );
    snap.assert_matches(&snap.content);
}

#[test]
fn snapshot_switch_checked_and_disabled() {
    let theme = MaterialTheme::new();
    let sw = Switch::new()
        .checked(true)
        .disabled(true)
        .with_theme(theme.clone());
    let props = sw.props();

    let snap = component_snapshot(
        "Switch/CheckedDisabled",
        format!(
            "Switch {{ checked: {}, disabled: {} }}",
            props.checked, props.disabled
        ),
    );
    snap.assert_matches(&snap.content);
}

#[test]
fn snapshot_menu_with_items() {
    let theme = MaterialTheme::new();
    let menu = Menu::new()
        .item("Copy", || {})
        .item("Cut", || {})
        .item("Paste", || {})
        .open(false)
        .with_theme(theme.clone());
    let props = menu.props();

    let mut desc = String::from("Menu { items: [");
    for (i, item) in props.items.iter().enumerate() {
        if i > 0 {
            desc.push_str(", ");
        }
        desc.push_str(&format!("\"{}\"", item.label));
    }
    desc.push_str(&format!("], open: {} }}", props.open));

    let snap = component_snapshot("Menu/ThreeItems", desc);
    snap.assert_matches(&snap.content);
}

#[test]
fn snapshot_card_with_elevation() {
    let theme = MaterialTheme::new();
    let card = Card::new().elevation(3).with_theme(theme.clone());
    let props = card.props();

    let snap = component_snapshot(
        "Card/Elevation3",
        format!("Card {{ elevation: {} }}", props.elevation),
    );
    snap.assert_matches(&snap.content);
}

#[test]
fn snapshot_form_layout_composition() {
    let theme = MaterialTheme::new();
    let name = TextField::new()
        .placeholder("Name")
        .with_theme(theme.clone());
    let email = TextField::new()
        .placeholder("Email")
        .with_theme(theme.clone());
    let terms = Checkbox::new("I agree")
        .checked(true)
        .with_theme(theme.clone());
    let submit = Button::new("Submit")
        .variant(ButtonVariant::Primary)
        .with_theme(theme.clone());

    let composed = format!(
        "Form {{ name: {:?}, email: {:?}, terms_checked: {}, submit: {:?} }}",
        name.props().placeholder,
        email.props().placeholder,
        terms.props().checked,
        submit.props().label,
    );

    let snap = component_snapshot("Composition/Form", composed);
    snap.assert_matches(&snap.content);
}
