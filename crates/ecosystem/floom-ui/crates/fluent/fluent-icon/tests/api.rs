//!

use floom_types::Color;
use floom_widgets::VectorIcon;

use floom_widget_fluent_icon::{fluent_icon, fluent_icon_path};

#[test]
fn a_name_in_an_engine_widget_out() {
    let mut icon: VectorIcon = fluent_icon("home");
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

#[test]
fn the_icon_can_be_handed_out_twice() {
    let icon = fluent_icon("home");
    let mut first = icon.clone();
    let second = icon.clone();
    first.set_color(Color::WHITE);
    assert_ne!(
        first.fill(),
        second.fill(),
        "so restyling one copy does not restyle the other"
    );
}

#[test]
fn the_name_is_resolved_by_this_kits_own_table() {
    assert_eq!(fluent_icon_path("home"), fluent_icon_path("home"));
    assert_ne!(
        fluent_icon_path("home"),
        "",
        "and this kit does have a `home` icon of its own"
    );
}
