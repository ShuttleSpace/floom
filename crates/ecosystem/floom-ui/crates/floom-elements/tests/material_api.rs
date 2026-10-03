use floom_elements::button::{ButtonSize, ButtonVariant};
use floom_elements::prelude::*;

#[test]
fn material_components_expose_builder_state_api() {
    let button = Button::new("Save")
        .variant(ButtonVariant::Secondary)
        .size(ButtonSize::Large)
        .disabled(true)
        .loading(true);
    assert_eq!(button.props().label, "Save");
    assert_eq!(button.props().variant, ButtonVariant::Secondary);
    assert_eq!(button.props().size, ButtonSize::Large);
    assert!(button.is_disabled());
    assert!(button.is_loading());

    let text = Text::new("Body copy").max_lines(2).selectable(true);
    assert_eq!(text.props().content, "Body copy");
    assert_eq!(text.props().max_lines, Some(2));
    assert!(text.props().is_selectable);

    let mut text_field = TextField::new()
        .placeholder("Name")
        .initial_value("Ada")
        .max_length(4);
    text_field.set_value("Lovelace");
    assert_eq!(text_field.value(), "Love");

    let mut checkbox = Checkbox::new("Accept");
    checkbox.toggle();
    assert!(checkbox.is_checked());

    let mut switch = Switch::new();
    switch.toggle();
    assert!(switch.is_checked());

    let dialog = Dialog::new()
        .title("Confirm")
        .content("Continue?")
        .open(true);
    assert!(dialog.is_open());

    let menu = Menu::new().item("Open", || {});
    assert_eq!(menu.props().items.len(), 1);

    let card = Card::new().elevation(3);
    assert_eq!(card.props().elevation, 3);
}
