use floem_core_theme::theme::{MaterialTheme, Theme};
use floem_elements::prelude::*;

#[test]
fn material_components_accept_explicit_theme_instances() {
    let theme = MaterialTheme::new();

    let button_style = Button::new("Save")
        .with_theme(theme.clone())
        .resolved_style();
    assert_eq!(button_style.background, theme.colors().primary);

    let text_style = Text::new("Body").with_theme(theme.clone()).resolved_style();
    assert_eq!(text_style.color, theme.colors().on_surface);

    let field_style = TextField::new().with_theme(theme.clone()).resolved_style();
    assert_eq!(field_style.height, 48.0);

    let checkbox_style = Checkbox::new("Accept")
        .with_theme(theme.clone())
        .resolved_style();
    assert_eq!(checkbox_style.size, 18.0);

    let switch_style = Switch::new().with_theme(theme.clone()).resolved_style();
    assert_eq!(switch_style.width, 52.0);

    let dialog_style = Dialog::new().with_theme(theme.clone()).resolved_style();
    assert_eq!(dialog_style.border_radius, 28.0);

    let menu_style = Menu::new().with_theme(theme.clone()).resolved_style();
    assert_eq!(menu_style.min_width, 112.0);

    let card_style = Card::new().with_theme(theme.clone()).resolved_style();
    assert_eq!(card_style.padding, 16.0);
}
