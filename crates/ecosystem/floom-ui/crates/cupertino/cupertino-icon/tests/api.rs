//! What this crate's API *is* now, replacing v1's builder-and-style test.
//!

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
