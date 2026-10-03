use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::theme::{CupertinoTheme, FluentTheme, Theme};
use floom_core_theme::tokens::color::Color;

#[test]
fn cupertino_theme_implements_theme_contract() {
    let theme = CupertinoTheme::new();

    assert_eq!(theme.colors().primary, Color::rgb(0, 122, 255));
    assert_eq!(theme.colors().surface, Color::rgb(255, 255, 255));
    assert_eq!(theme.typography().body.medium.font_size, 17.0);
    assert_eq!(theme.radius().md, 10.0);

    let ComponentTokens::Button(button) = theme.component(ComponentId::Button) else {
        panic!("button tokens expected")
    };
    assert_eq!(button.height, 44.0);
}

#[test]
fn fluent_theme_implements_theme_contract() {
    let theme = FluentTheme::new();

    assert_eq!(theme.colors().primary, Color::rgb(0, 120, 212));
    assert_eq!(theme.colors().surface, Color::rgb(255, 255, 255));
    assert_eq!(theme.typography().body.medium.font_size, 14.0);
    assert_eq!(theme.radius().md, 4.0);

    let ComponentTokens::Button(button) = theme.component(ComponentId::Button) else {
        panic!("button tokens expected")
    };
    assert_eq!(button.height, 32.0);
}
