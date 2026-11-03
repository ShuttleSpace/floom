//! Composite controls size themselves from their label.
//!
//! `Button`/`Switch` report an intrinsic extent through
//! `layout_request` (measured label + padding/indicator geometry). The host
//! applies that as *default* sizing: unpinned controls shrink-wrap their
//! content, an explicit mount style still wins.
//!
//! `TextField` used to be named here too, and its case moved out with the
//! widget: its measurement is pinned in
//! `floom-ui-composites/tests/text_field_measure.rs`, which asserts the same
//! 120px floor and 2x8 inset against the engine's numbers *and* against a
//! style's own -- the second half is new, and is why the move needed a file
//! rather than a deletion.
//!
//! `Radio` used to be named here too. It has no test in this file: the control
//! it would have shared a body with (`Slider`) moved out with its own
//! measurement. The radio's measurement lives in
//! `floom-ui-composites/tests/radio_measure.rs`, where the widget does.
//!
//! `Checkbox` was the row those notes kept pointing at -- it is the control
//! "indicator + gap + label, height fits both" was actually measured on -- and
//! it has gone the same way. Its measurement is now in
//! `floom-ui-composites/tests/checkbox_measure.rs`, where its own style is
//! readable. What is left of that row below is the switch, which is still a
//! primitive.

use floom_app::{NodeId, WidgetHost};
use floom_layout::{LayoutStyle, Length};
use floom_types::Size;
use floom_widgets::controls::{Button, Progress, Switch};
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

/// A switch's intrinsic width covers the 40px track, the 6px gap and the label;
/// the height fits the track rather than the label.
///
/// This row measured a checkbox beside it, and that is where the "indicator +
/// gap + label" claim was made for both. The checkbox half moved with the widget
/// to `floom-ui-composites/tests/checkbox_measure.rs`; the assertions below are
/// the ones that stayed.
#[test]
fn switch_includes_indicator_and_label_extent() {
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
    let short = host.mount(root, LayoutStyle::default(), Switch::new(label));
    host.frame();

    let bounds = node_bounds(&host, short);
    let measured = floom_text::measure_text(label, 14.0, f64::INFINITY);
    assert!(
        (bounds.width() - (40.0 + 6.0 + measured.width)).abs() < 1.5,
        "switch width = track + gap + label, got {bounds:?} (label {})",
        measured.width
    );
    // The 40x20 track is taller than the label's 14px line, so the height follows
    // the track rather than the text.
    assert!(
        (bounds.height() - 20.0).abs() < 1.5,
        "switch height = track height, got {bounds:?}"
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
