//! Every composite in this crate has to be mountable.
//!
//! A `Widget` is a description of a node; a `WidgetAny` is that description
//! *type-erased*, and it is `WidgetAny` -- not `Widget` -- that the engine
//! stores and that a caller boxes a component into. A composite that
//! implements `Widget` but not `WidgetAny` can be built, returned from a
//! public method, and then not mounted: `Box::new(Counter::new(1))` fails
//! with "not a Floom 2.0 `Widget`".
//!
//! `Ripple` had exactly that gap, and it was found the first time somebody
//! composed one. These tests hold the gap closed for the composites that had
//! it, so a new composite cannot quietly reopen it.

use floom_core::WidgetAny;
use floom_widgets::VectorIcon;

use floom_ui_composites::{Badge, Chip, Counter, IconButton};

/// A counter, boxed the way a caller boxes a component.
fn boxed_counter() -> Box<dyn WidgetAny> {
    Box::new(Counter::new(1))
}

/// An icon button, boxed the way a caller boxes a component.
fn boxed_icon_button() -> Box<dyn WidgetAny> {
    Box::new(IconButton::new(VectorIcon::from_path_data(
        "M0 0 L10 10",
        floom_types::Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 10.0,
            y1: 10.0,
        },
    )))
}

/// A badge, boxed the way a caller boxes a component.
fn boxed_badge() -> Box<dyn WidgetAny> {
    Box::new(Badge::new().content("7"))
}

/// A chip, boxed the way a caller boxes a component.
fn boxed_chip() -> Box<dyn WidgetAny> {
    Box::new(Chip::new().label("Rust"))
}

#[test]
fn a_counter_can_be_boxed_as_a_widget() {
    let widget = boxed_counter();
    assert!(
        widget.as_any().downcast_ref::<Counter>().is_some(),
        "the boxed counter reads back as a counter"
    );
}

#[test]
fn an_icon_button_can_be_boxed_as_a_widget() {
    let widget = boxed_icon_button();
    assert!(
        widget.as_any().downcast_ref::<IconButton>().is_some(),
        "the boxed icon button reads back as an icon button"
    );
}

#[test]
fn a_badge_can_be_boxed_as_a_widget() {
    let widget = boxed_badge();
    assert!(
        widget.as_any().downcast_ref::<Badge>().is_some(),
        "the boxed badge reads back as a badge"
    );
}

#[test]
fn a_chip_can_be_boxed_as_a_widget() {
    let widget = boxed_chip();
    assert!(
        widget.as_any().downcast_ref::<Chip>().is_some(),
        "the boxed chip reads back as a chip"
    );
}
