//! Composite controls size themselves from their label.
//!
//! `Button`/`Checkbox`/`Radio`/`Switch` report an intrinsic extent through
//! `layout_request` (measured label + padding/indicator geometry). The host
//! applies that as *default* sizing: unpinned controls shrink-wrap their
//! content, an explicit mount style still wins.

use floom_app::{NodeId, WidgetHost};
use floom_layout::{LayoutStyle, Length};
use floom_types::Size;
use floom_widgets::controls::{Button, Checkbox, Progress, Switch, TextField};
use floom_widgets::primitives::{Divider, Icon, Image, Spacer};

fn node_bounds(host: &WidgetHost, node: NodeId) -> floom_types::Rect {
    host.bounds(node)
}

/// An unpinned button shrink-wraps its label plus the paint padding
/// (8px each side horizontally, 4px vertically).
#[test]
fn button_shrink_wraps_its_label() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(400.0, 200.0));
    let root = host.mount_root(
        LayoutStyle {
            align_items: floom_layout::AlignItems::Start,
            ..LayoutStyle::default()
        },
        floom_widgets::Container::new(),
    );
    let label = "Click me";
    let button = host.mount(root, LayoutStyle::default(), Button::new(label));
    host.frame();

    let bounds = node_bounds(&host, button);
    let measured = floom_text::measure_text(label, 14.0, f64::INFINITY);
    assert!(
        (bounds.width() - (measured.width + 16.0)).abs() < 1.5,
        "button width = label + 2x8 padding, got {bounds:?} (label {})",
        measured.width
    );
    assert!(
        (bounds.height() - (measured.height + 8.0)).abs() < 1.5,
        "button height = label + 2x4 padding, got {bounds:?}"
    );

    // A longer label means a wider button.
    let wider = host.mount(root, LayoutStyle::default(), Button::new(label.repeat(3)));
    host.frame();
    assert!(
        node_bounds(&host, wider).width() > bounds.width(),
        "longer label, wider button"
    );
}

/// An explicit mount style always beats the intrinsic size.
#[test]
fn pinned_style_beats_the_button_intrinsic_size() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(400.0, 200.0));
    let root = host.mount_root(
        LayoutStyle {
            align_items: floom_layout::AlignItems::Start,
            ..LayoutStyle::default()
        },
        floom_widgets::Container::new(),
    );
    let button = host.mount(
        root,
        LayoutStyle {
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(40.0)),
            ..LayoutStyle::default()
        },
        Button::new("tiny"),
    );
    host.frame();

    let bounds = node_bounds(&host, button);
    assert!(
        (bounds.width() - 200.0).abs() < 0.5,
        "pinned width, got {bounds:?}"
    );
    assert!(
        (bounds.height() - 40.0).abs() < 0.5,
        "pinned height, got {bounds:?}"
    );
}

/// A checkbox's intrinsic width covers the 16px indicator, the 6px gap and
/// the label; the height fits both indicator and label.
#[test]
fn checkbox_includes_indicator_and_label_extent() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(400.0, 200.0));
    let root = host.mount_root(
        LayoutStyle {
            align_items: floom_layout::AlignItems::Start,
            ..LayoutStyle::default()
        },
        floom_widgets::Container::new(),
    );
    let label = "Ship it";
    let check = host.mount(root, LayoutStyle::default(), Checkbox::new(label));
    let short = host.mount(root, LayoutStyle::default(), Switch::new(label));
    host.frame();

    let bounds = node_bounds(&host, check);
    let measured = floom_text::measure_text(label, 14.0, f64::INFINITY);
    assert!(
        (bounds.width() - (16.0 + 6.0 + measured.width)).abs() < 1.5,
        "checkbox width = box + gap + label, got {bounds:?}"
    );
    assert!(bounds.height() >= 16.0, "indicator keeps its 16px side");

    // The switch's track (40x20) makes it taller than the checkbox's 16px box.
    let switch_bounds = node_bounds(&host, short);
    assert!(
        (switch_bounds.height() - 20.0).abs() < 1.5,
        "switch height = track height, got {switch_bounds:?}"
    );
}

