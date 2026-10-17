//! A radio's intrinsic extent, measured through a real host.
//!
//! This is the half of `floom-app/tests/composite_measure.rs` that belongs to
//! the widget rather than to the crate. Nothing named `Radio` was asserted
//! there -- the file's checkbox row covers "indicator + gap + label" for the
//! controls that stayed in the widgets crate -- but the measurement still has to
//! be pinned somewhere, and it has to be pinned *here*, because `layout_request`
//! now reads the style and a kit's 28px radio is laid out from the style's own
//! numbers rather than from the engine's.
//!
//! The 28px case is the reason this file exists rather than the default's 16.
//! Material's `RadioTokens` puts a 20px ring inside a 40px touch target, so a
//! layout that measured the label at the engine's 14 and reserved the engine's
//! 16 would clip a third of the control. That is the same defect the `Slider`
//! port fixed, and the assertions below are what hold the fix in place.

use floom_app::{NodeId, WidgetHost};
use floom_layout::LayoutStyle;
use floom_types::Size;
use floom_ui_composites::{Radio, RadioStyle};

fn node_bounds(host: &WidgetHost, node: NodeId) -> floom_types::Rect {
    host.bounds(node)
}

fn host_with_root() -> (WidgetHost, NodeId) {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(400.0, 200.0));
    let root = host.mount_root(
        LayoutStyle {
            align_items: floom_layout::AlignItems::Start,
            ..LayoutStyle::default()
        },
        floom_widgets::Container::new(),
    );
    (host, root)
}

/// The default radio's intrinsic size is its own geometry: the ring's diameter
/// plus the gap plus the measured label, and a height that fits the taller of the
/// two. Note the label is measured at `RadioStyle::label_font_size`, which
/// defaults to the engine's 14 -- so this is the *default* radio, and a kit's
/// font size is the next test.
#[test]
fn a_radio_reports_its_ring_gap_and_label() {
    let (mut host, root) = host_with_root();
    let label = "Ship it";
    let radio = host.mount(root, LayoutStyle::default(), Radio::new(label));
    host.frame();

    let bounds = node_bounds(&host, radio);
    let measured = floom_text::measure_text(label, 14.0, f64::INFINITY);
    assert!(
        (bounds.width() - (16.0 + 6.0 + measured.width)).abs() < 1.5,
        "radio width = ring + gap + label, got {bounds:?} (label {})",
        measured.width
    );
    assert!(
        bounds.height() >= 16.0,
        "the ring keeps its full diameter, got {bounds:?}"
    );

    // A longer label means a wider radio.
    let wider = host.mount(root, LayoutStyle::default(), Radio::new(label.repeat(3)));
    host.frame();
    assert!(
        node_bounds(&host, wider).width() > bounds.width(),
        "longer label, wider radio"
    );
}

/// **The defect this file exists for.** A style whose touch target is larger than
/// its ring reserves the touch target, and a style whose label is set larger is
/// measured at *that* size. A `layout_request` reading constants would report
/// the engine's 16 and the engine's 14 for both, and every kit's radio would be
/// laid out smaller than it paints.
#[test]
fn a_styled_radio_is_measured_from_its_style() {
    let (mut host, root) = host_with_root();
    let label = "Ship it";
    let style = RadioStyle {
        size: 20.0,
        touch_target: 40.0,
        label_font_size: 20.0,
        ..RadioStyle::default()
    };
    let styled = host.mount(root, LayoutStyle::default(), Radio::new(label).style(style));
    host.frame();

    let bounds = node_bounds(&host, styled);
    let measured = floom_text::measure_text(label, 20.0, f64::INFINITY);
    assert!(
        (bounds.width() - (40.0 + 6.0 + measured.width)).abs() < 1.5,
        "the touch target and the kit's own font size, got {bounds:?} (label {})",
        measured.width
    );
    assert!(
        bounds.height() >= 40.0,
        "the 40px touch target is reserved, got {bounds:?}"
    );

    // And it is genuinely larger than the default rather than coincidentally
    // equal, so the assertion above cannot pass by accident.
    let plain = host.mount(root, LayoutStyle::default(), Radio::new(label));
    host.frame();
    let plain_bounds = node_bounds(&host, plain);
    assert!(
        bounds.height() > plain_bounds.height() && bounds.width() > plain_bounds.width(),
        "a kit's radio reserves more than the engine's: {bounds:?} vs {plain_bounds:?}"
    );
}

/// A touch target *below* the ring must not shrink the reservation: the ring is
/// the thing that gets clipped, so it wins.
#[test]
fn the_ring_wins_when_the_touch_target_is_smaller() {
    let (mut host, root) = host_with_root();
    let style = RadioStyle {
        size: 24.0,
        touch_target: 8.0,
        ..RadioStyle::default()
    };
    let radio = host.mount(
        root,
        LayoutStyle::default(),
        Radio::new("Ship it").style(style),
    );
    host.frame();

    let bounds = node_bounds(&host, radio);
    assert!(
        bounds.height() >= 24.0,
        "the ring, not the smaller hit area, sets the height; got {bounds:?}"
    );
}
