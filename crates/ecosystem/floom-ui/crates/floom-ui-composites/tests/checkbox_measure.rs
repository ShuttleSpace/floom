//! A checkbox's intrinsic extent, measured through a real host.
//!
//! This is the half of `floom-app/tests/composite_measure.rs` that belongs to the
//! widget rather than to the crate. Nothing named `Checkbox` was left asserted
//! there after the move -- the file's switch row covers "indicator + gap +
//! label, height fits both" for the control that stayed -- but the measurement
//! still has to be pinned somewhere, and it has to be pinned *here*, because
//! `layout_request` now reads the style and a kit's 20px box is laid out from the
//! style's own numbers rather than from the engine's.
//!
//! The 20px case is the reason this file exists rather than the default's 16.
//! All three v1 kits read `CheckboxTokens::size`, and Material's is 20, so a
//! layout that measured the label at the engine's 14 and reserved the engine's
//! 16 would clip a fifth of the control. That is the same defect the `Slider`
//! and `Radio` ports fixed, and the assertions below are what hold the fix in
//! place.

use floom_app::{NodeId, WidgetHost};
use floom_core::Widget;
use floom_layout::LayoutStyle;
use floom_types::Size;
use floom_ui_composites::{Checkbox, CheckboxStyle};

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

/// The default checkbox's intrinsic size is its own geometry: the indicator's
/// side plus the gap plus the measured label, and a height that fits the taller
/// of the two. Note the label is measured at
/// `CheckboxStyle::label_font_size`, which defaults to the engine's 14 -- so
/// this is the *default* checkbox, and a kit's font size is the next test.
#[test]
fn a_checkbox_reports_its_indicator_gap_and_label() {
    let (mut host, root) = host_with_root();
    let label = "Ship it";
    let checkbox = host.mount(root, LayoutStyle::default(), Checkbox::new(label));
    host.frame();

    let bounds = node_bounds(&host, checkbox);
    let measured = floom_text::measure_text(label, 14.0, f64::INFINITY);
    assert!(
        (bounds.width() - (16.0 + 6.0 + measured.width)).abs() < 1.5,
        "checkbox width = box + gap + label, got {bounds:?} (label {})",
        measured.width
    );
    assert!(
        bounds.height() >= 16.0,
        "the indicator keeps its full 16px side, got {bounds:?}"
    );

    // A longer label means a wider checkbox.
    let wider = host.mount(root, LayoutStyle::default(), Checkbox::new(label.repeat(3)));
    host.frame();
    assert!(
        node_bounds(&host, wider).width() > bounds.width(),
        "longer label, wider checkbox"
    );
}

/// **The defect this file exists for.** A style whose indicator is larger than
/// the engine's 16 reserves *that* side, and a style whose label is set larger is
/// measured at *that* size. A `layout_request` reading constants would report
/// the engine's 16 and the engine's 14 for both, and every kit's checkbox would
/// be laid out smaller than it paints.
#[test]
fn a_styled_checkbox_is_measured_from_its_style() {
    let (mut host, root) = host_with_root();
    let label = "Ship it";
    let style = CheckboxStyle {
        size: 20.0,
        label_gap: 8.0,
        label_font_size: 20.0,
        ..CheckboxStyle::default()
    };
    let styled = host.mount(
        root,
        LayoutStyle::default(),
        Checkbox::new(label).style(style),
    );
    host.frame();

    let bounds = node_bounds(&host, styled);
    let measured = floom_text::measure_text(label, 20.0, f64::INFINITY);
    assert!(
        (bounds.width() - (20.0 + 8.0 + measured.width)).abs() < 1.5,
        "the kit's own side, gap and font size, got {bounds:?} (label {})",
        measured.width
    );
    assert!(
        bounds.height() >= 20.0,
        "the 20px indicator is reserved, got {bounds:?}"
    );

    // And it is genuinely larger than the default rather than coincidentally
    // equal, so the assertion above cannot pass by accident.
    let plain = host.mount(root, LayoutStyle::default(), Checkbox::new(label));
    host.frame();
    let plain_bounds = node_bounds(&host, plain);
    assert!(
        bounds.height() > plain_bounds.height() && bounds.width() > plain_bounds.width(),
        "a kit's checkbox reserves more than the engine's: {bounds:?} vs {plain_bounds:?}"
    );
}

/// An unlabelled checkbox measures to nothing, so an auto-sized control holding
/// one is not given a phantom extent. Carried across from the engine's
/// `layout_request`, which is the one half of the move that is not new.
#[test]
fn an_unlabelled_checkbox_asks_for_no_extent() {
    let checkbox = Checkbox::new("");
    assert!(
        checkbox.layout_request().is_none(),
        "an empty label measures to zero rather than reserving an indicator"
    );
}
