//! Composite controls size themselves from their label.
//!
//! `Button`/`Checkbox`/`Radio`/`Switch` report an intrinsic extent through
//! `layout_request` (measured label + padding/indicator geometry). The host
//! applies that as *default* sizing: unpinned controls shrink-wrap their
//! content, an explicit mount style still wins.

use floem_app::{NodeId, WidgetHost};
use floem_layout::LayoutStyle;
use floem_types::Size;
use floem_widgets::controls::{Button, Checkbox, Switch};

fn node_bounds(host: &WidgetHost, node: NodeId) -> floem_types::Rect {
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
            align_items: floem_layout::AlignItems::Start,
            ..LayoutStyle::default()
        },
        floem_widgets::Container::new(),
    );
    let label = "Click me";
    let button = host.mount(root, LayoutStyle::default(), Button::new(label));
    host.frame();

    let bounds = node_bounds(&host, button);
    let measured = floem_text::measure_text(label, 14.0, f64::INFINITY);
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
            align_items: floem_layout::AlignItems::Start,
            ..LayoutStyle::default()
        },
        floem_widgets::Container::new(),
    );
    let button = host.mount(
        root,
        LayoutStyle {
            width: Some(200.0),
            height: Some(40.0),
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
            align_items: floem_layout::AlignItems::Start,
            ..LayoutStyle::default()
        },
        floem_widgets::Container::new(),
    );
    let label = "Ship it";
    let check = host.mount(root, LayoutStyle::default(), Checkbox::new(label));
    let short = host.mount(root, LayoutStyle::default(), Switch::new(label));
    host.frame();

    let bounds = node_bounds(&host, check);
    let measured = floem_text::measure_text(label, 14.0, f64::INFINITY);
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
