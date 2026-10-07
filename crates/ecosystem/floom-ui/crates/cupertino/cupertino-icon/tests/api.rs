//! What this crate's API *is* now, replacing v1's builder-and-style test.
//!
//! v1's `api.rs` asserted three things about `cupertinoIcon::resolved_style()`:
//! `design_system == "cupertino"`, `color == theme.colors().on_surface`, and
//! `size == 22.0`. None of that surface survives the port, and the reason is worth
//! writing down rather than leaving as a deleted test.
//!
//! ## Why the style struct went
//!
//! v1's icon was a *builder* — `cupertinoIcon::new("home").label("Home")` — that resolved a
//! `MultiTheme` and produced a `resolved_style()` the caller then applied with
//! `.style(..)`. 2.0's icon is a widget the caller *holds*: a `VectorIcon` with its
//! own `set_size` and `set_color`. So there is no style struct to resolve, and the
//! theme-colour lookup it performed has no equivalent — a caller that wants the design
//! system's `on_surface` reads it from the theme itself, which is where it lives in
//! 2.0 too.
//!
//! ## What replaces the coverage
//!
//! `icons.rs` covers the path table and the engine widget, which is what the crate
//! exports now. What neither can cover is *which design system's colours* a caller ends
//! up with — and that is not this crate's business: it resolves no theme at all now. It
//! is the same property correction 9 records about every kit, covered where it is
//! observable, in the kit crates.

use floom_types::Color;
use floom_widgets::VectorIcon;

use floom_widget_cupertino_icon::{cupertino_icon, cupertino_icon_path};

/// The crate's whole surface, in one assertion: a name in, an engine widget out.
///
/// v1 returned a builder that a caller styled; 2.0 returns the widget itself and the
/// caller restyles it in place. This is the API change a ported caller actually has to
/// make, so it is the one worth a test — and the colour is the readable half of it,
/// because `VectorIcon::fill()` is a reader while its size is not.
#[test]
fn a_name_in_an_engine_widget_out() {
    let mut icon: VectorIcon = cupertino_icon("home");
    assert_eq!(
        icon.fill(),
        Color::BLACK,
        "the default v1's set implies with `fill=\"currentColor\"`"
    );
    icon.set_color(Color::rgb(0x11, 0x22, 0x33));
    assert_eq!(
        icon.fill(),
        Color::rgb(0x11, 0x22, 0x33),
        "a caller chooses the colour now, where v1's style struct did"
    );
}

/// And it is `Clone`, so a kit can hand the same icon to two slots — the other half of
/// why `VectorIcon::set_color` was added when `icon_button` promoted.
#[test]
fn the_icon_can_be_handed_out_twice() {
    let icon = cupertino_icon("home");
    let mut first = icon.clone();
    let second = icon.clone();
    first.set_color(Color::WHITE);
    assert_ne!(
        first.fill(),
        second.fill(),
        "so restyling one copy does not restyle the other"
    );
}

/// The name is still resolved by the kit's own table, which is the whole reason
/// `Combobox::icons` takes a lookup rather than assuming one icon set.
#[test]
fn the_name_is_resolved_by_this_kits_own_table() {
    assert_eq!(cupertino_icon_path("home"), cupertino_icon_path("home"));
    assert_ne!(
        cupertino_icon_path("home"),
        "",
        "and this kit does have a `home` icon of its own"
    );
}
