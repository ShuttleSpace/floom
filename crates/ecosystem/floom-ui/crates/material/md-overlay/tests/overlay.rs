
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

use floom_widgets::text_input::{read_flag, write_flag};

use floom_widget_md_overlay::{MdOverlay, MdOverlayStyle};

fn theme() -> MultiTheme {
    MultiTheme::new_for(DesignSystem::Material)
}

fn style_of(o: &MdOverlay) -> MdOverlayStyle {
    o.resolved_style()
}

#[test]
fn the_scrim_colour_is_the_scrim_token() {
    let o = MdOverlay::new().with_theme(theme());
    let style = style_of(&o);
    assert_eq!(style.scrim_color, theme().colors().scrim);
}

#[test]
fn the_content_padding_comes_from_overlay_tokens() {
    let o = MdOverlay::new().with_theme(theme());
    let style = style_of(&o);
    let tokens = match theme().component(ComponentId::Overlay) {
        ComponentTokens::Overlay(t) => t,
        _ => unreachable!(),
    };
    assert_eq!(style.content_padding, tokens.content_padding);
}

#[test]
fn show_writes_the_shared_flag() {
    let mut o = MdOverlay::new().with_theme(theme());
    let flag = o.open_flag();
    assert!(!read_flag(&flag));
    o = o.show(true);
    assert!(read_flag(&flag));
    o = o.show(false);
    assert!(!read_flag(&flag));
}

#[test]
fn with_show_signal_reads_back_the_external_flag() {
    let external = floom_widgets::text_input::shared_flag(true);
    let o = MdOverlay::new()
        .with_theme(theme())
        .with_show_signal(external.clone());
    assert!(read_flag(&o.open_flag()));
    write_flag(&external, false);
    assert!(!read_flag(&o.open_flag()));
}

#[test]
fn on_click_is_recorded_and_smoke_view_builds() {
    let o = MdOverlay::new()
        .with_theme(theme())
        .on_click(|| {})
        .child(|| Box::new(floom_widgets::Text::new("hi".to_string())));
    let _d = o.view();
}

#[test]
fn a_childless_overlay_builds() {
    let o = MdOverlay::new().with_theme(theme());
    let _d = o.view();
}
