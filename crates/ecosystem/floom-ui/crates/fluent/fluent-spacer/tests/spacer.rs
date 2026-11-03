//! Fluent's spacer: the grow factor, and the widget it reaches.

use floom_core::Widget;
use floom_types::Length;
use floom_widgets::Spacer;

use floom_widget_fluent_spacer::FluentSpacer;

fn spacer() -> FluentSpacer {
    FluentSpacer::new()
}

#[test]
fn the_grow_factor_reaches_the_engine_widget() {
    let spacer = spacer();
    assert_eq!(spacer.resolved_style().grow, 1.0, "v1's default");

    let widget = spacer.view();
    let request = widget
        .layout_request()
        .expect("a spacer asks for its own space");
    assert_eq!(
        request.flex_grow,
        Some(1.0),
        "and the engine widget says the same thing"
    );
    assert_eq!(widget.grow_factor(), Some(1.0));
}

#[test]
fn a_fractional_factor_survives() {
    let widget = spacer().grow(0.5).view();
    let request = widget
        .layout_request()
        .expect("a spacer asks for its own space");
    assert_eq!(request.flex_grow, Some(0.5));
    assert_eq!(widget.grow_factor(), Some(0.5));
    assert!(
        widget.is_flexible(),
        "a spacer that grows is a flexible one, whatever the factor"
    );
}

#[test]
fn the_props_report_the_factor() {
    assert_eq!(spacer().grow(2.0).props().grow, 2.0);
}

#[test]
fn a_fixed_extent_spacer_is_not_a_growing_one() {
    let fixed = Spacer::new(20.0);
    assert!(!fixed.is_flexible());
    assert_eq!(fixed.grow_factor(), None);
    let request = fixed
        .layout_request()
        .expect("a spacer asks for its own space");
    assert_eq!(request.flex_grow, None);
}
