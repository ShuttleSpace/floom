//! `Select` sizes its closed control from its style.
//!
//! This lived in `floom-app/tests/composite_measure.rs` next to the other
//! control-sizing tests, and moved with the widget for the reason the rest of
//! this crate's tests did: a test of a widget's behaviour belongs with the
//! widget, and `floom-app` is a layer below it.

use floom_app::WidgetHost;
use floom_core::Size;
use floom_layout::{AlignItems, LayoutStyle};
use floom_ui_composites::{Select, SelectStyle};

fn node_bounds(host: &WidgetHost, node: floom_app::NodeId) -> floom_core::Rect {
    host.bounds(node)
}

/// A select sizes its closed control from the widest label (option or
/// selection) at the style's font size, plus the style's padding on each side,
/// and one [`SelectStyle::height`] tall.
///
/// The two numbers used to be written here as `16.0` and `22.0` — a copy of the
/// constants the widget had at the time — which is why the test went red the
/// moment the `Select` started reading a `SelectStyle`: it was asserting the
/// copy, not the widget. It now reads the style, which is the thing the
/// assertion is actually about.
#[test]
fn select_sizes_from_the_widest_label() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(400.0, 200.0));
    let root = host.mount_root(
        LayoutStyle {
            align_items: AlignItems::Start,
            ..LayoutStyle::default()
        },
        floom_widgets::Container::new(),
    );
    let select = host.mount(
        root,
        LayoutStyle::default(),
        Select::new(["Alpha", "Beta Gamma Delta Epsilon"]).selected(0),
    );
    host.frame();

    let style = SelectStyle::default();
    let bounds = node_bounds(&host, select);
    let widest =
        floom_text::measure_text("Beta Gamma Delta Epsilon", style.text_size, f64::INFINITY);
    let expected = (widest.width + f64::from(style.padding_horizontal) * 2.0)
        .max(f64::from(style.min_width));
    assert!(
        (bounds.width() - expected).abs() < 1.5,
        "select width = widest option + 2x{} padding (floored at the style's min_width), \
         got {bounds:?}, expected {expected}",
        style.padding_horizontal
    );
    assert!(
        (bounds.height() - f64::from(style.height)).abs() < 1.5,
        "select height = the style's height, got {bounds:?}"
    );
}