/// A text field's intrinsic width covers the shown text plus padding, never
/// below the 120px minimum so an empty input stays usable; the height rests
/// on the line height even with no value and no placeholder.
#[test]
fn textfield_keeps_a_minimum_width_and_measures_its_text() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(400.0, 200.0));
    let root = host.mount_root(
        LayoutStyle {
            align_items: floom_layout::AlignItems::Start,
            ..LayoutStyle::default()
        },
        floom_widgets::Container::new(),
    );
    // Empty field: floored at the minimum width.
    let empty = host.mount(root, LayoutStyle::default(), TextField::new());
    // A long value grows past the minimum.
    let long_value = "a reasonably long piece of typed text";
    let filled = host.mount(
        root,
        LayoutStyle::default(),
        TextField::new().value(long_value),
    );
    host.frame();

    let empty_bounds = node_bounds(&host, empty);
    assert!(
        (empty_bounds.width() - 120.0).abs() < 1.5,
        "empty field floored at 120px, got {empty_bounds:?}"
    );
    assert!(
        empty_bounds.height() >= 20.0,
        "empty field still has a text row's height, got {empty_bounds:?}"
    );

    let filled_bounds = node_bounds(&host, filled);
    let measured = floom_text::measure_text(long_value, 14.0, f64::INFINITY);
    assert!(
        (filled_bounds.width() - (measured.width + 16.0)).abs() < 1.5,
        "filled field = text + 2x8 padding, got {filled_bounds:?} (text {})",
        measured.width
    );
    assert!(
        filled_bounds.width() > empty_bounds.width(),
        "typed text widens the field past the minimum"
    );
}

/// A progress bar has no text to measure either: its intrinsic extent is a
/// convention rather than a measurement -- an 8px bar of default length. The
/// slider that shared this test moved to `floom-ui-composites`, which is where
/// its own measurement lives now.
#[test]
fn progress_reports_its_painted_geometry() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(400.0, 200.0));
    let root = host.mount_root(
        LayoutStyle {
            align_items: floom_layout::AlignItems::Start,
            ..LayoutStyle::default()
        },
        floom_widgets::Container::new(),
    );
    let progress = host.mount(root, LayoutStyle::default(), Progress::determinate(0.5));
    host.frame();

    let progress_bounds = node_bounds(&host, progress);
    assert!(
        (progress_bounds.height() - 8.0).abs() < 1.5,
        "progress height = bar height, got {progress_bounds:?}"
    );
    assert!(progress_bounds.width() > 100.0, "progress has a length");
}

/// An icon box is square and sized from its glyph; an image reports the
/// intrinsic size its producer declared; a divider declares its thickness on
/// its own axis only; a spacer reports its extent along its axis.
#[test]
fn icon_image_divider_and_spacer_report_their_extents() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(400.0, 200.0));
    let root = host.mount_root(
        LayoutStyle {
            align_items: floom_layout::AlignItems::Start,
            ..LayoutStyle::default()
        },
        floom_widgets::Container::new(),
    );
    let icon = host.mount(root, LayoutStyle::default(), Icon::new('★').size(24.0));
    let image = host.mount(
        root,
        LayoutStyle::default(),
        Image::new(floom_types::ResourceId::new(1, 1))
            .intrinsic_size(floom_types::Size::new(32.0, 18.0)),
    );
    let divider = host.mount(root, LayoutStyle::default(), Divider::new());
    let spacer = host.mount(
        root,
        LayoutStyle::default(),
        Spacer::new(12.0).axis(floom_widgets::Axis::Horizontal),
    );
    host.frame();

    let icon_bounds = node_bounds(&host, icon);
    assert!(
        icon_bounds.width() > 8.0 && icon_bounds.height() > 8.0,
        "icon measured its glyph, got {icon_bounds:?}"
    );
    assert!(
        (icon_bounds.width() - icon_bounds.height()).abs() < 1.5,
        "icon box is square, got {icon_bounds:?}"
    );

    let image_bounds = node_bounds(&host, image);
    assert!(
        (image_bounds.width() - 32.0).abs() < 1.5 && (image_bounds.height() - 18.0).abs() < 1.5,
        "image reports its declared intrinsic size, got {image_bounds:?}"
    );

    let divider_bounds = node_bounds(&host, divider);
    assert!(
        divider_bounds.height() > 0.0 && divider_bounds.height() <= 2.0,
        "horizontal divider is as thick as its rule, got {divider_bounds:?}"
    );

    let spacer_bounds = node_bounds(&host, spacer);
    assert!(
        (spacer_bounds.width() - 12.0).abs() < 1.5,
        "spacer reports its extent on its axis, got {spacer_bounds:?}"
    );
}
