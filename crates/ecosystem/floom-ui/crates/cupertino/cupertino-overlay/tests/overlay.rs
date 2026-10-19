//! Cupertino's overlay as a kit crate: the token translation, plus the wiring that
//! is this kit's own rather than the engine's.
//!
//! This crate had never been compiled, which is how three near-identical scrim
//! kits came to exist without anyone noticing the engine already mounts a modal
//! `Dialog`. So the point of this file is to pin the palette *this kit wrote*
//! and the show/flag wiring, in the places where a plausible-looking edit would
//! change a pixel or a click.
//!
//! ## What v1 drew that this port cannot
//!
//! **The overlay-over-a-child positioning is gone** -- v1 wrapped arbitrary
//! content in a scrim and mounted it as a layer. The engine `Dialog` mounts the
//! card as a host modal over the viewport, and the scrim is the card (the engine
//! paints no scrim of its own). That is a faithful one-to-one: v1's scrim-plus-
//! content *is* the card. The only behaviour that does not transfer verbatim is
//! **v1's click-anywhere-outside-content dismiss**: 2.0's modal is not
//! light-dismissed, so the scrim is a `Pressable` carrying the dismiss. If 2.0's
//! event model bubbles a content click up to that `Pressable`, the content click
//! would also dismiss -- v1 `Stop`-ed propagation on the content, which 2.0's
//! `EventPropagation` does not expose here. Recorded, not worked around.

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

use floom_widgets::text_input::{read_flag, write_flag};

use floom_widget_cupertino_overlay::{ CupertinoOverlay, CupertinoOverlayStyle };

fn theme() -> MultiTheme {
    MultiTheme::new_for(DesignSystem::Cupertino)
}

fn style_of(o: &CupertinoOverlay) -> CupertinoOverlayStyle {
    o.resolved_style()
}

#[test]
fn the_scrim_colour_is_the_scrim_token() {
    let o = CupertinoOverlay::new().with_theme(theme());
    let style = style_of(&o);
    assert_eq!(style.scrim_color, theme().colors().scrim);
}

#[test]
fn the_content_padding_comes_from_overlay_tokens() {
    let o = CupertinoOverlay::new().with_theme(theme());
    let style = style_of(&o);
    let tokens = match theme().component(ComponentId::Overlay) {
        ComponentTokens::Overlay(t) => t,
        _ => unreachable!(),
    };
    assert_eq!(style.content_padding, tokens.content_padding);
}

#[test]
fn show_writes_the_shared_flag() {
    let mut o = CupertinoOverlay::new().with_theme(theme());
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
    let o = CupertinoOverlay::new().with_theme(theme()).with_show_signal(external.clone());
    assert!(read_flag(&o.open_flag()));
    write_flag(&external, false);
    // The kit copied the flag's value at construction; the held flag is the same
    // `Arc`, so reading it back sees the change.
    assert!(!read_flag(&o.open_flag()));
}

#[test]
fn on_click_is_recorded_and_smoke_view_builds() {
    let o = CupertinoOverlay::new()
        .with_theme(theme())
        .on_click(|| {})
        .child(|| Box::new(floom_widgets::Text::new("hi".to_string())));
    let _d = o.view();
}

#[test]
fn a_childless_overlay_builds() {
    let o = CupertinoOverlay::new().with_theme(theme());
    let _d = o.view();
}
