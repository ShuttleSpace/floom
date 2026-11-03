use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
use floom_core_theme::theme::{CupertinoTheme, FluentTheme, MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_signal::SignalUpdate;

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



/// The palette follows the global, and ignores the design system the theme was
/// built for.
#[test]
fn multi_theme_reads_follow_the_process_global_not_its_own_argument() {
    design_system_signal().set(DesignSystem::Material);
    let built_as_cupertino = MultiTheme::new_for(DesignSystem::Cupertino);

    design_system_signal().set(DesignSystem::Material);
    let material = built_as_cupertino.colors().surface_variant;

    design_system_signal().set(DesignSystem::Cupertino);
    let cupertino = built_as_cupertino.colors().surface_variant;

    assert_eq!(
        material,
        Color::rgb(231, 224, 236),
        "the same MultiTheme value, built as Cupertino, reads Material while the \
         global says Material"
    );
    assert_eq!(
        cupertino,
        Color::rgb(242, 242, 247),
        "and reads Cupertino the moment the global says Cupertino -- so it is not \
         frozen at construction either"
    );
}

/// Component tokens move the same way, which is the half a counter cares about.
#[test]
fn component_tokens_follow_the_process_global_too() {
    let gap_under = |ds: DesignSystem| {
        design_system_signal().set(ds);
        match MultiTheme::new_for(ds).component(ComponentId::Button) {
            ComponentTokens::Button(b) => b.height,
            _ => unreachable!(),
        }
    };
    assert_eq!(gap_under(DesignSystem::Material), 40.0);
    assert_eq!(gap_under(DesignSystem::Cupertino), 44.0);
    assert_eq!(gap_under(DesignSystem::Fluent), 32.0);
}
