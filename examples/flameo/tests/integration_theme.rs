use floom_core_theme::component_tokens::ComponentId;
use floom_core_theme::theme::{MaterialTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_elements::prelude::*;

#[test]
fn material_theme_creation_and_color_tokens() {
    let theme = MaterialTheme::new();

    assert_eq!(theme.colors().primary, Color::rgb(103, 80, 164));
    assert_eq!(theme.colors().on_primary, Color::rgb(255, 255, 255));
    assert_eq!(theme.colors().surface, Color::rgb(254, 247, 255));
    assert_eq!(theme.colors().on_surface, Color::rgb(29, 27, 32));
    assert_eq!(theme.colors().error, Color::rgb(179, 38, 30));
    assert_eq!(theme.colors().outline, Color::rgb(121, 116, 126));
    assert_eq!(theme.colors().outline_variant, Color::rgb(202, 196, 208));
    assert_eq!(theme.colors().scrim, Color::rgba(0, 0, 0, 128));

    assert_eq!(theme.spacing().xs, 4.0);
    assert_eq!(theme.spacing().sm, 8.0);
    assert_eq!(theme.spacing().md, 16.0);
    assert_eq!(theme.spacing().lg, 24.0);
    assert_eq!(theme.spacing().xl, 32.0);
    assert_eq!(theme.spacing().xxl, 48.0);

    let body_medium = &theme.typography().body.medium;
    assert_eq!(body_medium.font_size, 14.0);
    assert_eq!(body_medium.line_height, 20.0);

    assert_eq!(theme.radius().none, 0.0);
    assert_eq!(theme.radius().xs, 4.0);
    assert_eq!(theme.radius().sm, 8.0);
    assert_eq!(theme.radius().md, 12.0);
    assert_eq!(theme.radius().full, 9999.0);
}

#[test]
fn all_widgets_resolve_style_from_same_theme() {
    let theme = MaterialTheme::new();

    let button_style = Button::new("OK").with_theme(theme.clone()).resolved_style();
    let text_style = Text::new("Hello").with_theme(theme.clone()).resolved_style();
    let field_style = TextField::new().with_theme(theme.clone()).resolved_style();
    let _checkbox_style = Checkbox::new("X").with_theme(theme.clone()).resolved_style();
    let _switch_style = Switch::new().with_theme(theme.clone()).resolved_style();
    let dialog_style = Dialog::new().with_theme(theme.clone()).resolved_style();
    let menu_style = Menu::new().with_theme(theme.clone()).resolved_style();
    let card_style = Card::new().with_theme(theme.clone()).resolved_style();
    let icon_style = Icon::new("home").with_theme(theme.clone()).resolved_style();
    let divider_style = Divider::new().with_theme(theme.clone()).resolved_style();

    assert_eq!(button_style.background, theme.colors().primary);
    assert_eq!(card_style.container_color, theme.colors().surface);
    assert_eq!(dialog_style.container_color, theme.colors().surface);
    assert_eq!(menu_style.container_color, theme.colors().surface);

    assert_eq!(text_style.color, theme.colors().on_surface);
    assert_eq!(card_style.content_color, theme.colors().on_surface);
    assert_eq!(dialog_style.content_color, theme.colors().on_surface);
    assert_eq!(menu_style.text_color, theme.colors().on_surface);
    assert_eq!(field_style.text_color, theme.colors().on_surface);

    assert!(button_style.height > 0.0);
    assert!(field_style.height > 0.0);
    assert!(card_style.border_radius > 0.0);
    assert!(icon_style.size > 0.0);
    assert!(divider_style.thickness > 0.0);
}

#[test]
fn component_token_lookup_for_all_component_ids() {
    let theme = MaterialTheme::new();

    use floom_core_theme::component_tokens::ComponentTokens;

    assert!(matches!(theme.component(ComponentId::Button), ComponentTokens::Button(_)));
    assert!(matches!(theme.component(ComponentId::Text), ComponentTokens::Text(_)));
    assert!(matches!(theme.component(ComponentId::TextField), ComponentTokens::TextField(_)));
    assert!(matches!(theme.component(ComponentId::Checkbox), ComponentTokens::Checkbox(_)));
    assert!(matches!(theme.component(ComponentId::Switch), ComponentTokens::Switch(_)));
    assert!(matches!(theme.component(ComponentId::Dialog), ComponentTokens::Dialog(_)));
    assert!(matches!(theme.component(ComponentId::Menu), ComponentTokens::Menu(_)));
    assert!(matches!(theme.component(ComponentId::Card), ComponentTokens::Card(_)));
    assert!(matches!(theme.component(ComponentId::Icon), ComponentTokens::Icon(_)));
    assert!(matches!(theme.component(ComponentId::Divider), ComponentTokens::Divider(_)));
}

#[test]
fn theme_satisfies_trait_bounds() {
    fn assert_theme<T: Theme>(_theme: &T) {}

    let material = MaterialTheme::new();
    assert_theme(&material);

    let cloned = material.clone();
    assert_eq!(cloned.colors().primary, material.colors().primary);
    assert_eq!(
        cloned.typography().body.medium.font_size,
        material.typography().body.medium.font_size
    );
}

#[test]
fn button_size_tokens_from_material_theme() {
    let theme = MaterialTheme::new();

    let tok = theme.component(ComponentId::Button);
    let floom_core_theme::component_tokens::ComponentTokens::Button(button_tokens) = tok else {
        panic!("expected Button tokens");
    };
    assert_eq!(button_tokens.height, 40.0);

    let small_btn = Button::new("S")
        .size(ButtonSize::Small)
        .with_theme(theme.clone());
    assert!(small_btn.resolved_style().height <= 40.0);

    let medium_btn = Button::new("M")
        .size(ButtonSize::Medium)
        .with_theme(theme.clone());
    assert_eq!(medium_btn.resolved_style().height, 40.0);

    let large_btn = Button::new("L")
        .size(ButtonSize::Large)
        .with_theme(theme);
    assert!(large_btn.resolved_style().height >= 48.0);
}
