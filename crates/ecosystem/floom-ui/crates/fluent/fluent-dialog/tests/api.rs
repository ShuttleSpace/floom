//!

use floom_core_theme::theme::Theme;

use floom_widget_fluent_dialog::FluentDialog;

fn dialog() -> FluentDialog {
    FluentDialog::new().title("Confirm").content("Continue?")
}

#[test]
fn the_builder_chain_is_neutral() {
    let dialog = dialog().open(true);
    let props = dialog.props();
    assert_eq!(props.title, "Confirm");
    assert_eq!(props.content, "Continue?");
    assert!(
        props.open,
        "and `.open(true)` writes the flag `props()` reports"
    );
}

#[test]
fn the_style_resolves_from_a_theme() {
    let theme = floom_core_theme::theme::MultiTheme::new_for(
        floom_core_theme::global_theme::DesignSystem::Fluent,
    );
    let style = dialog().with_theme(theme.clone()).resolved_style();

    assert_eq!(
        style.surface,
        floom_types::Color {
            r: theme.colors().surface.r,
            g: theme.colors().surface.g,
            b: theme.colors().surface.b,
            a: theme.colors().surface.a,
        },
        "`container_color` renamed to `surface`"
    );
    assert_eq!(
        style.content_alpha, 222,
        "and `content_color`'s alpha is now a field of its own, so the body can be \
         said to be lighter than the title rather than merely differently coloured"
    );
    let floom_core_theme::component_tokens::ComponentTokens::Dialog(tokens) =
        theme.component(floom_core_theme::component_tokens::ComponentId::Dialog)
    else {
        panic!("expected DialogTokens");
    };
    assert_eq!(style.max_width, tokens.max_width);
    assert_ne!(
        style.max_width, 420.0,
        "which is not the literal v1's own test asserted — that one had gone stale \
         against the token it was nominally checking"
    );
}
