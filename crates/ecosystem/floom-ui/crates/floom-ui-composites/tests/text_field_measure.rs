
use floom_app::{NodeId, WidgetHost};
use floom_layout::LayoutStyle;
use floom_types::Size;
use floom_ui_composites::{TextField, TextFieldStyle};

fn node_bounds(host: &WidgetHost, node: NodeId) -> floom_types::Rect {
    host.bounds(node)
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

/// The half the move added: a pinned height and a larger font are honoured
/// rather than measured at the engine's 14. The old `layout_request` read
/// `LABEL_FONT_SIZE` and `TEXTFIELD_MIN_WIDTH` directly, so a kit's 56pt field
/// would have been laid out at the engine's row height and clipped.
#[test]
fn a_styled_field_is_measured_from_its_style() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(600.0, 300.0));
    let root = host.mount_root(
        LayoutStyle {
            align_items: floom_layout::AlignItems::Start,
            ..LayoutStyle::default()
        },
        floom_widgets::Container::new(),
    );
    let long_value = "a reasonably long piece of typed text";
    let plain = host.mount(
        root,
        LayoutStyle::default(),
        TextField::new().value(long_value),
    );
    let kit_style = TextFieldStyle {
        height: Some(56.0),
        font_size: 16.0,
        padding_horizontal: 12.0,
        min_width: 200.0,
        ..TextFieldStyle::default()
    };
    let kit = host.mount(
        root,
        LayoutStyle::default(),
        TextField::new().value(long_value).style(kit_style),
    );
    host.frame();

    let plain_bounds = node_bounds(&host, plain);
    let kit_bounds = node_bounds(&host, kit);
    assert!(
        (kit_bounds.height() - 56.0).abs() < 1.5,
        "a pinned height is the height the field gets, got {kit_bounds:?}"
    );
    let measured = floom_text::measure_text(long_value, 16.0, f64::INFINITY);
    assert!(
        (kit_bounds.width() - (measured.width + 24.0)).abs() < 1.5,
        "and the width is the text at 16px plus 2x the style's own inset, got \
         {kit_bounds:?} (text {})",
        measured.width
    );
    assert!(
        kit_bounds.width() > plain_bounds.width(),
        "the style's numbers, not the engine's"
    );

    // And an empty styled field is floored at the *style's* minimum, which is
    // the field `min_width` exists for. Its own host: the box above is already
    // full of fields, and what is under test is the floor rather than how three
    // siblings share a row.
    let mut lone = WidgetHost::new();
    lone.set_size(Size::new(600.0, 300.0));
    let lone_root = lone.mount_root(
        LayoutStyle {
            align_items: floom_layout::AlignItems::Start,
            ..LayoutStyle::default()
        },
        floom_widgets::Container::new(),
    );
    let empty = lone.mount(
        lone_root,
        LayoutStyle::default(),
        TextField::new().style(TextFieldStyle {
            min_width: 320.0,
            ..TextFieldStyle::default()
        }),
    );
    lone.frame();
    assert!(
        (node_bounds(&lone, empty).width() - 320.0).abs() < 1.5,
        "an empty styled field is floored at the style's own minimum, got {:?}",
        node_bounds(&lone, empty)
    );
}
