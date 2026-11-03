
use floom_app::{NodeId, WidgetHost};
use floom_layout::LayoutStyle;
use floom_types::Size;
use floom_ui_composites::Slider;

fn node_bounds(host: &WidgetHost, node: NodeId) -> floom_types::Rect {
    host.bounds(node)
}

/// A slider's intrinsic size is the geometry it paints: `thumb_size` tall (the
/// thumb is the tallest thing on it) and a default track length wide. Note the
/// height is [`SliderStyle::thumb_size`], so this is the *default* thumb -- a
/// kit that widens the thumb pins its own height rather than being measured
/// wrong.
#[test]
fn a_slider_reports_its_painted_geometry() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(400.0, 200.0));
    let root = host.mount_root(
        LayoutStyle {
            align_items: floom_layout::AlignItems::Start,
            ..LayoutStyle::default()
        },
        floom_widgets::Container::new(),
    );
    let slider = host.mount(root, LayoutStyle::default(), Slider::new(0.5, 0.0, 1.0));
    host.frame();

    let bounds = node_bounds(&host, slider);
    assert!(
        (bounds.height() - 16.0).abs() < 1.5,
        "slider height = thumb diameter, got {bounds:?}"
    );
    assert!(
        bounds.width() > 100.0,
        "slider has a usable track, got {bounds:?}"
    );
}
