use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::button::ButtonVariant;
use floom_core_theme::theme::{MaterialTheme, Theme};
use floom_elements::prelude::*;

#[test]
fn material_components_resolve_styles_from_theme_tokens() {
    let theme = MaterialTheme::new();

    let button = Button::new("Delete").variant(ButtonVariant::Danger);
    let button_style = button.resolved_style();
    assert_eq!(button_style.background, theme.colors().error);
    assert_eq!(button_style.foreground, theme.colors().on_error);
    assert_eq!(button_style.height, 40.0);

    let text = Text::new("Body");
    let text_style = text.resolved_style();
    assert_eq!(text_style.color, theme.colors().on_surface);
    assert_eq!(
        text_style.font_size,
        theme.typography().body.medium.font_size
    );

    let text_field = TextField::new();
    let unfocused_style = text_field.resolved_style();
    assert_eq!(unfocused_style.border_color, theme.colors().outline);
    text_field.set_focused(true);
    let focused_style = text_field.resolved_style();
    assert_eq!(focused_style.border_color, theme.colors().primary);

    let checkbox_style = Checkbox::new("Accept").checked(true).resolved_style();
    assert_eq!(checkbox_style.mark_color, theme.colors().on_primary);
    assert_eq!(checkbox_style.container_color, theme.colors().primary);

    let switch_style = Switch::new().checked(true).resolved_style();
    assert_eq!(switch_style.track_color, theme.colors().primary);
    assert_eq!(switch_style.thumb_offset, 20.0);

    let dialog_style = Dialog::new().open(true).resolved_style();
    assert_eq!(dialog_style.scrim_color, theme.colors().scrim);
    assert_eq!(dialog_style.max_width, 560.0);

    let menu_style = Menu::new().open(true).resolved_style();
    assert_eq!(menu_style.container_color, theme.colors().surface);
    assert_eq!(menu_style.item_height, 48.0);

    let card_style = Card::new().elevation(3).resolved_style();
    let expected_shadow = theme.shadow().levels[3];
    assert_eq!(card_style.shadow, expected_shadow);

    let ComponentTokens::Button(button_tokens) = theme.component(ComponentId::Button) else {
        panic!("button tokens expected")
    };
    assert_eq!(
        button_style.padding_horizontal,
        button_tokens.padding_horizontal
    );
}
